//! WebAssembly (`wasm-bindgen`) bindings for `richdem-rs`.
//!
//! Thin marshalling layer operating completely in memory on GeoTIFF / AAIGrid byte buffers
//! (`Uint8Array`) with zero filesystem or GDAL dependencies, for browsers and Node.js.
//! Core hydrologic algorithms live in `crates/richdem`.

use serde::Serialize;
use terrio::RasterWriteOptions;
use terrio::memory::{open_raster_bytes, write_raster_bytes};
use wasm_bindgen::prelude::*;

use richdem::{
    Array2D, DemNum, DhLabel, FlowAccumMethod, Flowdir, LindsayMode, NO_DEP, NO_FLOW, NO_VALUE,
    OCEAN, TerrainAttr, Topology, array2d_to_dataset, d8_flats_alter_dem, d8_flow_directions,
    dataset_to_dynamic, fill_depressions as core_fill_depressions,
    fill_depressions_epsilon as core_fill_depressions_epsilon, fill_spill_merge, garbrecht_alg,
    get_depression_hierarchy, get_flat_mask, get_new_depression_hierarchy_labels, has_depressions,
    lindsay2016, resolve_flats_epsilon, ta_aspect, ta_cti, ta_slope_radians, ta_slope_riserun,
    ta_spi, terrain_attribute as core_terrain_attribute, try_generate_perlin_terrain,
};

/// Upper bound on synthetic Perlin terrain dimension in synchronous browser execution.
const MAX_WASM_PERLIN_SIZE: usize = 2048;

#[wasm_bindgen(start)]
pub fn init_panic_hook() {
    console_error_panic_hook::set_once();
}

fn to_js_err(e: impl std::fmt::Display) -> JsValue {
    JsValue::from_str(&e.to_string())
}

fn parse_topology(topology: &str) -> Result<Topology, JsValue> {
    match topology.to_ascii_lowercase().as_str() {
        "" | "d8" => Ok(Topology::D8),
        "d4" => Ok(Topology::D4),
        other => Err(to_js_err(format!(
            "invalid topology '{other}': expected 'D8' or 'D4'"
        ))),
    }
}

fn read_array2d_f64(bytes: &[u8], filename: &str) -> Result<Array2D<f64>, JsValue> {
    let ds = open_raster_bytes(bytes.to_vec(), filename).map_err(to_js_err)?;
    let dyn_arr = dataset_to_dynamic(ds, filename).map_err(to_js_err)?;
    Ok(dyn_arr.to_f64())
}

fn write_array2d_tif<T: DemNum>(arr: &Array2D<T>, name: &str) -> Result<Vec<u8>, JsValue> {
    let ds = array2d_to_dataset(arr).map_err(to_js_err)?;
    write_raster_bytes(&ds, name, &RasterWriteOptions::default()).map_err(to_js_err)
}

fn raster_min_max(arr: &Array2D<f64>) -> (f64, f64) {
    let nd = arr.no_data();
    let mut vmin = f64::INFINITY;
    let mut vmax = f64::NEG_INFINITY;
    for &v in arr.data() {
        if v.is_finite() && !v.rd_eq(nd) {
            if v < vmin {
                vmin = v;
            }
            if v > vmax {
                vmax = v;
            }
        }
    }
    if vmin.is_finite() && vmax.is_finite() {
        (vmin, vmax)
    } else {
        (0.0, 0.0)
    }
}

fn viridis_rgb(t: f64) -> [u8; 3] {
    const STOPS: [(f64, [f64; 3]); 5] = [
        (0.00, [68.0, 1.0, 84.0]),
        (0.25, [59.0, 82.0, 139.0]),
        (0.50, [33.0, 145.0, 140.0]),
        (0.75, [94.0, 201.0, 98.0]),
        (1.00, [253.0, 231.0, 37.0]),
    ];
    let t = t.clamp(0.0, 1.0);
    for w in STOPS.windows(2) {
        let (t0, c0) = w[0];
        let (t1, c1) = w[1];
        if t <= t1 {
            let f = if t1 > t0 { (t - t0) / (t1 - t0) } else { 0.0 };
            return [
                (c0[0] + f * (c1[0] - c0[0])).round() as u8,
                (c0[1] + f * (c1[1] - c0[1])).round() as u8,
                (c0[2] + f * (c1[2] - c0[2])).round() as u8,
            ];
        }
    }
    [253, 231, 37]
}

