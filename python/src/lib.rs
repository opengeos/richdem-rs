//! PyO3 bindings exposing the full RichDEM API backed by pure-Rust `richdem` and `terrio`.

use std::collections::BTreeMap;

use numpy::ndarray::{Array2, Array3};
use numpy::{
    IntoPyArray, PyArray2, PyArray3, PyReadonlyArray2, PyReadonlyArray3, PyReadwriteArray2,
};
use pyo3::exceptions::{PyRuntimeError, PyValueError};
use pyo3::prelude::*;

use richdem::{
    Array2D, Array3D, Depression, DhLabel, DynamicArray2D, FLOW_PROPS_DEPTH, FlowAccumMethod,
    Flowdir, NO_DEP, NO_FLOW, NO_PARENT, NO_VALUE, OCEAN, TerrainAttr, Topology,
    breach_depressions, convert_arc_flowdirs_to_richdem_d8, convert_taudem_d8_to_richdem_d8,
    d8_flow_accum, d8_flow_directions, dem_surface_area, fill_depressions,
    fill_depressions_epsilon, fill_spill_merge, flow_accumulation, flow_accumulation_by_method,
    flow_accumulation_from_d8, flow_proportions_by_method, try_generate_perlin_terrain,
    get_depression_hierarchy, has_depressions, load_dynamic, pit_mask, resolve_flats_epsilon,
    save_raster, ta_cti, ta_spi, terrain_attribute,
};

fn to_py_err(e: richdem::Error) -> PyErr {
    match e {
        richdem::Error::InvalidArgument(_) | richdem::Error::DimensionMismatch(_) => {
            PyValueError::new_err(e.to_string())
        }
        other => PyRuntimeError::new_err(other.to_string()),
    }
}

fn parse_topology(topo: &str) -> PyResult<Topology> {
    match topo {
        "D8" | "d8" => Ok(Topology::D8),
        "D4" | "d4" => Ok(Topology::D4),
        other => Err(PyValueError::new_err(format!("Unknown topology: {other}"))),
    }
}

fn from_py_array2<T: richdem::DemNum + numpy::Element>(
    arr: PyReadonlyArray2<'_, T>,
    no_data: T,
    geotransform: Option<[f64; 6]>,
) -> PyResult<Array2D<T>> {
    let view = arr.as_array();
    let (h, w) = (view.nrows(), view.ncols());
    let data: Vec<T> = view.iter().copied().collect();
    let mut rd = Array2D::from_vec(w, h, data).map_err(to_py_err)?;
    rd.set_no_data(no_data);
    if let Some(gt) = geotransform {
        rd.geotransform = gt;
    }
    Ok(rd)
}

fn from_py_array2_f64(
    arr: PyReadonlyArray2<'_, f64>,
    no_data: f64,
    geotransform: Option<[f64; 6]>,
) -> PyResult<Array2D<f64>> {
    from_py_array2(arr, no_data, geotransform)
}

fn to_py_array2<'py, T: richdem::DemNum + numpy::Element>(
    py: Python<'py>,
    arr: Array2D<T>,
) -> PyResult<Bound<'py, PyArray2<T>>> {
    let (w, h) = (arr.width(), arr.height());
    let nd = Array2::from_shape_vec((h, w), arr.into_vec())
        .map_err(|e| PyRuntimeError::new_err(e.to_string()))?;
    Ok(nd.into_pyarray(py))
}

fn to_py_array3<'py>(py: Python<'py>, arr: Array3D<f32>) -> PyResult<Bound<'py, PyArray3<f32>>> {
    let (w, h) = (arr.width(), arr.height());
    let nd = Array3::from_shape_vec((h, w, FLOW_PROPS_DEPTH), arr.into_vec())
        .map_err(|e| PyRuntimeError::new_err(e.to_string()))?;
    Ok(nd.into_pyarray(py))
}

