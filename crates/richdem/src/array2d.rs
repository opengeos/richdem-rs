//! 2D georeferenced raster array matching RichDEM's `Array2D<T>`.

use std::collections::BTreeMap;
use std::fmt::Write as _;
use std::fs::File;
use std::io::{BufRead, BufReader, BufWriter, Write};
use std::path::Path;

use crate::error::{Error, Result};
use crate::num::DemNum;

/// Default identity geotransform: top-left `(0, 0)`, 1x1 north-up cells.
pub const STANDARD_GEOTRANSFORM: [f64; 6] = [0.0, 1.0, 0.0, 0.0, 0.0, -1.0];

/// A 2D row-major raster array with geospatial metadata (geotransform, projection, NoData, tags).
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Array2D<T: DemNum> {
    width: usize,
    height: usize,
    data: Vec<T>,
    no_data: T,
    /// GDAL-style 6-element affine geotransform `[origin_x, pixel_w, rot_x, origin_y, rot_y, pixel_h]`.
    pub geotransform: [f64; 6],
    /// CRS projection string (WKT, EPSG code, or PROJ string).
    pub projection: String,
    /// Raster key-value metadata tags (including `PROCESSING_HISTORY`).
    pub metadata: BTreeMap<String, String>,
    /// Source filename if loaded from disk.
    pub filename: String,
    /// Source basename if applicable.
    pub basename: String,
}

impl<T: DemNum> PartialEq for Array2D<T> {
    fn eq(&self, other: &Self) -> bool {
        if self.width != other.width || self.height != other.height {
            return false;
        }
        self.data
            .iter()
            .zip(other.data.iter())
            .all(|(a, b)| a.rd_eq(*b))
    }
}

impl<T: DemNum> Default for Array2D<T> {
    fn default() -> Self {
        Self {
            width: 0,
            height: 0,
            data: Vec::new(),
            no_data: T::ZERO,
            geotransform: STANDARD_GEOTRANSFORM,
            projection: String::new(),
            metadata: BTreeMap::new(),
            filename: String::new(),
            basename: String::new(),
        }
    }
}

impl<T: DemNum> Array2D<T> {
    /// Creates a `width x height` raster filled with `fill`.
    pub fn new(width: usize, height: usize, fill: T) -> Self {
        Self {
            width,
            height,
            data: vec![fill; width * height],
            no_data: T::ZERO,
            geotransform: STANDARD_GEOTRANSFORM,
            projection: String::new(),
            metadata: BTreeMap::new(),
            filename: String::new(),
            basename: String::new(),
        }
    }

    /// Creates a raster from an existing row-major buffer of length `width * height`.
    pub fn from_vec(width: usize, height: usize, data: Vec<T>) -> Result<Self> {
        if data.len() != width * height {
            return Err(Error::DimensionMismatch(format!(
                "expected {} elements ({}x{}), got {}",
                width * height,
                width,
                height,
                data.len()
            )));
        }
        Ok(Self {
            width,
            height,
            data,
            no_data: T::ZERO,
            geotransform: STANDARD_GEOTRANSFORM,
            projection: String::new(),
            metadata: BTreeMap::new(),
            filename: String::new(),
            basename: String::new(),
        })
    }

    /// Creates a raster from a slice of equal-length rows (convenient for tests).
    pub fn from_rows(rows: &[&[T]]) -> Result<Self> {
        let height = rows.len();
        let width = rows.first().map_or(0, |r| r.len());
        let mut data = Vec::with_capacity(width * height);
        for (y, row) in rows.iter().enumerate() {
            if row.len() != width {
                return Err(Error::DimensionMismatch(format!(
                    "row {y} has length {}, expected {width}",
                    row.len()
                )));
            }
            data.extend_from_slice(row);
        }
        Self::from_vec(width, height, data)
    }

    /// Creates a new raster with the same dimensions and geospatial metadata as `template`,
    /// filled with `fill`.
    pub fn make_from_template<U: DemNum>(template: &Array2D<U>, fill: T) -> Self {
        let mut out = Self::new(template.width(), template.height(), fill);
        out.copy_metadata_from(template);
        out
    }

