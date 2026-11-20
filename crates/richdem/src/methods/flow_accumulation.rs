//! Flow accumulation convenience wrappers for all flow metrics and D8 rasters.

use std::collections::VecDeque;

use crate::array2d::Array2D;
use crate::array3d::Array3D;
use crate::constants::{ACCUM_NO_DATA, D8X, D8Y, NO_FLOW, Topology};
use crate::error::{Error, Result};
use crate::flowmet::{
    fm_d4, fm_d8, fm_dinfinity, fm_fairfield_leymarie, fm_freeman, fm_holmgren, fm_ocallaghan,
    fm_quinn, fm_rho4, fm_rho8, fm_tarboton,
};
use crate::methods::flow_accumulation_generic::flow_accumulation;
use crate::num::DemNum;

/// Named flow routing/accumulation methods matching RichDEM's Python & CLI API.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FlowAccumMethod {
    /// Tarboton (1997) D-infinity.
    Tarboton,
    /// Alias for Tarboton.
    Dinf,
    /// Quinn (1991) multiple flow direction (`Holmgren` with `exponent = 1`).
    Quinn,
    /// Holmgren (1994) multiple flow direction (requires `exponent`).
    Holmgren,
    /// Freeman (1991) multiple flow direction (requires `exponent`).
    Freeman,
    /// Fairfield & Leymarie (1991) Rho8.
    FairfieldLeymarieD8,
    /// Fairfield & Leymarie (1991) Rho4.
    FairfieldLeymarieD4,
    /// Alias for `FairfieldLeymarieD8`.
    Rho8,
    /// Alias for `FairfieldLeymarieD4`.
    Rho4,
    /// O'Callaghan & Mark (1984) D8.
    OCallaghanD8,
    /// O'Callaghan & Mark (1984) D4.
    OCallaghanD4,
    /// Alias for `OCallaghanD8`.
    D8,
    /// Alias for `OCallaghanD4`.
    D4,
}

impl FlowAccumMethod {
    /// Parses a RichDEM method name string (case-sensitive or case-insensitive fallback).
    pub fn parse(s: &str) -> Result<Self> {
        match s {
            "Tarboton" | "tarboton" => Ok(Self::Tarboton),
            "Dinf" | "dinf" | "Dinfinity" | "dinfinity" => Ok(Self::Dinf),
            "Quinn" | "quinn" => Ok(Self::Quinn),
            "Holmgren" | "holmgren" => Ok(Self::Holmgren),
            "Freeman" | "freeman" => Ok(Self::Freeman),
            "FairfieldLeymarieD8" | "FairfieldLeymarie" | "fairfieldleymaried8" => {
                Ok(Self::FairfieldLeymarieD8)
            }
            "FairfieldLeymarieD4" | "fairfieldleymaried4" => Ok(Self::FairfieldLeymarieD4),
            "Rho8" | "rho8" => Ok(Self::Rho8),
            "Rho4" | "rho4" => Ok(Self::Rho4),
            "OCallaghanD8" | "OCallaghan" | "ocallaghand8" => Ok(Self::OCallaghanD8),
            "OCallaghanD4" | "ocallaghand4" => Ok(Self::OCallaghanD4),
            "D8" | "d8" => Ok(Self::D8),
            "D4" | "d4" => Ok(Self::D4),
            other => Err(Error::InvalidArgument(format!(
                "Invalid FlowAccumulation method '{other}'. Valid methods are: Tarboton, Dinf, Quinn, Holmgren, Freeman, FairfieldLeymarieD8, FairfieldLeymarieD4, Rho8, Rho4, OCallaghanD8, OCallaghanD4, D8, D4"
            ))),
        }
    }
}

