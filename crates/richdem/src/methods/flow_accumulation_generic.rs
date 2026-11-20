//! Generic flow accumulation over 3D flow proportions (`Array3D<f32>`).

use std::collections::VecDeque;

use crate::array2d::Array2D;
use crate::array3d::Array3D;
use crate::constants::{ACCUM_NO_DATA, D8X, D8Y};
use crate::error::{Error, Result};
use crate::num::DemNum;

/// Calculates flow accumulation from a 3D flow-proportion raster `props`.
///
/// `accum` must be pre-initialized with per-cell flow weights (typically `1.0`)
/// and have the same dimensions as `props`. On completion, `accum` holds each
/// cell's total upstream + local flow, with [`ACCUM_NO_DATA`] (`-1`) at NoData cells.
pub fn flow_accumulation<A: DemNum>(props: &Array3D<f32>, accum: &mut Array2D<A>) -> Result<()> {
    let w = props.width();
    let h = props.height();
    if accum.width() != w || accum.height() != h {
        return Err(Error::DimensionMismatch(
            "Accumulation array must have same dimensions as proportions array!".into(),
        ));
    }

    accum.set_no_data(A::from_i64(ACCUM_NO_DATA as i64));

    let mut deps = Array2D::<i8>::new(w, h, 0);
    for y in 1..h.saturating_sub(1) {
        for x in 1..w.saturating_sub(1) {
            let ci = accum.xy_to_i(x, y);
            if props.is_no_data_i(ci) {
                continue;
            }
            for n in 1..=8 {
                if props.get(x, y, n) > 0.0
                    && let Some(ni) = accum.n_to_i(ci, D8X[n], D8Y[n])
                {
                    let d = deps.get_i(ni) + 1;
                    deps.set_i(ni, d);
                }
            }
        }
    }

    let mut q = VecDeque::<usize>::new();
    for i in 0..deps.size() {
        if deps.get_i(i) == 0 && !props.is_no_data_i(i) {
            q.push_back(i);
        }
    }

    while let Some(ci) = q.pop_front() {
        debug_assert!(!props.is_no_data_i(ci));
        let c_accum = accum.get_i(ci).to_f64();

        for n in 1..=8 {
            let p = props.get_in(ci, n);
            if p <= 0.0 {
                continue;
            }
            let Some(ni) = accum.n_to_i(ci, D8X[n], D8Y[n]) else {
                continue;
            };
            if props.is_no_data_i(ni) {
                continue;
            }
            let next_val = accum.get_i(ni).to_f64() + (p as f64) * c_accum;
            accum.set_i(ni, A::from_f64(next_val));
            let rem = deps.get_i(ni) - 1;
            deps.set_i(ni, rem);
            debug_assert!(rem >= 0);
            if rem == 0 {
                q.push_back(ni);
            }
        }
    }

    let nd = accum.no_data();
    for i in 0..props.size() {
        if props.is_no_data_i(i) {
            accum.set_i(i, nd);
        }
    }

    Ok(())
}