    /// Copies geospatial metadata (geotransform, projection, metadata tags) from `other`.
    pub fn copy_metadata_from<U: DemNum>(&mut self, other: &Array2D<U>) {
        self.geotransform = other.geotransform;
        self.projection = other.projection.clone();
        self.metadata = other.metadata.clone();
        self.filename = other.filename.clone();
        self.basename = other.basename.clone();
    }

    /// Resizes the raster to `width x height`, filling with `fill`.
    pub fn resize(&mut self, width: usize, height: usize, fill: T) {
        self.width = width;
        self.height = height;
        self.data.clear();
        self.data.resize(width * height, fill);
    }

    /// Resizes the raster to match `other`'s dimensions and copies its metadata.
    pub fn resize_like<U: DemNum>(&mut self, other: &Array2D<U>, fill: T) {
        self.resize(other.width(), other.height(), fill);
        self.copy_metadata_from(other);
    }

    /// Clears the raster to `0 x 0`.
    pub fn clear(&mut self) {
        self.width = 0;
        self.height = 0;
        self.data.clear();
    }

    /// Expands the raster to `new_width x new_height`, placing existing cells at `(0, 0)`
    /// and filling newly added cells with `fill`.
    pub fn expand(&mut self, new_width: usize, new_height: usize, fill: T) -> Result<()> {
        if new_width < self.width || new_height < self.height {
            return Err(Error::DimensionMismatch(format!(
                "cannot expand {}x{} to smaller {}x{}",
                self.width, self.height, new_width, new_height
            )));
        }
        let mut new_data = vec![fill; new_width * new_height];
        for y in 0..self.height {
            let src_start = y * self.width;
            let dst_start = y * new_width;
            new_data[dst_start..dst_start + self.width]
                .copy_from_slice(&self.data[src_start..src_start + self.width]);
        }
        self.width = new_width;
        self.height = new_height;
        self.data = new_data;
        Ok(())
    }

    /// Width in cells (number of columns).
    #[inline]
    pub fn width(&self) -> usize {
        self.width
    }

    /// Height in cells (number of rows).
    #[inline]
    pub fn height(&self) -> usize {
        self.height
    }

    /// Total number of cells (`width * height`).
    #[inline]
    pub fn size(&self) -> usize {
        self.data.len()
    }

