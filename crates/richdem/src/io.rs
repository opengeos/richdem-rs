//! Pure-Rust geospatial raster I/O powered by [`terrio`].
//!
//! Replaces RichDEM's C++ GDAL dependency (`LoadGDAL` / `SaveGDAL`) with
//! zero-C-dependency pure-Rust reading and writing of GeoTIFF, COG, AAIGrid,
//! ENVI, HFA, VRT, and other raster formats supported by `terrio`.

use std::path::Path;

use terrio::crs::SpatialRef;
use terrio::raster::RasterData;
use terrio::{DataType, Raster, RasterConfig, RasterDataset, RasterWriteOptions};

use crate::array2d::{Array2D, DynamicArray2D};
use crate::error::{Error, Result};
use crate::num::DemNum;

fn path_to_utf8(path: &Path) -> Result<&str> {
    path.to_str()
        .ok_or_else(|| Error::InvalidArgument(format!("non-UTF-8 path: {}", path.display())))
}

/// Opens a raster file with `terrio` (falling back to RichDEM's ESRI ASCII Grid parser
/// for custom extensions such as `.dem`, `.d8`, `.out`), returning a [`DynamicArray2D`].
pub fn load_dynamic(path: impl AsRef<Path>) -> Result<DynamicArray2D> {
    let path = path.as_ref();
    let path_str = path_to_utf8(path)?;
    match terrio::open_raster(path_str) {
        Ok(ds) => dataset_to_dynamic(ds, path_str),
        Err(terrio_err) => {
            // Fall back to direct ESRI ASCII Grid parsing for RichDEM test fixtures
            // with extensions like `.dem`, `.d8`, `.out`.
            match Array2D::<f64>::load_ascii_grid(path) {
                Ok(arr) => Ok(DynamicArray2D::F64(arr)),
                Err(ascii_err) => Err(Error::Parse(format!(
                    "failed to open '{}' via terrio ({terrio_err}) or ESRI ASCII Grid fallback ({ascii_err})",
                    path.display()
                ))),
            }
        }
    }
}

/// Loads a raster band (band 1) into a typed [`Array2D<T>`].
///
/// If `override_no_data` is `Some`, it overrides the dataset's NoData value.
pub fn load_raster<T: DemNum>(
    path: impl AsRef<Path>,
    override_no_data: Option<T>,
) -> Result<Array2D<T>> {
    let path = path.as_ref();
    let path_str = path_to_utf8(path)?;

    let mut arr = match terrio::open_raster(path_str) {
        Ok(ds) => {
            let dyn_arr = dataset_to_dynamic(ds, path_str)?;
            dynamic_cast::<T>(dyn_arr)
        }
        Err(terrio_err) => match Array2D::<T>::load_ascii_grid(path) {
            Ok(a) => a,
            Err(ascii_err) => {
                return Err(Error::Parse(format!(
                    "failed to open '{}' via terrio ({terrio_err}) or ESRI ASCII Grid fallback ({ascii_err})",
                    path.display()
                )));
            }
        },
    };

    if let Some(nd) = override_no_data {
        arr.set_no_data(nd);
    }
    Ok(arr)
}

/// Loads a raster with symmetric halo padding (`pad_x`, `pad_y`) filled with `T::ZERO`,
/// matching RichDEM's `LoadGDAL(filename, arr, padx, pady)`.
pub fn load_raster_padded<T: DemNum>(
    path: impl AsRef<Path>,
    pad_x: usize,
    pad_y: usize,
) -> Result<Array2D<T>> {
    let inner = load_raster::<T>(path, None)?;
    if pad_x == 0 && pad_y == 0 {
        return Ok(inner);
    }
    let out_w = pad_x
        .checked_mul(2)
        .and_then(|p| p.checked_add(inner.width()))
        .ok_or_else(|| {
            Error::DimensionMismatch(format!("padded width overflows usize (pad_x={pad_x})"))
        })?;
    let out_h = pad_y
        .checked_mul(2)
        .and_then(|p| p.checked_add(inner.height()))
        .ok_or_else(|| {
            Error::DimensionMismatch(format!("padded height overflows usize (pad_y={pad_y})"))
        })?;
    let total = out_w.checked_mul(out_h).ok_or_else(|| {
        Error::DimensionMismatch(format!(
            "padded dimensions overflow usize ({out_w}x{out_h})"
        ))
    })?;
    let mut data = Vec::<T>::new();
    data.try_reserve_exact(total).map_err(|e| {
        Error::DimensionMismatch(format!("failed to allocate {out_w}x{out_h} raster: {e}"))
    })?;
    data.resize(total, T::ZERO);
    let mut out = Array2D::from_vec(out_w, out_h, data)?;
    out.copy_metadata_from(&inner);
    let gt = inner.geotransform;
    out.geotransform[0] = gt[0] - (pad_x as f64) * gt[1] - (pad_y as f64) * gt[2];
    out.geotransform[3] = gt[3] - (pad_x as f64) * gt[4] - (pad_y as f64) * gt[5];
    out.set_no_data(inner.no_data());
    for y in 0..inner.height() {
        for x in 0..inner.width() {
            out.set(x + pad_x, y + pad_y, inner.get(x, y));
        }
    }
    Ok(out)
}

