//! Terrain attributes: slope, aspect, curvatures, SPI, and CTI.
//!
//! References:
//! - Horn, B.K.P., 1981. Hill shading and the reflectance map. Proc. IEEE 69, 14-47.
//! - Zevenbergen, L.W., Thorne, C.R., 1987. Quantitative analysis of land surface
//!   topography. Earth Surface Processes and Landforms 12, 47-56.
//! - Burrough, P.A., McDonnell, R.A., 1998. Principles of Geographical Information Systems.

use std::f64::consts::PI;

use crate::array2d::{Array2D, for_each_row_mut};
use crate::error::{Error, Result};
use crate::num::DemNum;

/// Named terrain attribute matching RichDEM's Python & CLI API.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TerrainAttr {
    /// Rise/run slope (Horn, 1981).
    SlopeRiseRun,
    /// Percent slope (`rise/run * 100`).
    SlopePercentage,
    /// Slope in degrees (`atan(rise/run) * 180 / pi`).
    SlopeDegrees,
    /// Slope in radians (`atan(rise/run)`).
    SlopeRadians,
    /// Aspect in degrees clockwise from north (Horn, 1981).
    Aspect,
    /// Total curvature (Zevenbergen & Thorne, 1987).
    Curvature,
    /// Planform curvature (Zevenbergen & Thorne, 1987).
    PlanformCurvature,
    /// Profile curvature (Zevenbergen & Thorne, 1987).
    ProfileCurvature,
}

impl TerrainAttr {
    /// Parses a RichDEM terrain attribute string.
    pub fn parse(s: &str) -> Result<Self> {
        match s {
            "slope_riserun" => Ok(Self::SlopeRiseRun),
            "slope_percentage" => Ok(Self::SlopePercentage),
            "slope_degrees" => Ok(Self::SlopeDegrees),
            "slope_radians" => Ok(Self::SlopeRadians),
            "aspect" => Ok(Self::Aspect),
            "curvature" => Ok(Self::Curvature),
            "planform_curvature" => Ok(Self::PlanformCurvature),
            "profile_curvature" => Ok(Self::ProfileCurvature),
            other => Err(Error::InvalidArgument(format!(
                "Invalid TerrainAttribute '{other}'. Valid attributes are: slope_riserun, slope_percentage, slope_degrees, slope_radians, aspect, curvature, planform_curvature, profile_curvature"
            ))),
        }
    }
}

struct SetupVars {
    a: f64,
    b: f64,
    c: f64,
    d: f64,
    e: f64,
    f: f64,
    g: f64,
    h: f64,
    i: f64,
}

struct CurveVars {
    d: f64,
    e: f64,
    f: f64,
    g: f64,
    h: f64,
}

#[inline]
fn terrain_setup<T: DemNum>(elevations: &Array2D<T>, x: usize, y: usize, zscale: f32) -> SetupVars {
    let xi = x as i32;
    let yi = y as i32;
    let center = elevations.get(x, y).to_f64();
    let sample = |dx: i32, dy: i32| -> f64 {
        let nx = xi + dx;
        let ny = yi + dy;
        if elevations.in_grid(nx, ny) && !elevations.is_no_data_xy(nx, ny) {
            elevations.get_xy(nx, ny).to_f64()
        } else {
            center
        }
    };
    let zs = zscale as f64;
    SetupVars {
        a: sample(-1, -1) * zs,
        b: sample(0, -1) * zs,
        c: sample(1, -1) * zs,
        d: sample(-1, 0) * zs,
        e: center * zs,
        f: sample(1, 0) * zs,
        g: sample(-1, 1) * zs,
        h: sample(0, 1) * zs,
        i: sample(1, 1) * zs,
    }
}

#[inline]
fn terrain_curvature_setup<T: DemNum>(
    elevations: &Array2D<T>,
    x: usize,
    y: usize,
    zscale: f32,
) -> CurveVars {
    let tsv = terrain_setup(elevations, x, y, zscale);
    let dx = elevations.get_cell_length_x();
    let dy = elevations.get_cell_length_y();
    CurveVars {
        d: ((tsv.d + tsv.f) / 2.0 - tsv.e) / (dx * dx),
        e: ((tsv.b + tsv.h) / 2.0 - tsv.e) / (dy * dy),
        f: (-tsv.a + tsv.c + tsv.g - tsv.i) / (4.0 * dx * dy),
        g: (-tsv.d + tsv.f) / (2.0 * dx),
        h: (tsv.b - tsv.h) / (2.0 * dy),
    }
}