fn terrain_rgb(t: f64) -> [u8; 3] {
    const STOPS: [(f64, [f64; 3]); 5] = [
        (0.00, [46.0, 114.0, 82.0]),
        (0.25, [112.0, 168.0, 94.0]),
        (0.50, [202.0, 198.0, 132.0]),
        (0.75, [170.0, 128.0, 92.0]),
        (1.00, [242.0, 240.0, 238.0]),
    ];
    let t = t.clamp(0.0, 1.0);
    for w in STOPS.windows(2) {
        let (t0, c0) = w[0];
        let (t1, c1) = w[1];
        if t <= t1 {
            let f = if t1 > t0 { (t - t0) / (t1 - t0) } else { 0.0 };
            return [
                (c0[0] + f * (c1[0] - c0[0])).round() as u8,
                (c0[1] + f * (c1[1] - c0[1])).round() as u8,
                (c0[2] + f * (c1[2] - c0[2])).round() as u8,
            ];
        }
    }
    [242, 240, 238]
}

fn blues_rgb(t: f64) -> [u8; 3] {
    let t = t.clamp(0.0, 1.0);
    [
        (220.0 * (1.0 - t) + 8.0 * t).round() as u8,
        (238.0 * (1.0 - t) + 52.0 * t).round() as u8,
        (252.0 * (1.0 - t) + 122.0 * t).round() as u8,
    ]
}

fn diverging_rgb(t: f64) -> [u8; 3] {
    let t = t.clamp(0.0, 1.0);
    if t < 0.5 {
        let f = t * 2.0;
        [
            (33.0 + f * (245.0 - 33.0)).round() as u8,
            (102.0 + f * (245.0 - 102.0)).round() as u8,
            (172.0 + f * (245.0 - 172.0)).round() as u8,
        ]
    } else {
        let f = (t - 0.5) * 2.0;
        [
            (245.0 + f * (178.0 - 245.0)).round() as u8,
            (245.0 + f * (24.0 - 245.0)).round() as u8,
            (245.0 + f * (43.0 - 245.0)).round() as u8,
        ]
    }
}

#[derive(Clone, Copy)]
enum Ramp {
    Viridis,
    Blues,
    Diverging,
}

fn colorize_f64_rgba(
    arr: &Array2D<f64>,
    ramp: Ramp,
    log_scale: bool,
    positive_only: bool,
) -> Vec<u8> {
    let nd = arr.no_data();
    let is_valid =
        |v: f64| -> bool { v.is_finite() && !v.rd_eq(nd) && (!positive_only || v > 0.0) };
    let transform = |v: f64| -> f64 {
        if log_scale {
            (v.max(0.0) + 1.0).ln()
        } else {
            v
        }
    };

    let data = arr.data();
    let mut vmin = f64::INFINITY;
    let mut vmax = f64::NEG_INFINITY;
    for &v in data {
        if is_valid(v) {
            let tv = transform(v);
            if tv < vmin {
                vmin = tv;
            }
            if tv > vmax {
                vmax = tv;
            }
        }
    }
    // Symmetric stretch around zero for diverging ramps (curvatures).
    if matches!(ramp, Ramp::Diverging) && vmin.is_finite() && vmax.is_finite() {
        let lim = vmin.abs().max(vmax.abs()).max(1e-12);
        vmin = -lim;
        vmax = lim;
    }
    let span = (vmax - vmin).max(1e-12);

    let mut out = vec![0u8; data.len() * 4];
    for (i, &v) in data.iter().enumerate() {
        if !is_valid(v) {
            continue;
        }
        let t = ((transform(v) - vmin) / span).clamp(0.0, 1.0);
        let c = match ramp {
            Ramp::Viridis => viridis_rgb(t),
            Ramp::Blues => blues_rgb(t),
            Ramp::Diverging => diverging_rgb(t),
        };
        let p = i * 4;
        out[p] = c[0];
        out[p + 1] = c[1];
        out[p + 2] = c[2];
        out[p + 3] = 255;
    }
    out
}