    /// Returns `true` if the raster has zero cells.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.data.is_empty()
    }

    /// Immutable slice of underlying row-major data.
    #[inline]
    pub fn data(&self) -> &[T] {
        &self.data
    }

    /// Mutable slice of underlying row-major data.
    #[inline]
    pub fn data_mut(&mut self) -> &mut [T] {
        &mut self.data
    }

    /// Consumes the raster and returns its row-major pixel vector.
    #[inline]
    pub fn into_vec(self) -> Vec<T> {
        self.data
    }

    /// Current NoData sentinel value.
    #[inline]
    pub fn no_data(&self) -> T {
        self.no_data
    }

    /// Sets the NoData sentinel value.
    #[inline]
    pub fn set_no_data(&mut self, val: T) {
        self.no_data = val;
    }

    /// Converts `(x, y)` to a flat row-major index `y * width + x`.
    #[inline]
    pub fn xy_to_i(&self, x: usize, y: usize) -> usize {
        y * self.width + x
    }

    /// Converts a flat index `i` to `(x, y)`.
    #[inline]
    pub fn i_to_xy(&self, i: usize) -> (usize, usize) {
        (i % self.width, i / self.width)
    }

    /// Checks whether signed coordinates `(x, y)` lie inside `[0, width) x [0, height)`.
    #[inline]
    pub fn in_grid(&self, x: i32, y: i32) -> bool {
        x >= 0 && y >= 0 && (x as usize) < self.width && (y as usize) < self.height
    }

    /// Computes neighbor flat index from flat index `i` and signed offset `(dx, dy)`.
    #[inline]
    pub fn n_to_i(&self, i: usize, dx: i32, dy: i32) -> Option<usize> {
        let x = (i % self.width) as i32 + dx;
        let y = (i / self.width) as i32 + dy;
        if self.in_grid(x, y) {
            Some((y as usize) * self.width + (x as usize))
        } else {
            None
        }
    }

    /// Gets the value at `(x, y)`.
    #[inline]
    pub fn get(&self, x: usize, y: usize) -> T {
        self.data[y * self.width + x]
    }

    /// Sets the value at `(x, y)`.
    #[inline]
    pub fn set(&mut self, x: usize, y: usize, val: T) {
        self.data[y * self.width + x] = val;
    }

    /// Gets the value at signed coordinates `(x, y)`.
    #[inline]
    pub fn get_xy(&self, x: i32, y: i32) -> T {
        debug_assert!(self.in_grid(x, y));
        self.data[(y as usize) * self.width + (x as usize)]
    }

    /// Sets the value at signed coordinates `(x, y)`.
    #[inline]
    pub fn set_xy(&mut self, x: i32, y: i32, val: T) {
        debug_assert!(self.in_grid(x, y));
        self.data[(y as usize) * self.width + (x as usize)] = val;
    }

    /// Gets the value at flat index `i`.
    #[inline]
    pub fn get_i(&self, i: usize) -> T {
        self.data[i]
    }

    /// Sets the value at flat index `i`.
    #[inline]
    pub fn set_i(&mut self, i: usize, val: T) {
        self.data[i] = val;
    }

    /// Mutable reference to flat index `i`.
    #[inline]
    pub fn get_i_mut(&mut self, i: usize) -> &mut T {
        &mut self.data[i]
    }

    /// Mutable reference to `(x, y)`.
    #[inline]
    pub fn get_mut(&mut self, x: usize, y: usize) -> &mut T {
        &mut self.data[y * self.width + x]
    }

    /// Returns `true` if cell `(x, y)` equals `no_data`.
    #[inline]
    pub fn is_no_data(&self, x: usize, y: usize) -> bool {
        self.get(x, y).rd_eq(self.no_data)
    }

    /// Returns `true` if signed cell `(x, y)` equals `no_data`.
    #[inline]
    pub fn is_no_data_xy(&self, x: i32, y: i32) -> bool {
        self.get_xy(x, y).rd_eq(self.no_data)
    }

    /// Returns `true` if flat index `i` equals `no_data`.
    #[inline]
    pub fn is_no_data_i(&self, i: usize) -> bool {
        self.data[i].rd_eq(self.no_data)
    }

    /// Returns `true` if cell `(x, y)` is valid data (not NoData).
    #[inline]
    pub fn is_data(&self, x: usize, y: usize) -> bool {
        !self.is_no_data(x, y)
    }

    /// Returns `true` if flat index `i` is valid data (not NoData).
    #[inline]
    pub fn is_data_i(&self, i: usize) -> bool {
        !self.is_no_data_i(i)
    }

    /// Counts the number of non-NoData cells.
    pub fn num_data_cells(&self) -> usize {
        self.data.iter().filter(|v| !v.rd_eq(self.no_data)).count()
    }

    /// Returns `true` if `(x, y)` is on the top row (`y == 0`).
    #[inline]
    pub fn is_top_row(&self, y: usize) -> bool {
        y == 0
    }

    /// Returns `true` if `(x, y)` is on the bottom row (`y == height - 1`).
    #[inline]
    pub fn is_bottom_row(&self, y: usize) -> bool {
        self.height > 0 && y == self.height - 1
    }

    /// Returns `true` if `(x, y)` is on the left column (`x == 0`).
    #[inline]
    pub fn is_left_col(&self, x: usize) -> bool {
        x == 0
    }

    /// Returns `true` if `(x, y)` is on the right column (`x == width - 1`).
    #[inline]
    pub fn is_right_col(&self, x: usize) -> bool {
        self.width > 0 && x == self.width - 1
    }

    /// Returns `true` if `(x, y)` is on any perimeter edge of the grid.
    #[inline]
    pub fn is_edge_cell(&self, x: usize, y: usize) -> bool {
        self.is_top_row(y) || self.is_bottom_row(y) || self.is_left_col(x) || self.is_right_col(x)
    }

    /// Returns `true` if `(x, y)` is top-left corner `(0, 0)`.
    #[inline]
    pub fn is_top_left(&self, x: usize, y: usize) -> bool {
        x == 0 && y == 0
    }

    /// Returns `true` if `(x, y)` is top-right corner.
    #[inline]
    pub fn is_top_right(&self, x: usize, y: usize) -> bool {
        self.width > 0 && x == self.width - 1 && y == 0
    }

    /// Returns `true` if `(x, y)` is bottom-left corner.
    #[inline]
    pub fn is_bottom_left(&self, x: usize, y: usize) -> bool {
        self.height > 0 && x == 0 && y == self.height - 1
    }

    /// Returns `true` if `(x, y)` is bottom-right corner.
    #[inline]
    pub fn is_bottom_right(&self, x: usize, y: usize) -> bool {
        self.width > 0 && self.height > 0 && x == self.width - 1 && y == self.height - 1
    }

    /// Sets every cell in the raster to `val`.
    pub fn set_all(&mut self, val: T) {
        self.data.fill(val);
    }

    /// Sets all perimeter edge cells to `val`.
    pub fn set_edges(&mut self, val: T) {
        if self.width == 0 || self.height == 0 {
            return;
        }
        for x in 0..self.width {
            self.set(x, 0, val);
            self.set(x, self.height - 1, val);
        }
        for y in 0..self.height {
            self.set(0, y, val);
            self.set(self.width - 1, y, val);
        }
    }

    /// Minimum valid (non-NoData) value in the raster.
    pub fn min(&self) -> Option<T> {
        let mut best: Option<T> = None;
        for &v in &self.data {
            if v.rd_eq(self.no_data) || v.is_nan() {
                continue;
            }
            best = Some(match best {
                Some(b) => b.rd_min(v),
                None => v,
            });
        }
        best
    }

    /// Maximum valid (non-NoData) value in the raster.
    pub fn max(&self) -> Option<T> {
        let mut best: Option<T> = None;
        for &v in &self.data {
            if v.rd_eq(self.no_data) || v.is_nan() {
                continue;
            }
            best = Some(match best {
                Some(b) => b.rd_max(v),
                None => v,
            });
        }
        best
    }

    /// Counts occurrences of `val` in the raster.
    pub fn count_val(&self, val: T) -> usize {
        self.data.iter().filter(|v| v.rd_eq(val)).count()
    }

    /// X cell resolution (`|geotransform[1]|`).
    #[inline]
    pub fn get_cell_length_x(&self) -> f64 {
        let v = self.geotransform[1].abs();
        if v == 0.0 { 1.0 } else { v }
    }

    /// Y cell resolution (`|geotransform[5]|`).
    #[inline]
    pub fn get_cell_length_y(&self) -> f64 {
        let v = self.geotransform[5].abs();
        if v == 0.0 { 1.0 } else { v }
    }

    /// Cell area (`get_cell_length_x() * get_cell_length_y()`).
    #[inline]
    pub fn get_cell_area(&self) -> f64 {
        self.get_cell_length_x() * self.get_cell_length_y()
    }

    /// Appends an entry to the `PROCESSING_HISTORY` metadata item.
    pub fn add_processing_history(&mut self, operation: &str) {
        let entry = self
            .metadata
            .entry("PROCESSING_HISTORY".to_string())
            .or_default();
        if !entry.is_empty() {
            entry.push('\n');
        }
        let _ = write!(
            entry,
            "richdem-rs {} | {}",
            env!("CARGO_PKG_VERSION"),
            operation
        );
    }

    /// Converts all pixel values to another `DemNum` type, preserving geospatial metadata.
    pub fn cast<U: DemNum>(&self) -> Array2D<U> {
        let data = self.data.iter().map(|&v| U::from_f64(v.to_f64())).collect();
        let mut out = Array2D {
            width: self.width,
            height: self.height,
            data,
            no_data: U::from_f64(self.no_data.to_f64()),
            geotransform: self.geotransform,
            projection: self.projection.clone(),
            metadata: self.metadata.clone(),
            filename: self.filename.clone(),
            basename: self.basename.clone(),
        };
        out.set_no_data(U::from_f64(self.no_data.to_f64()));
        out
    }

    /// Formats the raster as a human-readable ASCII grid table.
    pub fn format_grid(&self) -> String {
        let mut s = String::new();
        for y in 0..self.height {
            for x in 0..self.width {
                if x > 0 {
                    s.push(' ');
                }
                let _ = write!(s, "{:>6}", self.get(x, y));
            }
            s.push('\n');
        }
        s
    }

    /// Loads an ESRI ASCII Grid (`.asc`, `.dem`, `.d8`, `.out`) directly without external drivers.
    pub fn load_ascii_grid(path: impl AsRef<Path>) -> Result<Self> {
        let path = path.as_ref();
        let file = File::open(path)?;
        let reader = BufReader::new(file);
        let mut lines = reader.lines();

        let mut ncols: Option<usize> = None;
        let mut nrows: Option<usize> = None;
        let mut xllcorner: Option<f64> = None;
        let mut yllcorner: Option<f64> = None;
        let mut xllcenter: Option<f64> = None;
        let mut yllcenter: Option<f64> = None;
        let mut cellsize: Option<f64> = None;
        let mut dx: Option<f64> = None;
        let mut dy: Option<f64> = None;
        let mut nodata: Option<f64> = None;
        let mut data_tokens: Vec<String> = Vec::new();

        for line_res in &mut lines {
            let line = line_res?;
            let trimmed = line.trim();
            if trimmed.is_empty() {
                continue;
            }
            let mut parts = trimmed.split_whitespace();
            let Some(first) = parts.next() else {
                continue;
            };
            let key = first.to_ascii_lowercase();
            match key.as_str() {
                "ncols" => {
                    let v = parts
                        .next()
                        .ok_or_else(|| Error::Parse("missing ncols value".into()))?;
                    ncols = Some(
                        v.parse::<usize>()
                            .map_err(|e| Error::Parse(format!("invalid ncols: {e}")))?,
                    );
                }
                "nrows" => {
                    let v = parts
                        .next()
                        .ok_or_else(|| Error::Parse("missing nrows value".into()))?;
                    nrows = Some(
                        v.parse::<usize>()
                            .map_err(|e| Error::Parse(format!("invalid nrows: {e}")))?,
                    );
                }
                "xllcorner" => {
                    let v = parts
                        .next()
                        .ok_or_else(|| Error::Parse("missing xllcorner value".into()))?;
                    xllcorner = Some(
                        v.parse::<f64>()
                            .map_err(|e| Error::Parse(format!("invalid xllcorner: {e}")))?,
                    );
                }
                "yllcorner" => {
                    let v = parts
                        .next()
                        .ok_or_else(|| Error::Parse("missing yllcorner value".into()))?;
                    yllcorner = Some(
                        v.parse::<f64>()
                            .map_err(|e| Error::Parse(format!("invalid yllcorner: {e}")))?,
                    );
                }
                "xllcenter" => {
                    let v = parts
                        .next()
                        .ok_or_else(|| Error::Parse("missing xllcenter value".into()))?;
                    xllcenter = Some(
                        v.parse::<f64>()
                            .map_err(|e| Error::Parse(format!("invalid xllcenter: {e}")))?,
                    );
                }
                "yllcenter" => {
                    let v = parts
                        .next()
                        .ok_or_else(|| Error::Parse("missing yllcenter value".into()))?;
                    yllcenter = Some(
                        v.parse::<f64>()
                            .map_err(|e| Error::Parse(format!("invalid yllcenter: {e}")))?,
                    );
                }
                "cellsize" => {
                    let v = parts
                        .next()
                        .ok_or_else(|| Error::Parse("missing cellsize value".into()))?;
                    cellsize = Some(
                        v.parse::<f64>()
                            .map_err(|e| Error::Parse(format!("invalid cellsize: {e}")))?,
                    );
                }
                "dx" => {
                    let v = parts
                        .next()
                        .ok_or_else(|| Error::Parse("missing dx value".into()))?;
                    dx = Some(
                        v.parse::<f64>()
                            .map_err(|e| Error::Parse(format!("invalid dx: {e}")))?,
                    );
                }
                "dy" => {
                    let v = parts
                        .next()
                        .ok_or_else(|| Error::Parse("missing dy value".into()))?;
                    dy = Some(
                        v.parse::<f64>()
                            .map_err(|e| Error::Parse(format!("invalid dy: {e}")))?,
                    );
                }
                "nodata_value" => {
                    let v = parts
                        .next()
                        .ok_or_else(|| Error::Parse("missing NODATA_value".into()))?;
                    nodata = Some(
                        v.parse::<f64>()
                            .map_err(|e| Error::Parse(format!("invalid NODATA_value: {e}")))?,
                    );
                }
                _ => {
                    data_tokens.push(first.to_string());
                    data_tokens.extend(parts.map(str::to_string));
                    break;
                }
            }
        }

        let width = ncols.ok_or_else(|| Error::Parse("missing ncols header".into()))?;
        let height = nrows.ok_or_else(|| Error::Parse("missing nrows header".into()))?;
        let expected = width * height;

        for line_res in lines {
            let line = line_res?;
            data_tokens.extend(line.split_whitespace().map(str::to_string));
        }

        if data_tokens.len() < expected {
            return Err(Error::Parse(format!(
                "ASCII grid {}: expected {expected} values, got {}",
                path.display(),
                data_tokens.len()
            )));
        }

        let mut data = Vec::with_capacity(expected);
        for tok in data_tokens.into_iter().take(expected) {
            let val = match tok.parse::<T>() {
                Ok(v) => v,
                Err(_) => {
                    let f = tok
                        .parse::<f64>()
                        .map_err(|e| Error::Parse(format!("invalid cell value '{tok}': {e}")))?;
                    T::from_f64(f)
                }
            };
            data.push(val);
        }

        let cs_x = dx.or(cellsize).unwrap_or(1.0);
        let cs_y = dy.or(cellsize).unwrap_or(1.0);
        let origin_x = xllcorner
            .or_else(|| xllcenter.map(|c| c - cs_x * 0.5))
            .unwrap_or(0.0);
        let y_ll = yllcorner
            .or_else(|| yllcenter.map(|c| c - cs_y * 0.5))
            .unwrap_or(0.0);
        let origin_y = y_ll + cs_y * (height as f64);

        let mut arr = Self::from_vec(width, height, data)?;
        arr.geotransform = [origin_x, cs_x, 0.0, origin_y, 0.0, -cs_y];
        if let Some(nd) = nodata {
            arr.set_no_data(T::from_f64(nd));
        }
        arr.filename = path.to_string_lossy().into_owned();
        Ok(arr)
    }

    /// Saves the raster as an ESRI ASCII Grid (`.asc`).
    pub fn save_ascii_grid(&self, path: impl AsRef<Path>) -> Result<()> {
        let file = File::create(path)?;
        let mut w = BufWriter::new(file);
        let cs_x = self.get_cell_length_x();
        let cs_y = self.get_cell_length_y();
        let xll = self.geotransform[0];
        let yll = self.geotransform[3] - cs_y * (self.height as f64);
        writeln!(w, "ncols         {}", self.width)?;
        writeln!(w, "nrows         {}", self.height)?;
        writeln!(w, "xllcorner     {xll}")?;
        writeln!(w, "yllcorner     {yll}")?;
        if (cs_x - cs_y).abs() < 1e-12 {
            writeln!(w, "cellsize      {cs_x}")?;
        } else {
            writeln!(w, "dx            {cs_x}")?;
            writeln!(w, "dy            {cs_y}")?;
        }
        writeln!(w, "NODATA_value  {}", self.no_data)?;
        for y in 0..self.height {
            for x in 0..self.width {
                if x > 0 {
                    write!(w, " ")?;
                }
                write!(w, "{}", self.get(x, y))?;
            }
            writeln!(w)?;
        }
        w.flush()?;
        Ok(())
    }
}