/// Saves a typed [`Array2D<T>`] to disk using `terrio` (GeoTIFF/COG/ASCII/etc. by extension).
///
/// Preserves geotransform, projection CRS, NoData value, and metadata (including
/// `PROCESSING_HISTORY`).
pub fn save_raster<T: DemNum>(
    arr: &Array2D<T>,
    path: impl AsRef<Path>,
    analysis: Option<&str>,
    compress: bool,
) -> Result<()> {
    let path = path.as_ref();
    let mut ds = array2d_to_dataset(arr)?;
    if let Some(op) = analysis {
        let line = format!("richdem-rs {} | {op}", env!("CARGO_PKG_VERSION"));
        if let Some((_, val)) = ds
            .raster
            .metadata
            .iter_mut()
            .find(|(k, _)| k == "PROCESSING_HISTORY")
        {
            if !val.is_empty() {
                val.push('\n');
            }
            val.push_str(&line);
        } else {
            ds.raster
                .metadata
                .push(("PROCESSING_HISTORY".to_string(), line));
        }
    }
    let mut opts = RasterWriteOptions::default();
    if compress {
        opts.compression = Some("DEFLATE".into());
    }
    let path_str = path_to_utf8(path)?;
    terrio::write_raster_dataset(&ds, path_str, &opts)?;
    Ok(())
}

/// Saves a [`DynamicArray2D`] preserving its exact native pixel type.
pub fn save_dynamic(
    arr: &DynamicArray2D,
    path: impl AsRef<Path>,
    analysis: Option<&str>,
    compress: bool,
) -> Result<()> {
    match arr {
        DynamicArray2D::U8(a) => save_raster(a, path, analysis, compress),
        DynamicArray2D::I8(a) => save_raster(a, path, analysis, compress),
        DynamicArray2D::U16(a) => save_raster(a, path, analysis, compress),
        DynamicArray2D::I16(a) => save_raster(a, path, analysis, compress),
        DynamicArray2D::U32(a) => save_raster(a, path, analysis, compress),
        DynamicArray2D::I32(a) => save_raster(a, path, analysis, compress),
        DynamicArray2D::U64(a) => save_raster(a, path, analysis, compress),
        DynamicArray2D::I64(a) => save_raster(a, path, analysis, compress),
        DynamicArray2D::F32(a) => save_raster(a, path, analysis, compress),
        DynamicArray2D::F64(a) => save_raster(a, path, analysis, compress),
    }
}

/// Converts a `terrio::RasterDataset` (first band) into a [`DynamicArray2D`].
pub fn dataset_to_dynamic(ds: RasterDataset, filename: &str) -> Result<DynamicArray2D> {
    let width = ds.raster.cols;
    let height = ds.raster.rows;
    let pixels = width.checked_mul(height).ok_or_else(|| {
        Error::DimensionMismatch(format!(
            "raster dimensions overflow usize ({width}x{height})"
        ))
    })?;
    let gt = ds.geo_transform();
    let crs = ds.crs();
    let proj = crs.to_wkt().or_else(|| crs.to_proj4()).unwrap_or_default();
    let nodata_opt = ds.nodata();
    let metadata = ds.raster.metadata.iter().cloned().collect();

    macro_rules! build {
        ($variant:ident, $vec:expr, $ty:ty) => {{
            let slice = if $vec.len() >= pixels {
                $vec[..pixels].to_vec()
            } else {
                return Err(Error::DimensionMismatch(format!(
                    "raster band 1 has {} pixels, expected {pixels}",
                    $vec.len()
                )));
            };
            let mut arr = Array2D::<$ty>::from_vec(width, height, slice)?;
            arr.geotransform = gt;
            arr.projection = proj;
            arr.metadata = metadata;
            arr.filename = filename.to_string();
            if let Some(nd) = nodata_opt {
                arr.set_no_data(<$ty as DemNum>::from_nodata_f64(nd));
            }
            Ok(DynamicArray2D::$variant(arr))
        }};
    }

    match &ds.raster.data {
        RasterData::U8(v) => build!(U8, v, u8),
        RasterData::I8(v) => build!(I8, v, i8),
        RasterData::U16(v) => build!(U16, v, u16),
        RasterData::I16(v) => build!(I16, v, i16),
        RasterData::U32(v) => build!(U32, v, u32),
        RasterData::I32(v) => build!(I32, v, i32),
        RasterData::U64(v) => build!(U64, v, u64),
        RasterData::I64(v) => build!(I64, v, i64),
        RasterData::F32(v) => build!(F32, v, f32),
        RasterData::F64(v) => build!(F64, v, f64),
    }
}