/// Computes flow proportions (`Array3D<f32>`) by method name and optional exponent.
pub fn flow_proportions_by_method<T: DemNum>(
    elevations: &Array2D<T>,
    method: FlowAccumMethod,
    exponent: Option<f64>,
) -> Result<Array3D<f32>> {
    match method {
        FlowAccumMethod::Tarboton | FlowAccumMethod::Dinf => Ok(fm_tarboton(elevations)),
        FlowAccumMethod::Quinn => Ok(fm_quinn(elevations)),
        FlowAccumMethod::Holmgren => {
            let e = exponent.ok_or_else(|| {
                Error::InvalidArgument("Flow method 'Holmgren' requires an exponent!".into())
            })?;
            Ok(fm_holmgren(elevations, e))
        }
        FlowAccumMethod::Freeman => {
            let e = exponent.ok_or_else(|| {
                Error::InvalidArgument("Flow method 'Freeman' requires an exponent!".into())
            })?;
            Ok(fm_freeman(elevations, e))
        }
        FlowAccumMethod::FairfieldLeymarieD8 | FlowAccumMethod::Rho8 => Ok(fm_rho8(elevations)),
        FlowAccumMethod::FairfieldLeymarieD4 | FlowAccumMethod::Rho4 => Ok(fm_rho4(elevations)),
        FlowAccumMethod::OCallaghanD8 | FlowAccumMethod::D8 => Ok(fm_d8(elevations)),
        FlowAccumMethod::OCallaghanD4 | FlowAccumMethod::D4 => Ok(fm_d4(elevations)),
    }
}

/// Computes flow accumulation into `accum` by method name and optional exponent.
pub fn flow_accumulation_by_method<T: DemNum, A: DemNum>(
    elevations: &Array2D<T>,
    accum: &mut Array2D<A>,
    method: FlowAccumMethod,
    exponent: Option<f64>,
) -> Result<()> {
    let props = flow_proportions_by_method(elevations, method, exponent)?;
    flow_accumulation(&props, accum)
}

/// Tarboton (D-infinity) flow accumulation.
pub fn fa_tarboton<T: DemNum, A: DemNum>(
    elevations: &Array2D<T>,
    accum: &mut Array2D<A>,
) -> Result<()> {
    let props = fm_tarboton(elevations);
    flow_accumulation(&props, accum)
}

/// Alias for [`fa_tarboton`].
pub fn fa_dinfinity<T: DemNum, A: DemNum>(
    elevations: &Array2D<T>,
    accum: &mut Array2D<A>,
) -> Result<()> {
    let props = fm_dinfinity(elevations);
    flow_accumulation(&props, accum)
}

/// Holmgren (1994) flow accumulation with exponent `xparam`.
pub fn fa_holmgren<T: DemNum, A: DemNum>(
    elevations: &Array2D<T>,
    accum: &mut Array2D<A>,
    xparam: f64,
) -> Result<()> {
    let props = fm_holmgren(elevations, xparam);
    flow_accumulation(&props, accum)
}

/// Quinn (1991) flow accumulation.
pub fn fa_quinn<T: DemNum, A: DemNum>(
    elevations: &Array2D<T>,
    accum: &mut Array2D<A>,
) -> Result<()> {
    let props = fm_quinn(elevations);
    flow_accumulation(&props, accum)
}

/// Freeman (1991) flow accumulation with exponent `xparam`.
pub fn fa_freeman<T: DemNum, A: DemNum>(
    elevations: &Array2D<T>,
    accum: &mut Array2D<A>,
    xparam: f64,
) -> Result<()> {
    let props = fm_freeman(elevations, xparam);
    flow_accumulation(&props, accum)
}

/// Fairfield & Leymarie (1991) Rho8 flow accumulation.
pub fn fa_fairfield_leymarie_d8<T: DemNum, A: DemNum>(
    elevations: &Array2D<T>,
    accum: &mut Array2D<A>,
) -> Result<()> {
    let props = fm_fairfield_leymarie(elevations, Topology::D8);
    flow_accumulation(&props, accum)
}

/// Fairfield & Leymarie (1991) Rho4 flow accumulation.
pub fn fa_fairfield_leymarie_d4<T: DemNum, A: DemNum>(
    elevations: &Array2D<T>,
    accum: &mut Array2D<A>,
) -> Result<()> {
    let props = fm_fairfield_leymarie(elevations, Topology::D4);
    flow_accumulation(&props, accum)
}

/// Rho8 flow accumulation.
pub fn fa_rho8<T: DemNum, A: DemNum>(
    elevations: &Array2D<T>,
    accum: &mut Array2D<A>,
) -> Result<()> {
    let props = fm_rho8(elevations);
    flow_accumulation(&props, accum)
}

/// Rho4 flow accumulation.
pub fn fa_rho4<T: DemNum, A: DemNum>(
    elevations: &Array2D<T>,
    accum: &mut Array2D<A>,
) -> Result<()> {
    let props = fm_rho4(elevations);
    flow_accumulation(&props, accum)
}