/// Python-exposed Depression node matching `richdem.depression_hierarchy.Depression`.
#[pyclass(name = "Depression", from_py_object)]
#[derive(Clone)]
pub struct PyDepression {
    #[pyo3(get, set)]
    pub pit_cell: u32,
    #[pyo3(get, set)]
    pub out_cell: u32,
    #[pyo3(get, set)]
    pub parent: u32,
    #[pyo3(get, set)]
    pub odep: u32,
    #[pyo3(get, set)]
    pub geolink: u32,
    #[pyo3(get, set)]
    pub pit_elev: f64,
    #[pyo3(get, set)]
    pub out_elev: f64,
    #[pyo3(get, set)]
    pub lchild: u32,
    #[pyo3(get, set)]
    pub rchild: u32,
    #[pyo3(get, set)]
    pub ocean_parent: bool,
    #[pyo3(get, set)]
    pub ocean_linked: Vec<u32>,
    #[pyo3(get, set)]
    pub dep_label: u32,
    #[pyo3(get, set)]
    pub cell_count: u32,
    #[pyo3(get, set)]
    pub dep_vol: f64,
    #[pyo3(get, set)]
    pub water_vol: f64,
    #[pyo3(get, set)]
    pub total_elevation: f64,
    topology: Topology,
}

#[pymethods]
impl PyDepression {
    #[new]
    fn new() -> Self {
        Self::from(Depression::<f64>::default())
    }
}

impl From<Depression<f64>> for PyDepression {
    fn from(d: Depression<f64>) -> Self {
        Self {
            pit_cell: d.pit_cell,
            out_cell: d.out_cell,
            parent: d.parent,
            odep: d.odep,
            geolink: d.geolink,
            pit_elev: d.pit_elev,
            out_elev: d.out_elev,
            lchild: d.lchild,
            rchild: d.rchild,
            ocean_parent: d.ocean_parent,
            ocean_linked: d.ocean_linked,
            dep_label: d.dep_label,
            cell_count: d.cell_count,
            dep_vol: d.dep_vol,
            water_vol: d.water_vol,
            total_elevation: d.total_elevation,
            topology: d.topology,
        }
    }
}

impl From<&PyDepression> for Depression<f64> {
    fn from(d: &PyDepression) -> Self {
        Self {
            pit_cell: d.pit_cell,
            out_cell: d.out_cell,
            parent: d.parent,
            odep: d.odep,
            geolink: d.geolink,
            pit_elev: d.pit_elev,
            out_elev: d.out_elev,
            lchild: d.lchild,
            rchild: d.rchild,
            ocean_parent: d.ocean_parent,
            ocean_linked: d.ocean_linked.clone(),
            dep_label: d.dep_label,
            cell_count: d.cell_count,
            dep_vol: d.dep_vol,
            water_vol: d.water_vol,
            total_elevation: d.total_elevation,
            topology: d.topology,
        }
    }
}

#[pyfunction]
fn load_raster_terrio<'py>(
    py: Python<'py>,
    filename: &str,
) -> PyResult<(Py<PyAny>, f64, [f64; 6], String, BTreeMap<String, String>)> {
    let dyn_arr = load_dynamic(filename).map_err(to_py_err)?;
    let nd = dyn_arr.no_data_f64();
    let gt = dyn_arr.geotransform();
    let proj = dyn_arr.projection().to_string();
    let meta = dyn_arr.metadata().clone();

    let py_arr: Py<PyAny> = match dyn_arr {
        DynamicArray2D::U8(a) => to_py_array2(py, a)?.into_any().unbind(),
        DynamicArray2D::I8(a) => to_py_array2(py, a)?.into_any().unbind(),
        DynamicArray2D::U16(a) => to_py_array2(py, a)?.into_any().unbind(),
        DynamicArray2D::I16(a) => to_py_array2(py, a)?.into_any().unbind(),
        DynamicArray2D::U32(a) => to_py_array2(py, a)?.into_any().unbind(),
        DynamicArray2D::I32(a) => to_py_array2(py, a)?.into_any().unbind(),
        DynamicArray2D::U64(a) => to_py_array2(py, a)?.into_any().unbind(),
        DynamicArray2D::I64(a) => to_py_array2(py, a)?.into_any().unbind(),
        DynamicArray2D::F32(a) => to_py_array2(py, a)?.into_any().unbind(),
        DynamicArray2D::F64(a) => to_py_array2(py, a)?.into_any().unbind(),
    };

    Ok((py_arr, nd, gt, proj, meta))
}