#[inline]
fn cell_slope_riserun<T: DemNum>(elevations: &Array2D<T>, x: usize, y: usize, zscale: f32) -> f64 {
    let tsv = terrain_setup(elevations, x, y, zscale);
    let dzdx = ((tsv.c + 2.0 * tsv.f + tsv.i) - (tsv.a + 2.0 * tsv.d + tsv.g))
        / 8.0
        / elevations.get_cell_length_x();
    let dzdy = ((tsv.g + 2.0 * tsv.h + tsv.i) - (tsv.a + 2.0 * tsv.b + tsv.c))
        / 8.0
        / elevations.get_cell_length_y();
    (dzdx * dzdx + dzdy * dzdy).sqrt()
}

#[inline]
fn cell_aspect<T: DemNum>(elevations: &Array2D<T>, x: usize, y: usize, zscale: f32) -> f64 {
    let tsv = terrain_setup(elevations, x, y, zscale);
    let dzdx = ((tsv.c + 2.0 * tsv.f + tsv.i) - (tsv.a + 2.0 * tsv.d + tsv.g))
        / 8.0
        / elevations.get_cell_length_x();
    let dzdy = ((tsv.g + 2.0 * tsv.h + tsv.i) - (tsv.a + 2.0 * tsv.b + tsv.c))
        / 8.0
        / elevations.get_cell_length_y();
    let the_aspect = (180.0 / PI) * dzdy.atan2(-dzdx);
    if the_aspect < 0.0 {
        90.0 - the_aspect
    } else if the_aspect > 90.0 {
        360.0 - the_aspect + 90.0
    } else {
        90.0 - the_aspect
    }
}

#[inline]
fn cell_curvature<T: DemNum>(elevations: &Array2D<T>, x: usize, y: usize, zscale: f32) -> f64 {
    let tscv = terrain_curvature_setup(elevations, x, y, zscale);
    -2.0 * (tscv.d + tscv.e) * 100.0
}

#[inline]
fn cell_planform_curvature<T: DemNum>(
    elevations: &Array2D<T>,
    x: usize,
    y: usize,
    zscale: f32,
) -> f64 {
    let p = terrain_curvature_setup(elevations, x, y, zscale);
    if p.g == 0.0 && p.h == 0.0 {
        0.0
    } else {
        -2.0 * (p.d * p.h * p.h + p.e * p.g * p.g - p.f * p.g * p.h) / (p.g * p.g + p.h * p.h)
            * 100.0
    }
}

#[inline]
fn cell_profile_curvature<T: DemNum>(
    elevations: &Array2D<T>,
    x: usize,
    y: usize,
    zscale: f32,
) -> f64 {
    let p = terrain_curvature_setup(elevations, x, y, zscale);
    if p.g == 0.0 && p.h == 0.0 {
        0.0
    } else {
        2.0 * (p.d * p.g * p.g + p.e * p.h * p.h + p.f * p.g * p.h) / (p.g * p.g + p.h * p.h)
            * 100.0
    }
}

fn terrain_processor<T: DemNum, F>(elevations: &Array2D<T>, zscale: f32, func: F) -> Array2D<f32>
where
    F: Fn(&Array2D<T>, usize, usize, f32) -> f64 + Sync,
{
    let w = elevations.width();
    let mut output = Array2D::<f32>::make_from_template(elevations, 0.0);
    output.set_no_data(-9999.0);
    let nd = output.no_data();

    for_each_row_mut(output.data_mut(), w, |y, row| {
        for (x, dst) in row.iter_mut().enumerate() {
            *dst = if elevations.is_no_data(x, y) {
                nd
            } else {
                func(elevations, x, y, zscale) as f32
            };
        }
    });

    output
}

/// Calculates a terrain attribute by [`TerrainAttr`] selector.
pub fn terrain_attribute<T: DemNum>(
    elevations: &Array2D<T>,
    attrib: TerrainAttr,
    zscale: f32,
) -> Array2D<f32> {
    match attrib {
        TerrainAttr::SlopeRiseRun => ta_slope_riserun(elevations, zscale),
        TerrainAttr::SlopePercentage => ta_slope_percentage(elevations, zscale),
        TerrainAttr::SlopeDegrees => ta_slope_degrees(elevations, zscale),
        TerrainAttr::SlopeRadians => ta_slope_radians(elevations, zscale),
        TerrainAttr::Aspect => ta_aspect(elevations, zscale),
        TerrainAttr::Curvature => ta_curvature(elevations, zscale),
        TerrainAttr::PlanformCurvature => ta_planform_curvature(elevations, zscale),
        TerrainAttr::ProfileCurvature => ta_profile_curvature(elevations, zscale),
    }
}

/// Rise/run slope (Horn, 1981).
pub fn ta_slope_riserun<T: DemNum>(elevations: &Array2D<T>, zscale: f32) -> Array2D<f32> {
    terrain_processor(elevations, zscale, cell_slope_riserun)
}