/// Dynamically typed 2D raster for CLI utilities and runtime-dispatched raster I/O.
#[derive(Debug, Clone)]
pub enum DynamicArray2D {
    /// Unsigned 8-bit integer raster.
    U8(Array2D<u8>),
    /// Signed 8-bit integer raster.
    I8(Array2D<i8>),
    /// Unsigned 16-bit integer raster.
    U16(Array2D<u16>),
    /// Signed 16-bit integer raster.
    I16(Array2D<i16>),
    /// Unsigned 32-bit integer raster.
    U32(Array2D<u32>),
    /// Signed 32-bit integer raster.
    I32(Array2D<i32>),
    /// Unsigned 64-bit integer raster.
    U64(Array2D<u64>),
    /// Signed 64-bit integer raster.
    I64(Array2D<i64>),
    /// 32-bit floating-point raster.
    F32(Array2D<f32>),
    /// 64-bit floating-point raster.
    F64(Array2D<f64>),
}

impl DynamicArray2D {
    /// Pixel type name (e.g., `"float32"`, `"int32"`).
    pub fn dtype_name(&self) -> &'static str {
        match self {
            Self::U8(_) => "uint8",
            Self::I8(_) => "int8",
            Self::U16(_) => "uint16",
            Self::I16(_) => "int16",
            Self::U32(_) => "uint32",
            Self::I32(_) => "int32",
            Self::U64(_) => "uint64",
            Self::I64(_) => "int64",
            Self::F32(_) => "float32",
            Self::F64(_) => "float64",
        }
    }

    /// Width in cells.
    pub fn width(&self) -> usize {
        match self {
            Self::U8(a) => a.width(),
            Self::I8(a) => a.width(),
            Self::U16(a) => a.width(),
            Self::I16(a) => a.width(),
            Self::U32(a) => a.width(),
            Self::I32(a) => a.width(),
            Self::U64(a) => a.width(),
            Self::I64(a) => a.width(),
            Self::F32(a) => a.width(),
            Self::F64(a) => a.width(),
        }
    }

    /// Height in cells.
    pub fn height(&self) -> usize {
        match self {
            Self::U8(a) => a.height(),
            Self::I8(a) => a.height(),
            Self::U16(a) => a.height(),
            Self::I16(a) => a.height(),
            Self::U32(a) => a.height(),
            Self::I32(a) => a.height(),
            Self::U64(a) => a.height(),
            Self::I64(a) => a.height(),
            Self::F32(a) => a.height(),
            Self::F64(a) => a.height(),
        }
    }

    /// Geotransform array.
    pub fn geotransform(&self) -> [f64; 6] {
        match self {
            Self::U8(a) => a.geotransform,
            Self::I8(a) => a.geotransform,
            Self::U16(a) => a.geotransform,
            Self::I16(a) => a.geotransform,
            Self::U32(a) => a.geotransform,
            Self::I32(a) => a.geotransform,
            Self::U64(a) => a.geotransform,
            Self::I64(a) => a.geotransform,
            Self::F32(a) => a.geotransform,
            Self::F64(a) => a.geotransform,
        }
    }

    /// Projection string.
    pub fn projection(&self) -> &str {
        match self {
            Self::U8(a) => &a.projection,
            Self::I8(a) => &a.projection,
            Self::U16(a) => &a.projection,
            Self::I16(a) => &a.projection,
            Self::U32(a) => &a.projection,
            Self::I32(a) => &a.projection,
            Self::U64(a) => &a.projection,
            Self::I64(a) => &a.projection,
            Self::F32(a) => &a.projection,
            Self::F64(a) => &a.projection,
        }
    }

    /// Metadata map.
    pub fn metadata(&self) -> &BTreeMap<String, String> {
        match self {
            Self::U8(a) => &a.metadata,
            Self::I8(a) => &a.metadata,
            Self::U16(a) => &a.metadata,
            Self::I16(a) => &a.metadata,
            Self::U32(a) => &a.metadata,
            Self::I32(a) => &a.metadata,
            Self::U64(a) => &a.metadata,
            Self::I64(a) => &a.metadata,
            Self::F32(a) => &a.metadata,
            Self::F64(a) => &a.metadata,
        }
    }

    /// NoData value as `f64`.
    pub fn no_data_f64(&self) -> f64 {
        match self {
            Self::U8(a) => a.no_data().to_f64(),
            Self::I8(a) => a.no_data().to_f64(),
            Self::U16(a) => a.no_data().to_f64(),
            Self::I16(a) => a.no_data().to_f64(),
            Self::U32(a) => a.no_data().to_f64(),
            Self::I32(a) => a.no_data().to_f64(),
            Self::U64(a) => a.no_data().to_f64(),
            Self::I64(a) => a.no_data().to_f64(),
            Self::F32(a) => a.no_data().to_f64(),
            Self::F64(a) => a.no_data().to_f64(),
        }
    }

    /// Converts to `Array2D<f64>`.
    pub fn to_f64(&self) -> Array2D<f64> {
        match self {
            Self::U8(a) => a.cast(),
            Self::I8(a) => a.cast(),
            Self::U16(a) => a.cast(),
            Self::I16(a) => a.cast(),
            Self::U32(a) => a.cast(),
            Self::I32(a) => a.cast(),
            Self::U64(a) => a.cast(),
            Self::I64(a) => a.cast(),
            Self::F32(a) => a.cast(),
            Self::F64(a) => a.clone(),
        }
    }
}