#[pyfunction]
#[pyo3(signature = (filename, data, no_data, geotransform, projection, metadata, compress=false))]
fn save_raster_terrio(
    filename: &str,
    data: &Bound<'_, PyAny>,
    no_data: &Bound<'_, PyAny>,
    geotransform: [f64; 6],
    projection: &str,
    metadata: BTreeMap<String, String>,
    compress: bool,
) -> PyResult<()> {
    macro_rules! try_save {
        ($ty:ty) => {
            if let Ok(arr) = data.extract::<PyReadonlyArray2<'_, $ty>>() {
                let nd: $ty = match no_data.extract::<$ty>() {
                    Ok(v) => v,
                    Err(_) => {
                        let f = no_data.extract::<f64>()?;
                        <$ty as richdem::DemNum>::from_nodata_f64(f)
                    }
                };
                let mut rd = from_py_array2::<$ty>(arr, nd, Some(geotransform))?;
                rd.projection = projection.to_string();
                rd.metadata = metadata;
                return save_raster(&rd, filename, None, compress).map_err(to_py_err);
            }
        };
    }

    try_save!(f64);
    try_save!(f32);
    try_save!(u8);
    try_save!(i8);
    try_save!(u16);
    try_save!(i16);
    try_save!(u32);
    try_save!(i32);
    try_save!(u64);
    try_save!(i64);

    Err(PyValueError::new_err(
        "Unsupported array dtype for SaveGDAL; expected a 2D array of u8/i8/u16/i16/u32/i32/u64/i64/f32/f64",
    ))
}

#[pyfunction]
fn fill_depressions_f64<'py>(
    py: Python<'py>,
    dem: PyReadonlyArray2<'_, f64>,
    no_data: f64,
    epsilon: bool,
    topology: &str,
) -> PyResult<Bound<'py, PyArray2<f64>>> {
    let topo = parse_topology(topology)?;
    let mut rd = from_py_array2_f64(dem, no_data, None)?;
    if epsilon {
        fill_depressions_epsilon(&mut rd, topo).map_err(to_py_err)?;
    } else {
        fill_depressions(&mut rd, topo);
    }
    to_py_array2(py, rd)
}

#[pyfunction]
fn fill_depressions_f32<'py>(
    py: Python<'py>,
    dem: PyReadonlyArray2<'_, f32>,
    no_data: f32,
    epsilon: bool,
    topology: &str,
) -> PyResult<Bound<'py, PyArray2<f32>>> {
    let topo = parse_topology(topology)?;
    let mut rd = from_py_array2::<f32>(dem, no_data, None)?;
    if epsilon {
        fill_depressions_epsilon(&mut rd, topo).map_err(to_py_err)?;
    } else {
        fill_depressions(&mut rd, topo);
    }
    to_py_array2(py, rd)
}

#[pyfunction]
fn breach_depressions_f64<'py>(
    py: Python<'py>,
    dem: PyReadonlyArray2<'_, f64>,
    no_data: f64,
    topology: &str,
) -> PyResult<Bound<'py, PyArray2<f64>>> {
    let topo = parse_topology(topology)?;
    let mut rd = from_py_array2_f64(dem, no_data, None)?;
    breach_depressions(&mut rd, topo);
    to_py_array2(py, rd)
}

#[pyfunction]
fn resolve_flats_f64<'py>(
    py: Python<'py>,
    dem: PyReadonlyArray2<'_, f64>,
    no_data: f64,
) -> PyResult<Bound<'py, PyArray2<f64>>> {
    let mut rd = from_py_array2_f64(dem, no_data, None)?;
    resolve_flats_epsilon(&mut rd).map_err(to_py_err)?;
    to_py_array2(py, rd)
}

#[pyfunction]
fn resolve_flats_f32<'py>(
    py: Python<'py>,
    dem: PyReadonlyArray2<'_, f32>,
    no_data: f32,
) -> PyResult<Bound<'py, PyArray2<f32>>> {
    let mut rd = from_py_array2::<f32>(dem, no_data, None)?;
    resolve_flats_epsilon(&mut rd).map_err(to_py_err)?;
    to_py_array2(py, rd)
}