fn colorize_labels_rgba(labels: &Array2D<DhLabel>) -> Vec<u8> {
    let data = labels.data();
    let mut out = vec![0u8; data.len() * 4];
    for (i, &lbl) in data.iter().enumerate() {
        if lbl == OCEAN || lbl == NO_DEP || lbl == NO_VALUE {
            continue;
        }
        // Golden-ratio hue hash for distinct visual separation of adjacent basins.
        let h = ((lbl as f64) * 0.618_033_988_75).fract();
        let c = viridis_rgb(h);
        let p = i * 4;
        out[p] = c[0];
        out[p + 1] = c[1];
        out[p + 2] = c[2];
        out[p + 3] = 255;
    }
    out
}

fn f32_array_to_f64(arr: &Array2D<f32>) -> Array2D<f64> {
    let data: Vec<f64> = arr.data().iter().map(|&v| f64::from(v)).collect();
    let mut out = Array2D::from_vec(arr.width(), arr.height(), data)
        .expect("f32_array_to_f64 dimensions match");
    out.copy_metadata_from(arr);
    out.set_no_data(f64::from(arr.no_data()));
    out
}

/// Computes shaded hypsometric relief RGBA pixels using Horn slope & aspect from `richdem`.
fn compute_hillshade_rgba(
    dem: &Array2D<f64>,
    azimuth_deg: f64,
    elevation_deg: f64,
    z_factor: f64,
) -> Vec<u8> {
    let slope = ta_slope_radians(dem, z_factor as f32);
    let aspect = ta_aspect(dem, z_factor as f32);
    let (vmin, vmax) = raster_min_max(dem);
    let span = (vmax - vmin).max(1e-12);
    let zenith = (90.0 - elevation_deg.clamp(0.0, 90.0)).to_radians();
    let az_rad = azimuth_deg.to_radians();
    let cos_z = zenith.cos();
    let sin_z = zenith.sin();

    let nd = dem.no_data();
    let s_nd = slope.no_data();
    let a_nd = aspect.no_data();
    let n = dem.data().len();
    let mut out = vec![0u8; n * 4];

    for i in 0..n {
        let z = dem.data()[i];
        if !z.is_finite() || z.rd_eq(nd) {
            continue;
        }
        let s = slope.data()[i];
        let a = aspect.data()[i];
        let shade = if s.is_finite() && !s.rd_eq(s_nd) && a.is_finite() && !a.rd_eq(a_nd) {
            let s_f64 = f64::from(s);
            let a_f64 = f64::from(a);
            let aspect_rad = if a_f64 < 0.0 { 0.0 } else { a_f64.to_radians() };
            let raw = cos_z * s_f64.cos() + sin_z * s_f64.sin() * (az_rad - aspect_rad).cos();
            raw.clamp(0.15, 1.0)
        } else {
            0.78
        };
        let t = ((z - vmin) / span).clamp(0.0, 1.0);
        let base = terrain_rgb(t);
        let factor = 0.42 + 0.68 * shade;
        let p = i * 4;
        out[p] = (base[0] as f64 * factor).clamp(0.0, 255.0).round() as u8;
        out[p + 1] = (base[1] as f64 * factor).clamp(0.0, 255.0).round() as u8;
        out[p + 2] = (base[2] as f64 * factor).clamp(0.0, 255.0).round() as u8;
        out[p + 3] = 255;
    }
    out
}

#[derive(Serialize)]
struct RasterInfoJson {
    width: usize,
    height: usize,
    geotransform: [f64; 6],
    projection: String,
    no_data: f64,
    min: f64,
    max: f64,
    has_depressions: bool,
}

#[derive(Serialize)]
struct DepressionSummaryJson {
    id: u32,
    parent: Option<u32>,
    cell_count: u32,
    pit_elev: f64,
    out_elev: f64,
    depth: f64,
    dep_vol: f64,
    water_vol: f64,
    ocean_parent: bool,
}

/// Returns the `richdem-rs` crate version string.
#[wasm_bindgen]
pub fn version() -> String {
    richdem::VERSION.to_string()
}

