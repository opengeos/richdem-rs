//! Freeman (1991) multiple-flow-direction proportions.
//!
//! Reference: Freeman, T.G., 1991. Calculating catchment area with divergent flow
//! based on a regular grid. Computers & Geosciences 17, 413-422.

use std::cmp::Ordering;

use crate::array2d::Array2D;
use crate::array3d::{Array3D, FLOW_PROPS_DEPTH};
use crate::constants::{D8X, D8Y, DR, HAS_FLOW_GEN, NO_DATA_GEN, NO_FLOW_GEN};
use crate::num::DemNum;

/// Computes Freeman (1991) flow proportions with exponent `xparam`.
pub fn fm_freeman<T: DemNum>(elevations: &Array2D<T>, xparam: f64) -> Array3D<f32> {
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
            let mut grads = [0.0f64; 9];
            let mut max_grad = 0.0f64;

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
                    if grad > 0.0 && grad.is_finite() {
                        grads[n] = grad;
                        if grad > max_grad {
                            max_grad = grad;
                        }
                    }
                }
            }

            if max_grad > 0.0 {
                let mut weights = [0.0f64; 9];
                let mut c_sum = 0.0f64;
                for n in 1..=8 {
                    if grads[n] > 0.0 {
                        // Rescale by max_grad so large exponents cannot overflow to inf/inf = NaN.
                        let cval = (grads[n] / max_grad).powf(xparam);
                        weights[n] = cval;
                        c_sum += cval;
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