#[pyfunction]
fn has_depressions_f64(
    dem: PyReadonlyArray2<'_, f64>,
    no_data: f64,
    topology: &str,
) -> PyResult<bool> {
    let topo = parse_topology(topology)?;
    let rd = from_py_array2_f64(dem, no_data, None)?;
    Ok(has_depressions(&rd, topo))
}

#[pyfunction]
fn pit_mask_f64<'py>(
    py: Python<'py>,
    dem: PyReadonlyArray2<'_, f64>,
    no_data: f64,
    topology: &str,
) -> PyResult<Bound<'py, PyArray2<u8>>> {
    let topo = parse_topology(topology)?;
    let rd = from_py_array2_f64(dem, no_data, None)?;
    to_py_array2(py, pit_mask(&rd, topo))
}

#[pyfunction]
#[pyo3(signature = (dem, no_data, method, exponent=None, weights=None))]
fn flow_accumulation_f64<'py>(
    py: Python<'py>,
    dem: PyReadonlyArray2<'_, f64>,
    no_data: f64,
    method: &str,
    exponent: Option<f64>,
    weights: Option<PyReadonlyArray2<'_, f64>>,
) -> PyResult<Bound<'py, PyArray2<f64>>> {
    let m = FlowAccumMethod::parse(method).map_err(to_py_err)?;
    let rd = from_py_array2_f64(dem, no_data, None)?;
    let mut accum = match weights {
        Some(w) => from_py_array2_f64(w, -1.0, None)?,
        None => Array2D::make_from_template(&rd, 1.0),
    };
    flow_accumulation_by_method(&rd, &mut accum, m, exponent).map_err(to_py_err)?;
    to_py_array2(py, accum)
}

#[pyfunction]
#[pyo3(signature = (dem, no_data, method, exponent=None))]
fn flow_proportions_f64<'py>(
    py: Python<'py>,
    dem: PyReadonlyArray2<'_, f64>,
    no_data: f64,
    method: &str,
    exponent: Option<f64>,
) -> PyResult<Bound<'py, PyArray3<f32>>> {
    let m = FlowAccumMethod::parse(method).map_err(to_py_err)?;
    let rd = from_py_array2_f64(dem, no_data, None)?;
    let props = flow_proportions_by_method(&rd, m, exponent).map_err(to_py_err)?;
    to_py_array3(py, props)
}

#[pyfunction]
#[pyo3(signature = (props, weights=None))]
fn flow_accum_from_props<'py>(
    py: Python<'py>,
    props: PyReadonlyArray3<'_, f32>,
    weights: Option<PyReadonlyArray2<'_, f64>>,
) -> PyResult<Bound<'py, PyArray2<f64>>> {
    let view = props.as_array();
    let shape = view.shape();
    if shape[2] != FLOW_PROPS_DEPTH {
        return Err(PyValueError::new_err(format!(
            "props must have shape (H, W, 9), got ({}, {}, {})",
            shape[0], shape[1], shape[2]
        )));
    }
    let (h, w) = (shape[0], shape[1]);
    let data: Vec<f32> = view.iter().copied().collect();
    let mut rd_props = Array3D::from_vec(w, h, data).map_err(to_py_err)?;
    rd_props.set_no_data(richdem::NO_DATA_GEN);

    let mut accum = match weights {
        Some(wt) => from_py_array2_f64(wt, -1.0, None)?,
        None => Array2D::new(w, h, 1.0),
    };
    flow_accumulation(&rd_props, &mut accum).map_err(to_py_err)?;
    to_py_array2(py, accum)
}

#[pyfunction]
fn flow_accumulation_from_d8_py<'py>(
    py: Python<'py>,
    d8: PyReadonlyArray2<'_, i32>,
    no_data: i32,
) -> PyResult<Bound<'py, PyArray2<u32>>> {
    let view = d8.as_array();
    let (h, w) = (view.nrows(), view.ncols());
    let mut rd = Array2D::from_vec(w, h, view.iter().copied().collect()).map_err(to_py_err)?;
    rd.set_no_data(no_data);
    to_py_array2(py, flow_accumulation_from_d8(&rd))
}