/// Returns basic raster metadata, elevation range, and whether depressions are present as JSON.
#[wasm_bindgen(js_name = rasterInfo)]
pub fn raster_info(bytes: &[u8], filename: &str) -> Result<String, JsValue> {
    let dem = read_array2d_f64(bytes, filename)?;
    let (vmin, vmax) = raster_min_max(&dem);
    let nd = dem.no_data();
    let info = RasterInfoJson {
        width: dem.width(),
        height: dem.height(),
        geotransform: dem.geotransform,
        projection: dem.projection.clone(),
        no_data: if nd.is_finite() { nd } else { f64::NAN },
        min: vmin,
        max: vmax,
        has_depressions: has_depressions(&dem, Topology::D8),
    };
    serde_json::to_string(&info).map_err(to_js_err)
}

/// Generates a square synthetic Perlin-noise DEM GeoTIFF in memory for testing or demos.
#[wasm_bindgen(js_name = generatePerlinDem)]
pub fn generate_perlin_dem(
    size: usize,
    seed: u32,
    relief_m: f64,
    cell_size_m: f64,
) -> Result<Vec<u8>, JsValue> {
    if size == 0 || size > MAX_WASM_PERLIN_SIZE {
        return Err(to_js_err(format!(
            "size must be in 1..={MAX_WASM_PERLIN_SIZE}, got {size}"
        )));
    }
    if !relief_m.is_finite() || relief_m <= 0.0 {
        return Err(to_js_err(format!(
            "relief_m must be finite and > 0, got {relief_m}"
        )));
    }
    if !cell_size_m.is_finite() || cell_size_m <= 0.0 {
        return Err(to_js_err(format!(
            "cell_size_m must be finite and > 0, got {cell_size_m}"
        )));
    }
    let mut dem = try_generate_perlin_terrain(size, seed).map_err(to_js_err)?;
    for v in dem.data_mut() {
        *v = 100.0 + *v * relief_m;
    }
    dem.geotransform = [
        0.0,
        cell_size_m,
        0.0,
        (size as f64) * cell_size_m,
        0.0,
        -cell_size_m,
    ];
    dem.projection = "EPSG:3857".to_string();
    write_array2d_tif(&dem, "perlin_dem.tif")
}

/// Computes an RGBA shaded-relief preview buffer (`width * height * 4` bytes) for `<canvas>`.
#[wasm_bindgen(js_name = hillshadeRgba)]
pub fn hillshade_rgba(
    bytes: &[u8],
    filename: &str,
    azimuth_deg: f64,
    elevation_deg: f64,
    z_factor: f64,
) -> Result<Vec<u8>, JsValue> {
    let dem = read_array2d_f64(bytes, filename)?;
    Ok(compute_hillshade_rgba(
        &dem,
        azimuth_deg,
        elevation_deg,
        z_factor,
    ))
}

/// Generic single-raster output returned by depression, flat, flow, and terrain operations.
#[wasm_bindgen]
pub struct RasterOutput {
    width: usize,
    height: usize,
    min: f64,
    max: f64,
    modified_cells: usize,
    projection: String,
    tif_bytes: Vec<u8>,
    rgba: Vec<u8>,
    diff_rgba: Vec<u8>,
}

#[wasm_bindgen]
impl RasterOutput {
    #[wasm_bindgen(getter)]
    pub fn width(&self) -> usize {
        self.width
    }
    #[wasm_bindgen(getter)]
    pub fn height(&self) -> usize {
        self.height
    }
    #[wasm_bindgen(getter)]
    pub fn min(&self) -> f64 {
        self.min
    }
    #[wasm_bindgen(getter)]
    pub fn max(&self) -> f64 {
        self.max
    }
    #[wasm_bindgen(getter, js_name = modifiedCells)]
    pub fn modified_cells(&self) -> usize {
        self.modified_cells
    }
    #[wasm_bindgen(getter)]
    pub fn projection(&self) -> String {
        self.projection.clone()
    }
    #[wasm_bindgen(js_name = tifBytes)]
    pub fn tif_bytes(&self) -> Vec<u8> {
        self.tif_bytes.clone()
    }
    pub fn rgba(&self) -> Vec<u8> {
        self.rgba.clone()
    }
    #[wasm_bindgen(js_name = diffRgba)]
    pub fn diff_rgba(&self) -> Vec<u8> {
        self.diff_rgba.clone()
    }
}

