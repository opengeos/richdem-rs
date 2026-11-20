//! Holmgren (1994) multiple-flow-direction proportions (MFD / MD8).
//!
//! Reference: Holmgren, P., 1994. Multiple flow direction algorithms for runoff
//! modelling in grid based elevation models: an empirical evaluation.
//! Hydrological Processes 8, 327-334.

use std::cmp::Ordering;

use crate::array2d::Array2D;
use crate::array3d::{Array3D, FLOW_PROPS_DEPTH};
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
    if w == 0 || h == 0 {
        return props;
    }

    let row_stride = w * FLOW_PROPS_DEPTH;
    let process_row = |y: usize, row: &mut [f32]| {
        for x in 0..w {
            let cell = &mut row[x * FLOW_PROPS_DEPTH..(x + 1) * FLOW_PROPS_DEPTH];
            if elevations.is_no_data(x, y) {
                cell[0] = NO_DATA_GEN;
                continue;
            }
            if elevations.is_edge_cell(x, y) {
                continue;
            }

            let e = elevations.get(x, y);
            let mut scaled = [0.0f64; 9];
            let mut max_term = 0.0f64;

            for n in 1..=8 {
                let nx = x as i32 + D8X[n];
                let ny = y as i32 + D8Y[n];
                if !elevations.in_grid(nx, ny) || elevations.is_no_data_xy(nx, ny) {
                    continue;
                }
                let ne = elevations.get_xy(nx, ny);
                if ne.rd_cmp(e) == Ordering::Less {
                    let rise = e.to_f64() - ne.to_f64();
                    let term = (rise / DR[n]) * L[n];
                    if term > 0.0 && term.is_finite() {
                        scaled[n] = term;
                        if term > max_term {
                            max_term = term;
                        }
                    }
                }
            }

            if max_term > 0.0 {
                let mut weights = [0.0f64; 9];
                let mut c_sum = 0.0f64;
                for n in 1..=8 {
                    if scaled[n] > 0.0 {
                        // Rescale by max_term so large exponents cannot overflow to inf/inf = NaN.
                        let val = (scaled[n] / max_term).powf(xparam);
                        weights[n] = val;
                        c_sum += val;
                    }
                }
                if c_sum > 0.0 {
                    cell[0] = HAS_FLOW_GEN;
                    for (n, &wval) in weights.iter().enumerate().skip(1) {
                        cell[n] = if wval > 0.0 {
                            (wval / c_sum) as f32
                        } else {
                            0.0
                        };
                    }
                }
            }
        }
    };

    #[cfg(feature = "parallel")]
    {
        use rayon::prelude::*;
        props
            .data_mut()
            .par_chunks_mut(row_stride)
            .enumerate()
            .for_each(|(y, row)| process_row(y, row));
    }

    #[cfg(not(feature = "parallel"))]
    {
        for (y, row) in props.data_mut().chunks_mut(row_stride).enumerate() {
            process_row(y, row);
        }
    }

    props
}