#[pyfunction]
fn d8_flow_accum_py<'py>(
    py: Python<'py>,
    d8: PyReadonlyArray2<'_, u8>,
    no_data: u8,
) -> PyResult<Bound<'py, PyArray2<i32>>> {
    let view = d8.as_array();
    let (h, w) = (view.nrows(), view.ncols());
    let mut rd = Array2D::from_vec(w, h, view.iter().copied().collect()).map_err(to_py_err)?;
    rd.set_no_data(no_data);
    to_py_array2(py, d8_flow_accum::<u8, i32>(&rd))
}

#[pyfunction]
fn d8_flow_directions_py<'py>(
    py: Python<'py>,
    dem: PyReadonlyArray2<'_, f64>,
    no_data: f64,
) -> PyResult<Bound<'py, PyArray2<u8>>> {
    let rd = from_py_array2_f64(dem, no_data, None)?;
    to_py_array2(py, d8_flow_directions(&rd))
}

#[pyfunction]
#[pyo3(signature = (arc, no_data=255))]
fn convert_arc_flowdirs_py<'py>(
    py: Python<'py>,
    arc: PyReadonlyArray2<'_, u8>,
    no_data: u8,
) -> PyResult<(Bound<'py, PyArray2<u8>>, u8)> {
    let view = arc.as_array();
    let (h, w) = (view.nrows(), view.ncols());
    let mut rd = Array2D::from_vec(w, h, view.iter().copied().collect()).map_err(to_py_err)?;
    rd.set_no_data(no_data);
    let out = convert_arc_flowdirs_to_richdem_d8(&rd).map_err(to_py_err)?;
    let out_nd = out.no_data();
    Ok((to_py_array2(py, out)?, out_nd))
}

#[pyfunction]
#[pyo3(signature = (td, no_data=255))]
fn convert_taudem_flowdirs_py<'py>(
    py: Python<'py>,
    td: PyReadonlyArray2<'_, u8>,
    no_data: u8,
) -> PyResult<(Bound<'py, PyArray2<u8>>, u8)> {
    let view = td.as_array();
    let (h, w) = (view.nrows(), view.ncols());
    let mut rd = Array2D::from_vec(w, h, view.iter().copied().collect()).map_err(to_py_err)?;
    rd.set_no_data(no_data);
    let out = convert_taudem_d8_to_richdem_d8(&rd).map_err(to_py_err)?;
    let out_nd = out.no_data();
    Ok((to_py_array2(py, out)?, out_nd))
}

#[pyfunction]
fn terrain_attribute_f64<'py>(
    py: Python<'py>,
    dem: PyReadonlyArray2<'_, f64>,
    no_data: f64,
    geotransform: [f64; 6],
    attrib: &str,
    zscale: f32,
) -> PyResult<Bound<'py, PyArray2<f32>>> {
    let attr = TerrainAttr::parse(attrib).map_err(to_py_err)?;
    let rd = from_py_array2_f64(dem, no_data, Some(geotransform))?;
    to_py_array2(py, terrain_attribute(&rd, attr, zscale))
}

#[pyfunction]
fn spi_f64<'py>(
    py: Python<'py>,
    flow_accum: PyReadonlyArray2<'_, f64>,
    fa_nodata: f64,
    slope_riserun: PyReadonlyArray2<'_, f32>,
    sl_nodata: f32,
    geotransform: [f64; 6],
) -> PyResult<Bound<'py, PyArray2<f64>>> {
    let fa = from_py_array2_f64(flow_accum, fa_nodata, Some(geotransform))?;
    let sv = slope_riserun.as_array();
    let mut sl = Array2D::from_vec(sv.ncols(), sv.nrows(), sv.iter().copied().collect())
        .map_err(to_py_err)?;
    sl.set_no_data(sl_nodata);
    to_py_array2(py, ta_spi(&fa, &sl).map_err(to_py_err)?)
}