fn diff_and_count(orig: &Array2D<f64>, modified: &Array2D<f64>) -> (Array2D<f64>, usize) {
    let nd = orig.no_data();
    let mut count = 0usize;
    let diff_data: Vec<f64> = orig
        .data()
        .iter()
        .zip(modified.data().iter())
        .map(|(&a, &b)| {
            if !a.is_finite() || a.rd_eq(nd) || !b.is_finite() || b.rd_eq(nd) {
                0.0
            } else {
                let d = (b - a).abs();
                if d > 1e-12 {
                    count += 1;
                }
                d
            }
        })
        .collect();
    let mut diff = Array2D::from_vec(orig.width(), orig.height(), diff_data)
        .expect("diff dimensions match input");
    diff.copy_metadata_from(orig);
    diff.set_no_data(0.0);
    (diff, count)
}

/// Fills depressions in a DEM using Priority-Flood (Barnes et al., 2014).
#[wasm_bindgen(js_name = fillDepressions)]
pub fn fill_depressions(
    bytes: &[u8],
    filename: &str,
    epsilon: bool,
    topology: &str,
) -> Result<RasterOutput, JsValue> {
    let topo = parse_topology(topology)?;
    let orig = read_array2d_f64(bytes, filename)?;
    let mut dem = orig.clone();
    if epsilon {
        core_fill_depressions_epsilon(&mut dem, topo).map_err(to_js_err)?;
    } else {
        core_fill_depressions(&mut dem, topo);
    }
    let (diff, modified_cells) = diff_and_count(&orig, &dem);
    let (min, max) = raster_min_max(&dem);
    let tif_bytes = write_array2d_tif(&dem, "filled.tif")?;
    let rgba = compute_hillshade_rgba(&dem, 315.0, 45.0, 1.0);
    let diff_rgba = colorize_f64_rgba(&diff, Ramp::Blues, false, true);
    Ok(RasterOutput {
        width: dem.width(),
        height: dem.height(),
        min,
        max,
        modified_cells,
        projection: dem.projection,
        tif_bytes,
        rgba,
        diff_rgba,
    })
}

/// Breaches depressions in a DEM using Lindsay (2016) complete, selective, or constrained breaching.
#[wasm_bindgen(js_name = breachDepressions)]
#[allow(clippy::too_many_arguments)]
pub fn breach_depressions(
    bytes: &[u8],
    filename: &str,
    mode: &str,
    max_path_len: u32,
    max_depth: f64,
    eps_gradients: bool,
    fill_remaining: bool,
) -> Result<RasterOutput, JsValue> {
    let lindsay_mode = match mode.to_ascii_lowercase().as_str() {
        "" | "complete" => LindsayMode::CompleteBreaching,
        "selective" => LindsayMode::SelectiveBreaching,
        "constrained" => LindsayMode::ConstrainedBreaching,
        other => {
            return Err(to_js_err(format!(
                "invalid breach mode '{other}': expected 'complete', 'selective', or 'constrained'"
            )));
        }
    };
    let orig = read_array2d_f64(bytes, filename)?;
    let mut dem = orig.clone();
    lindsay2016(
        &mut dem,
        lindsay_mode,
        eps_gradients,
        fill_remaining,
        max_path_len,
        max_depth,
    )
    .map_err(to_js_err)?;
    let (diff, modified_cells) = diff_and_count(&orig, &dem);
    let (min, max) = raster_min_max(&dem);
    let tif_bytes = write_array2d_tif(&dem, "breached.tif")?;
    let rgba = compute_hillshade_rgba(&dem, 315.0, 45.0, 1.0);
    let diff_rgba = colorize_f64_rgba(&diff, Ramp::Viridis, false, true);
    Ok(RasterOutput {
        width: dem.width(),
        height: dem.height(),
        min,
        max,
        modified_cells,
        projection: dem.projection,
        tif_bytes,
        rgba,
        diff_rgba,
    })
}

