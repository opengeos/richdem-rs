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

    let mut deps = vec![0i8; w * h];
    let wi = w as isize;
    let n_offsets: [isize; 9] = [0, -1, -1 - wi, -wi, 1 - wi, 1, 1 + wi, wi, -1 + wi];
    let p_data = props.data();
    let p_nd = props.no_data();

    for y in 1..h.saturating_sub(1) {
        let row_base = y * w;
        for x in 1..w.saturating_sub(1) {
            let ci = row_base + x;
            let p_base = ci * 9;
            if p_data[p_base].rd_eq(p_nd) {
                continue;
            }
            let p_cell = &p_data[p_base..p_base + 9];
            for n in 1..=8 {
                if p_cell[n] > 0.0 {
                    let ni = ci.wrapping_add_signed(n_offsets[n]);
                    deps[ni] += 1;
                }
            }
        }
    }

    let mut q = VecDeque::<(u32, u32)>::new();
    for y in 0..h {
        let row_base = y * w;
        for x in 0..w {
            let ci = row_base + x;
            if deps[ci] == 0 && !p_data[ci * 9].rd_eq(p_nd) {
                q.push_back((x as u32, y as u32));
            }
        }
    }

    let accum_data = accum.data_mut();
    let wi32 = w as i32;
    let hi32 = h as i32;

    while let Some((cx, cy)) = q.pop_front() {
        let ci = (cy as usize) * w + (cx as usize);
        let p_base = ci * 9;
        debug_assert!(!p_data[p_base].rd_eq(p_nd));
        let c_accum = accum_data[ci].to_f64();
        let p_cell = &p_data[p_base..p_base + 9];
        let interior = cx > 0 && (cx as usize) + 1 < w && cy > 0 && (cy as usize) + 1 < h;

        for n in 1..=8 {
            let p = p_cell[n];
            if p <= 0.0 {
                continue;
            }
            let ni = if interior {
                ci.wrapping_add_signed(n_offsets[n])
            } else {
                let nx = cx as i32 + D8X[n];
                let ny = cy as i32 + D8Y[n];
                if nx < 0 || ny < 0 || nx >= wi32 || ny >= hi32 {
                    continue;
                }
                (ny as usize) * w + (nx as usize)
            };
            if p_data[ni * 9].rd_eq(p_nd) {
                continue;
            }
            let next_val = accum_data[ni].to_f64() + (p as f64) * c_accum;
            accum_data[ni] = A::from_f64(next_val);
            let rem = deps[ni] - 1;
            deps[ni] = rem;
            debug_assert!(rem >= 0);
            if rem == 0 {
                let nx = (cx as i32 + D8X[n]) as u32;
                let ny = (cy as i32 + D8Y[n]) as u32;
                q.push_back((nx, ny));
            }
        }
    }

    let nd = accum.no_data();
    let accum_data = accum.data_mut();
    for (ci, slot) in accum_data.iter_mut().enumerate() {
        if p_data[ci * 9].rd_eq(p_nd) {
            *slot = nd;
        }
    }

    Ok(())
}