#[pyfunction]
fn cti_f64<'py>(
    py: Python<'py>,
    flow_accum: PyReadonlyArray2<'_, f64>,
    fa_nodata: f64,
    slope_riserun: PyReadonlyArray2<'_, f32>,
    sl_nodata: f32,
    geotransform: [f64; 6],
) -> PyResult<Bound<'py, PyArray2<f64>>> {
    let fa = from_py_array2_f64(flow_accum, fa_nodata, Some(geotransform))?;
    let sv = slope_riserun.as_array();
    let mut sl = Array2D::from_vec(sv.ncols(), sv.nrows(), sv.iter().copied().collect())
        .map_err(to_py_err)?;
    sl.set_no_data(sl_nodata);
    to_py_array2(py, ta_cti(&fa, &sl).map_err(to_py_err)?)
}

#[pyfunction]
fn surface_area_f64(
    dem: PyReadonlyArray2<'_, f64>,
    no_data: f64,
    geotransform: [f64; 6],
    zscale: f64,
) -> PyResult<f64> {
    let rd = from_py_array2_f64(dem, no_data, Some(geotransform))?;
    dem_surface_area(&rd, zscale).map_err(to_py_err)
}

#[pyfunction]
fn generate_perlin_terrain_py<'py>(
    py: Python<'py>,
    size: usize,
    seed: u32,
) -> PyResult<Bound<'py, PyArray2<f64>>> {
    let rd = try_generate_perlin_terrain(size, seed).map_err(to_py_err)?;
    to_py_array2(py, rd)
}

#[pyfunction]
#[pyo3(signature = (dem, labels, topology, no_data=-9999.0))]
fn get_depression_hierarchy_py<'py>(
    py: Python<'py>,
    dem: PyReadonlyArray2<'_, f64>,
    mut labels: PyReadwriteArray2<'_, u32>,
    topology: &str,
    no_data: f64,
) -> PyResult<(Vec<PyDepression>, Bound<'py, PyArray2<i8>>)> {
    let topo = parse_topology(topology)?;
    let rd_dem = from_py_array2_f64(dem, no_data, None)?;
    let (h, w) = (rd_dem.height(), rd_dem.width());

    let mut lbl_view = labels.as_array_mut();
    if lbl_view.shape() != [h, w] {
        return Err(PyValueError::new_err(format!(
            "labels shape {:?} does not match dem shape [{h}, {w}]",
            lbl_view.shape()
        )));
    }
    let mut rd_labels = Array2D::<DhLabel>::from_vec(w, h, lbl_view.iter().copied().collect())
        .map_err(to_py_err)?;
    let mut rd_flowdirs = Array2D::<Flowdir>::new(w, h, NO_FLOW);

    let dh = get_depression_hierarchy(&rd_dem, &mut rd_labels, &mut rd_flowdirs, topo)
        .map_err(to_py_err)?;

    // Write modified labels back in place
    for y in 0..h {
        for x in 0..w {
            lbl_view[[y, x]] = rd_labels.get(x, y);
        }
    }

    let py_dh: Vec<PyDepression> = dh.into_iter().map(PyDepression::from).collect();
    let py_fds = to_py_array2(py, rd_flowdirs)?;
    Ok((py_dh, py_fds))
}