/// Resolves flat surfaces in a DEM using `"epsilon"`, `"barnes"`, or `"garbrecht"`.
#[wasm_bindgen(js_name = resolveFlats)]
pub fn resolve_flats(bytes: &[u8], filename: &str, method: &str) -> Result<RasterOutput, JsValue> {
    let orig = read_array2d_f64(bytes, filename)?;
    let mut dem = orig.clone();
    match method.to_ascii_lowercase().as_str() {
        "" | "epsilon" => {
            resolve_flats_epsilon(&mut dem).map_err(to_js_err)?;
        }
        "barnes" => {
            let (flat_mask, labels) = get_flat_mask(&dem);
            d8_flats_alter_dem(&flat_mask, &labels, &mut dem).map_err(to_js_err)?;
        }
        "garbrecht" => {
            let mut flowdirs = d8_flow_directions(&dem);
            garbrecht_alg(&mut dem, &mut flowdirs, 0.001).map_err(to_js_err)?;
        }
        other => {
            return Err(to_js_err(format!(
                "invalid flat resolution method '{other}': expected 'epsilon', 'barnes', or 'garbrecht'"
            )));
        }
    }
    let (diff, modified_cells) = diff_and_count(&orig, &dem);
    let (min, max) = raster_min_max(&dem);
    let tif_bytes = write_array2d_tif(&dem, "flats_resolved.tif")?;
    let rgba = compute_hillshade_rgba(&dem, 315.0, 45.0, 1.0);
    let diff_rgba = colorize_f64_rgba(&diff, Ramp::Viridis, false, true);
    Ok(RasterOutput {
        width: dem.width(),
        height: dem.height(),
        min,
        max,
        modified_cells,
        projection: dem.projection,
        tif_bytes,
        rgba,
        diff_rgba,
    })
}

/// Computes flow accumulation (`Dinf`, `D8`, `D4`, `Quinn`, `Holmgren`, `Freeman`, `Rho8`, `Rho4`).
///
/// When `prefill` is `true`, depressions are first filled with Priority-Flood+Epsilon.
/// Pass `exponent <= 0` or `NaN` for methods that do not use an exponent.
#[wasm_bindgen(js_name = flowAccumulation)]
pub fn flow_accumulation(
    bytes: &[u8],
    filename: &str,
    method: &str,
    exponent: f64,
    prefill: bool,
    log_preview: bool,
) -> Result<RasterOutput, JsValue> {
    let fa_method = FlowAccumMethod::parse(method).map_err(to_js_err)?;
    let mut dem = read_array2d_f64(bytes, filename)?;
    if prefill {
        core_fill_depressions_epsilon(&mut dem, Topology::D8).map_err(to_js_err)?;
    }
    let mut accum = Array2D::make_from_template(&dem, 1.0f64);
    let exp_opt = if exponent.is_finite() && exponent > 0.0 {
        Some(exponent)
    } else {
        None
    };
    richdem::flow_accumulation_by_method(&dem, &mut accum, fa_method, exp_opt)
        .map_err(to_js_err)?;
    let (min, max) = raster_min_max(&accum);
    let tif_bytes = write_array2d_tif(&accum, "flow_accum.tif")?;
    let rgba = colorize_f64_rgba(&accum, Ramp::Blues, log_preview, true);
    Ok(RasterOutput {
        width: accum.width(),
        height: accum.height(),
        min,
        max,
        modified_cells: 0,
        projection: accum.projection,
        tif_bytes,
        rgba,
        diff_rgba: Vec::new(),
    })
}