/// Converts a typed [`Array2D<T>`] into a single-band [`terrio::RasterDataset`].
///
/// Pixel buffers are preserved in their exact native type (including 64-bit integers).
/// Note that `terrio::RasterConfig::nodata` is currently stored as `f64`, so 64-bit
/// integer NoData sentinels above `2^53` are rounded to the nearest `f64`.
pub fn array2d_to_dataset<T: DemNum>(arr: &Array2D<T>) -> Result<RasterDataset> {
    let (data_type, raster_data) = match T::type_name() {
        "uint8" => (
            DataType::U8,
            RasterData::U8(arr.data().iter().map(|v| v.to_u64() as u8).collect()),
        ),
        "int8" => (
            DataType::I8,
            RasterData::I8(arr.data().iter().map(|v| v.to_i64() as i8).collect()),
        ),
        "uint16" => (
            DataType::U16,
            RasterData::U16(arr.data().iter().map(|v| v.to_u64() as u16).collect()),
        ),
        "int16" => (
            DataType::I16,
            RasterData::I16(arr.data().iter().map(|v| v.to_i64() as i16).collect()),
        ),
        "uint32" => (
            DataType::U32,
            RasterData::U32(arr.data().iter().map(|v| v.to_u64() as u32).collect()),
        ),
        "int32" => (
            DataType::I32,
            RasterData::I32(arr.data().iter().map(|v| v.to_i64() as i32).collect()),
        ),
        "uint64" => (
            DataType::U64,
            RasterData::U64(arr.data().iter().map(|v| v.to_u64()).collect()),
        ),
        "int64" => (
            DataType::I64,
            RasterData::I64(arr.data().iter().map(|v| v.to_i64()).collect()),
        ),
        "float32" => (
            DataType::F32,
            RasterData::F32(arr.data().iter().map(|v| v.to_f64() as f32).collect()),
        ),
        _ => (
            DataType::F64,
            RasterData::F64(arr.data().iter().map(|v| v.to_f64()).collect()),
        ),
    };

    let config = RasterConfig {
        cols: arr.width(),
        rows: arr.height(),
        bands: 1,
        data_type,
        nodata: arr.no_data().to_f64(),
        ..Default::default()
    };

    let mut raster = Raster::from_data_native(config, raster_data)
        .map_err(|e| Error::Algorithm(format!("failed to construct terrio Raster: {e}")))?;

    raster.metadata = arr
        .metadata
        .iter()
        .map(|(k, v)| (k.clone(), v.clone()))
        .collect();

    if !arr.projection.is_empty() {
        let sref = SpatialRef::parse(&arr.projection).map_err(|e| {
            Error::InvalidArgument(format!("invalid projection '{}': {e}", arr.projection))
        })?;
        raster.crs = sref.to_crs_info();
    }

    let mut ds = RasterDataset::new(raster, true);
    ds.set_geo_transform(arr.geotransform);
    Ok(ds)
}

fn dynamic_cast<T: DemNum>(dyn_arr: DynamicArray2D) -> Array2D<T> {
    match dyn_arr {
        DynamicArray2D::U8(a) => a.cast(),
        DynamicArray2D::I8(a) => a.cast(),
        DynamicArray2D::U16(a) => a.cast(),
        DynamicArray2D::I16(a) => a.cast(),
        DynamicArray2D::U32(a) => a.cast(),
        DynamicArray2D::I32(a) => a.cast(),
        DynamicArray2D::U64(a) => a.cast(),
        DynamicArray2D::I64(a) => a.cast(),
        DynamicArray2D::F32(a) => a.cast(),
        DynamicArray2D::F64(a) => a.cast(),
    }
}
