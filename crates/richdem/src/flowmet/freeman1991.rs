//! Freeman (1991) multiple-flow-direction proportions.
//!
//! Reference: Freeman, T.G., 1991. Calculating catchment area with divergent flow
//! based on a regular grid. Computers & Geosciences 17, 413-422.

use std::cmp::Ordering;

use crate::array2d::Array2D;
use crate::array3d::Array3D;
use crate::constants::{D8X, D8Y, DR, HAS_FLOW_GEN, NO_DATA_GEN, NO_FLOW_GEN};
use crate::num::DemNum;

/// Computes Freeman (1991) flow proportions with exponent `xparam`.
pub fn fm_freeman<T: DemNum>(elevations: &Array2D<T>, xparam: f64) -> Array3D<f32> {
    let w = elevations.width();
    let h = elevations.height();
    let mut props = Array3D::<f32>::make_from_template(elevations, NO_FLOW_GEN);
    props.set_no_data(NO_DATA_GEN);

    for y in 0..h {
        for x in 0..w {
            if elevations.is_no_data(x, y) {
                props.set(x, y, 0, NO_DATA_GEN);
                continue;
            }
            if elevations.is_edge_cell(x, y) {
                continue;
            }

            let e = elevations.get(x, y);
            let mut c_sum = 0.0f64;

            for n in 1..=8 {
                let nx = x as i32 + D8X[n];
                let ny = y as i32 + D8Y[n];
                if !elevations.in_grid(nx, ny) || elevations.is_no_data_xy(nx, ny) {
                    continue;
                }
                let ne = elevations.get_xy(nx, ny);
                if ne.rd_cmp(e) == Ordering::Less {
                    let rise = e.to_f64() - ne.to_f64();
                    let grad = rise / DR[n];
                    let cval = grad.powf(xparam);
                    props.set(x, y, n, cval as f32);
                    c_sum += cval;
                }
            }

            if c_sum > 0.0 {
                props.set(x, y, 0, HAS_FLOW_GEN);
                let inv_c = 1.0 / c_sum;
                for n in 1..=8 {
                    let p = props.get(x, y, n);
                    if p > 0.0 {
                        props.set(x, y, n, (p as f64 * inv_c) as f32);
                    } else {
                        props.set(x, y, n, 0.0);
                    }
                }
            }
        }
    }

    props
}