/// Computes a terrain attribute (`slope_degrees`, `slope_percentage`, `slope_riserun`,
/// `slope_radians`, `aspect`, `curvature`, `planform_curvature`, `profile_curvature`,
/// `spi`, or `cti`).
#[wasm_bindgen(js_name = terrainAttribute)]
pub fn terrain_attribute(
    bytes: &[u8],
    filename: &str,
    attrib: &str,
    z_scale: f64,
) -> Result<RasterOutput, JsValue> {
    if !z_scale.is_finite() || z_scale <= 0.0 {
        return Err(to_js_err(format!(
            "z_scale must be finite and > 0, got {z_scale}"
        )));
    }
    let dem = read_array2d_f64(bytes, filename)?;
    let (out, ramp) = match attrib.to_ascii_lowercase().as_str() {
        "spi" | "cti" => {
            let mut filled = dem.clone();
            core_fill_depressions_epsilon(&mut filled, Topology::D8).map_err(to_js_err)?;
            let mut accum = Array2D::make_from_template(&filled, 1.0f64);
            richdem::flow_accumulation_by_method(&filled, &mut accum, FlowAccumMethod::Dinf, None)
                .map_err(to_js_err)?;
            let slope = ta_slope_riserun(&filled, z_scale as f32);
            let res = if attrib.eq_ignore_ascii_case("spi") {
                ta_spi(&accum, &slope).map_err(to_js_err)?
            } else {
                ta_cti(&accum, &slope).map_err(to_js_err)?
            };
            (res, Ramp::Viridis)
        }
        other => {
            let ta = TerrainAttr::parse(other).map_err(to_js_err)?;
            let ramp = match ta {
                TerrainAttr::Curvature
                | TerrainAttr::PlanformCurvature
                | TerrainAttr::ProfileCurvature => Ramp::Diverging,
                _ => Ramp::Viridis,
            };
            let res_f32 = core_terrain_attribute(&dem, ta, z_scale as f32);
            (f32_array_to_f64(&res_f32), ramp)
        }
    };
    let (min, max) = raster_min_max(&out);
    let tif_bytes = write_array2d_tif(&out, "terrain_attr.tif")?;
    let rgba = colorize_f64_rgba(&out, ramp, false, false);
    Ok(RasterOutput {
        width: out.width(),
        height: out.height(),
        min,
        max,
        modified_cells: 0,
        projection: out.projection,
        tif_bytes,
        rgba,
        diff_rgba: Vec::new(),
    })
}

/// Output container returned by [`depression_hierarchy_fsm`].
#[wasm_bindgen]
pub struct FsmOutput {
    width: usize,
    height: usize,
    projection: String,
    num_depressions: usize,
    inundated_cells: usize,
    max_water_depth: f64,
    total_water_volume: f64,
    depressions_json: String,
    water_depth_tif: Vec<u8>,
    labels_tif: Vec<u8>,
    hillshade_rgba: Vec<u8>,
    water_depth_rgba: Vec<u8>,
    labels_rgba: Vec<u8>,
}

#[wasm_bindgen]
impl FsmOutput {
    #[wasm_bindgen(getter)]
    pub fn width(&self) -> usize {
        self.width
    }
    #[wasm_bindgen(getter)]
    pub fn height(&self) -> usize {
        self.height
    }
    #[wasm_bindgen(getter)]
    pub fn projection(&self) -> String {
        self.projection.clone()
    }
    #[wasm_bindgen(getter, js_name = numDepressions)]
    pub fn num_depressions(&self) -> usize {
        self.num_depressions
    }
    #[wasm_bindgen(getter, js_name = inundatedCells)]
    pub fn inundated_cells(&self) -> usize {
        self.inundated_cells
    }
    #[wasm_bindgen(getter, js_name = maxWaterDepth)]
    pub fn max_water_depth(&self) -> f64 {
        self.max_water_depth
    }
    #[wasm_bindgen(getter, js_name = totalWaterVolume)]
    pub fn total_water_volume(&self) -> f64 {
        self.total_water_volume
    }
    #[wasm_bindgen(js_name = depressionsJson)]
    pub fn depressions_json(&self) -> String {
        self.depressions_json.clone()
    }
    #[wasm_bindgen(js_name = waterDepthTif)]
    pub fn water_depth_tif(&self) -> Vec<u8> {
        self.water_depth_tif.clone()
    }
    #[wasm_bindgen(js_name = labelsTif)]
    pub fn labels_tif(&self) -> Vec<u8> {
        self.labels_tif.clone()
    }
    #[wasm_bindgen(js_name = hillshadeRgba)]
    pub fn hillshade_rgba(&self) -> Vec<u8> {
        self.hillshade_rgba.clone()
    }
    #[wasm_bindgen(js_name = waterDepthRgba)]
    pub fn water_depth_rgba(&self) -> Vec<u8> {
        self.water_depth_rgba.clone()
    }
    #[wasm_bindgen(js_name = labelsRgba)]
    pub fn labels_rgba(&self) -> Vec<u8> {
        self.labels_rgba.clone()
    }
}