/// O'Callaghan & Mark (1984) D8 flow accumulation.
pub fn fa_ocallaghan_d8<T: DemNum, A: DemNum>(
    elevations: &Array2D<T>,
    accum: &mut Array2D<A>,
) -> Result<()> {
    let props = fm_ocallaghan(elevations, Topology::D8);
    flow_accumulation(&props, accum)
}

/// O'Callaghan & Mark (1984) D4 flow accumulation.
pub fn fa_ocallaghan_d4<T: DemNum, A: DemNum>(
    elevations: &Array2D<T>,
    accum: &mut Array2D<A>,
) -> Result<()> {
    let props = fm_ocallaghan(elevations, Topology::D4);
    flow_accumulation(&props, accum)
}

/// D8 flow accumulation.
pub fn fa_d8<T: DemNum, A: DemNum>(elevations: &Array2D<T>, accum: &mut Array2D<A>) -> Result<()> {
    let props = fm_d8(elevations);
    flow_accumulation(&props, accum)
}

/// D4 flow accumulation.
pub fn fa_d4<T: DemNum, A: DemNum>(elevations: &Array2D<T>, accum: &mut Array2D<A>) -> Result<()> {
    let props = fm_d4(elevations);
    flow_accumulation(&props, accum)
}

/// Calculates interior-cell D8 flow accumulation directly from a D8 flow-direction raster,
/// matching upstream RichDEM's `flow_accumulation_from_d8`.
///
/// By upstream convention, only interior cells (`1..width-1`, `1..height-1`) are seeded and
/// accumulated; edge-ring cells remain `0`. Use [`d8_flow_accum`] for full-grid accumulation
/// that includes boundary cells.
pub fn flow_accumulation_from_d8<T: DemNum>(d8_in: &Array2D<T>) -> Array2D<u32> {
    let w = d8_in.width();
    let h = d8_in.height();
    let mut accum = Array2D::<u32>::make_from_template(d8_in, 0);
    accum.set_no_data(ACCUM_NO_DATA as u32);

    let mut deps = Array2D::<i8>::new(w, h, 0);
    for y in 1..h.saturating_sub(1) {
        for x in 1..w.saturating_sub(1) {
            if d8_in.is_no_data(x, y) {
                continue;
            }
            let n = d8_in.get(x, y).to_i64();
            if n == NO_FLOW as i64 || !(1..=8).contains(&n) {
                continue;
            }
            let nu = n as usize;
            let nx = x as i32 + D8X[nu];
            let ny = y as i32 + D8Y[nu];
            if !d8_in.in_grid(nx, ny) {
                continue;
            }
            let cur = deps.get_xy(nx, ny);
            deps.set_xy(nx, ny, cur + 1);
        }
    }

    let mut q = VecDeque::<usize>::new();
    for y in 1..h.saturating_sub(1) {
        for x in 1..w.saturating_sub(1) {
            if deps.get(x, y) == 0 && !d8_in.is_no_data(x, y) {
                q.push_back(d8_in.xy_to_i(x, y));
            }
        }
    }

    while let Some(ci) = q.pop_front() {
        debug_assert!(!d8_in.is_no_data_i(ci));
        let c_accum = accum.get_i(ci) + 1;
        accum.set_i(ci, c_accum);

        let n = d8_in.get_i(ci).to_i64();
        if n == NO_FLOW as i64 || !(1..=8).contains(&n) {
            continue;
        }
        let nu = n as usize;
        let (cx, cy) = d8_in.i_to_xy(ci);
        let nx = cx as i32 + D8X[nu];
        let ny = cy as i32 + D8Y[nu];
        if !d8_in.in_grid(nx, ny) {
            continue;
        }
        let (nxu, nyu) = (nx as usize, ny as usize);
        let ni = d8_in.xy_to_i(nxu, nyu);
        if d8_in.is_edge_cell(nxu, nyu) || d8_in.is_no_data_i(ni) {
            continue;
        }
        let next = accum.get_i(ni) + c_accum;
        accum.set_i(ni, next);
        let rem = deps.get_i(ni) - 1;
        deps.set_i(ni, rem);
        debug_assert!(rem >= 0);
        if rem == 0 {
            q.push_back(ni);
        }
    }

    let nd = accum.no_data();
    for i in 0..d8_in.size() {
        if d8_in.is_no_data_i(i) {
            accum.set_i(i, nd);
        }
    }

    accum
}