/// Slope in percent (`rise/run * 100`, Horn, 1981).
pub fn ta_slope_percentage<T: DemNum>(elevations: &Array2D<T>, zscale: f32) -> Array2D<f32> {
    terrain_processor(elevations, zscale, |e, x, y, z| {
        cell_slope_riserun(e, x, y, z) * 100.0
    })
}

/// Slope in degrees (Horn, 1981).
pub fn ta_slope_degrees<T: DemNum>(elevations: &Array2D<T>, zscale: f32) -> Array2D<f32> {
    terrain_processor(elevations, zscale, |e, x, y, z| {
        cell_slope_riserun(e, x, y, z).atan() * 180.0 / PI
    })
}

/// Slope in radians (Horn, 1981).
pub fn ta_slope_radians<T: DemNum>(elevations: &Array2D<T>, zscale: f32) -> Array2D<f32> {
    terrain_processor(elevations, zscale, |e, x, y, z| {
        cell_slope_riserun(e, x, y, z).atan()
    })
}

/// Aspect in degrees clockwise from north (Horn, 1981).
pub fn ta_aspect<T: DemNum>(elevations: &Array2D<T>, zscale: f32) -> Array2D<f32> {
    terrain_processor(elevations, zscale, cell_aspect)
}

/// Total curvature (Zevenbergen & Thorne, 1987).
pub fn ta_curvature<T: DemNum>(elevations: &Array2D<T>, zscale: f32) -> Array2D<f32> {
    terrain_processor(elevations, zscale, cell_curvature)
}

/// Planform curvature (Zevenbergen & Thorne, 1987).
pub fn ta_planform_curvature<T: DemNum>(elevations: &Array2D<T>, zscale: f32) -> Array2D<f32> {
    terrain_processor(elevations, zscale, cell_planform_curvature)
}

/// Profile curvature (Zevenbergen & Thorne, 1987).
pub fn ta_profile_curvature<T: DemNum>(elevations: &Array2D<T>, zscale: f32) -> Array2D<f32> {
    terrain_processor(elevations, zscale, cell_profile_curvature)
}

/// Stream Power Index (SPI): `ln((flow_accumulation / cell_area) * (riserun_slope + 0.001))`.
pub fn ta_spi<T: DemNum, U: DemNum>(
    flow_accumulation: &Array2D<T>,
    riserun_slope: &Array2D<U>,
) -> Result<Array2D<f64>> {
    if flow_accumulation.width() != riserun_slope.width()
        || flow_accumulation.height() != riserun_slope.height()
    {
        return Err(Error::DimensionMismatch(
            "Couldn't calculate SPI: input matrices have unequal dimensions!".into(),
        ));
    }
    let mut result = Array2D::<f64>::make_from_template(flow_accumulation, 0.0);
    result.set_no_data(-1.0);
    let cell_area = flow_accumulation.get_cell_area();

    for y in 0..flow_accumulation.height() {
        for x in 0..flow_accumulation.width() {
            if flow_accumulation.is_no_data(x, y) || riserun_slope.is_no_data(x, y) {
                result.set(x, y, -1.0);
            } else {
                let fa = flow_accumulation.get(x, y).to_f64();
                let sl = riserun_slope.get(x, y).to_f64();
                result.set(x, y, ((fa / cell_area) * (sl + 0.001)).ln());
            }
        }
    }
    Ok(result)
}

/// Compound Topographic Index (CTI / TWI): `ln((flow_accumulation / cell_area) / (riserun_slope + 0.001))`.
pub fn ta_cti<T: DemNum, U: DemNum>(
    flow_accumulation: &Array2D<T>,
    riserun_slope: &Array2D<U>,
) -> Result<Array2D<f64>> {
    if flow_accumulation.width() != riserun_slope.width()
        || flow_accumulation.height() != riserun_slope.height()
    {
        return Err(Error::DimensionMismatch(
            "Couldn't calculate CTI: input matrices have unequal dimensions!".into(),
        ));
    }
    let mut result = Array2D::<f64>::make_from_template(flow_accumulation, 0.0);
    result.set_no_data(-1.0);
    let cell_area = flow_accumulation.get_cell_area();

    for y in 0..flow_accumulation.height() {
        for x in 0..flow_accumulation.width() {
            if flow_accumulation.is_no_data(x, y) || riserun_slope.is_no_data(x, y) {
                result.set(x, y, -1.0);
            } else {
                let fa = flow_accumulation.get(x, y).to_f64();
                let sl = riserun_slope.get(x, y).to_f64();
                result.set(x, y, ((fa / cell_area) / (sl + 0.001)).ln());
            }
        }
    }
    Ok(result)
}