/// Computes the Barnes et al. (2020) Depression Hierarchy and executes the Barnes et al. (2021)
/// Fill-Spill-Merge water-routing algorithm with uniform standing rainfall `water_depth`.
#[wasm_bindgen(js_name = depressionHierarchyFsm)]
pub fn depression_hierarchy_fsm(
    bytes: &[u8],
    filename: &str,
    water_depth: f64,
    topology: &str,
) -> Result<FsmOutput, JsValue> {
    if !water_depth.is_finite() || water_depth < 0.0 {
        return Err(to_js_err(format!(
            "water_depth must be finite and >= 0, got {water_depth}"
        )));
    }
    let topo = parse_topology(topology)?;
    let dem = read_array2d_f64(bytes, filename)?;
    let w = dem.width();
    let h = dem.height();

    let mut labels = get_new_depression_hierarchy_labels(w, h, Some(dem.geotransform));
    labels.copy_metadata_from(&dem);
    let mut flowdirs = Array2D::<Flowdir>::make_from_template(&dem, NO_FLOW);
    let mut deps =
        get_depression_hierarchy(&dem, &mut labels, &mut flowdirs, topo).map_err(to_js_err)?;

    let mut wtd = Array2D::<f64>::make_from_template(&dem, water_depth);
    wtd.set_no_data(-1.0);
    // Keep ocean boundary cells dry so standing water exits at the DEM boundary.
    for y in 0..h {
        for x in 0..w {
            if dem.is_no_data(x, y) || labels.get(x, y) == OCEAN {
                wtd.set(x, y, 0.0);
            }
        }
    }

    fill_spill_merge(&dem, &labels, &flowdirs, &mut deps, &mut wtd).map_err(to_js_err)?;

    let cell_area = dem.get_cell_area();
    let mut inundated_cells = 0usize;
    let mut max_water_depth = 0.0f64;
    let mut total_water_volume = 0.0f64;
    for &d in wtd.data() {
        if d.is_finite() && d > 1e-9 {
            inundated_cells += 1;
            if d > max_water_depth {
                max_water_depth = d;
            }
            total_water_volume += d * cell_area;
        }
    }

    let mut summaries: Vec<DepressionSummaryJson> = deps
        .iter()
        .skip(1) // skip OCEAN root (id 0)
        .map(|d| DepressionSummaryJson {
            id: d.dep_label,
            parent: if d.parent == NO_VALUE || d.parent == OCEAN {
                None
            } else {
                Some(d.parent)
            },
            cell_count: d.cell_count,
            pit_elev: if d.pit_elev.is_finite() {
                d.pit_elev
            } else {
                0.0
            },
            out_elev: if d.out_elev.is_finite() {
                d.out_elev
            } else {
                0.0
            },
            depth: if d.out_elev.is_finite() && d.pit_elev.is_finite() {
                (d.out_elev - d.pit_elev).max(0.0)
            } else {
                0.0
            },
            dep_vol: d.dep_vol,
            water_vol: d.water_vol,
            ocean_parent: d.ocean_parent,
        })
        .collect();
    summaries.sort_by(|a, b| b.dep_vol.total_cmp(&a.dep_vol));

    let num_depressions = summaries.len();
    let depressions_json = serde_json::to_string(&summaries).map_err(to_js_err)?;
    let water_depth_tif = write_array2d_tif(&wtd, "water_depth.tif")?;
    let labels_tif = write_array2d_tif(&labels, "depression_labels.tif")?;
    let hillshade_rgba = compute_hillshade_rgba(&dem, 315.0, 45.0, 1.0);
    let water_depth_rgba = colorize_f64_rgba(&wtd, Ramp::Blues, false, true);
    let labels_rgba = colorize_labels_rgba(&labels);

    Ok(FsmOutput {
        width: w,
        height: h,
        projection: dem.projection,
        num_depressions,
        inundated_cells,
        max_water_depth,
        total_water_volume,
        depressions_json,
        water_depth_tif,
        labels_tif,
        hillshade_rgba,
        water_depth_rgba,
        labels_rgba,
    })
}
