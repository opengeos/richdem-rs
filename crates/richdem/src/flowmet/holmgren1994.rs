//! Holmgren (1994) multiple-flow-direction proportions (MFD / MD8).
//!
//! Reference: Holmgren, P., 1994. Multiple flow direction algorithms for runoff
//! modelling in grid based elevation models: an empirical evaluation.
//! Hydrological Processes 8, 327-334.

use std::cmp::Ordering;

use crate::array2d::Array2D;
use crate::array3d::Array3D;
use crate::constants::{D8X, D8Y, DR, HAS_FLOW_GEN, NO_DATA_GEN, NO_FLOW_GEN};
use crate::num::DemNum;

/// Computes Holmgren (1994) flow proportions with exponent `xparam`.
pub fn fm_holmgren<T: DemNum>(elevations: &Array2D<T>, xparam: f64) -> Array3D<f32> {
    const L1: f64 = 0.5;
    const L2: f64 = 0.354;
    const L: [f64; 9] = [0.0, L1, L2, L1, L2, L1, L2, L1, L2];

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
                    let val = (grad * L[n]).powf(xparam) as f32;
                    props.set(x, y, n, val);
                    c_sum += val as f64;
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