#[pyfunction]
#[pyo3(signature = (dem, labels, flowdirs, deps, wtd, no_data=-9999.0))]
fn fill_spill_merge_py(
    dem: PyReadonlyArray2<'_, f64>,
    labels: PyReadonlyArray2<'_, u32>,
    flowdirs: PyReadonlyArray2<'_, i8>,
    deps: Vec<Py<PyDepression>>,
    mut wtd: PyReadwriteArray2<'_, f64>,
    no_data: f64,
    py: Python<'_>,
) -> PyResult<()> {
    let rd_dem = from_py_array2_f64(dem, no_data, None)?;
    let (h, w) = (rd_dem.height(), rd_dem.width());

    let lbl_view = labels.as_array();
    if lbl_view.shape() != [h, w] {
        return Err(PyValueError::new_err(format!(
            "labels shape {:?} does not match dem shape [{h}, {w}]",
            lbl_view.shape()
        )));
    }
    let fds_view = flowdirs.as_array();
    if fds_view.shape() != [h, w] {
        return Err(PyValueError::new_err(format!(
            "flowdirs shape {:?} does not match dem shape [{h}, {w}]",
            fds_view.shape()
        )));
    }
    let mut wtd_view = wtd.as_array_mut();
    if wtd_view.shape() != [h, w] {
        return Err(PyValueError::new_err(format!(
            "wtd shape {:?} does not match dem shape [{h}, {w}]",
            wtd_view.shape()
        )));
    }

    let rd_labels = Array2D::<DhLabel>::from_vec(w, h, lbl_view.iter().copied().collect())
        .map_err(to_py_err)?;
    let rd_fds = Array2D::<Flowdir>::from_vec(w, h, fds_view.iter().copied().collect())
        .map_err(to_py_err)?;

    let mut rd_deps: Vec<Depression<f64>> = deps
        .iter()
        .map(|obj| Depression::from(&*obj.borrow(py)))
        .collect();

    let mut rd_wtd =
        Array2D::<f64>::from_vec(w, h, wtd_view.iter().copied().collect()).map_err(to_py_err)?;

    fill_spill_merge(&rd_dem, &rd_labels, &rd_fds, &mut rd_deps, &mut rd_wtd).map_err(to_py_err)?;

    // Write modified wtd and deps back in place
    for y in 0..h {
        for x in 0..w {
            wtd_view[[y, x]] = rd_wtd.get(x, y);
        }
    }
    for (py_obj, rust_dep) in deps.iter().zip(rd_deps.into_iter()) {
        *py_obj.borrow_mut(py) = PyDepression::from(rust_dep);
    }

    Ok(())
}

#[pymodule]
fn _richdem_rs(py: Python<'_>, m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add("NO_FLOW", NO_FLOW)?;
    m.add("__version__", richdem::VERSION)?;

    m.add_function(wrap_pyfunction!(load_raster_terrio, m)?)?;
    m.add_function(wrap_pyfunction!(save_raster_terrio, m)?)?;
    m.add_function(wrap_pyfunction!(fill_depressions_f64, m)?)?;
    m.add_function(wrap_pyfunction!(fill_depressions_f32, m)?)?;
    m.add_function(wrap_pyfunction!(breach_depressions_f64, m)?)?;
    m.add_function(wrap_pyfunction!(resolve_flats_f64, m)?)?;
    m.add_function(wrap_pyfunction!(resolve_flats_f32, m)?)?;
    m.add_function(wrap_pyfunction!(has_depressions_f64, m)?)?;
    m.add_function(wrap_pyfunction!(pit_mask_f64, m)?)?;
    m.add_function(wrap_pyfunction!(flow_accumulation_f64, m)?)?;
    m.add_function(wrap_pyfunction!(flow_proportions_f64, m)?)?;
    m.add_function(wrap_pyfunction!(flow_accum_from_props, m)?)?;
    m.add_function(wrap_pyfunction!(flow_accumulation_from_d8_py, m)?)?;
    m.add_function(wrap_pyfunction!(d8_flow_accum_py, m)?)?;
    m.add_function(wrap_pyfunction!(d8_flow_directions_py, m)?)?;
    m.add_function(wrap_pyfunction!(convert_arc_flowdirs_py, m)?)?;
    m.add_function(wrap_pyfunction!(convert_taudem_flowdirs_py, m)?)?;
    m.add_function(wrap_pyfunction!(terrain_attribute_f64, m)?)?;
    m.add_function(wrap_pyfunction!(spi_f64, m)?)?;
    m.add_function(wrap_pyfunction!(cti_f64, m)?)?;
    m.add_function(wrap_pyfunction!(surface_area_f64, m)?)?;
    m.add_function(wrap_pyfunction!(generate_perlin_terrain_py, m)?)?;

    let dephier = PyModule::new(py, "depression_hierarchy")?;
    dephier.add("NO_PARENT", NO_PARENT)?;
    dephier.add("NO_VALUE", NO_VALUE)?;
    dephier.add("NO_DEP", NO_DEP)?;
    dephier.add("OCEAN", OCEAN)?;
    dephier.add_class::<PyDepression>()?;
    dephier.add_function(wrap_pyfunction!(get_depression_hierarchy_py, &dephier)?)?;
    dephier.add_function(wrap_pyfunction!(fill_spill_merge_py, &dephier)?)?;
    m.add_submodule(&dephier)?;

    Ok(())
}
