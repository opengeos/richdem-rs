//! Tarboton (1997) D-infinity flow direction angles `[0, 2*pi)`.
//!
//! Reference: Tarboton, D.G., 1997. A new method for the determination of flow
//! directions and upslope areas in grid digital elevation models. Water Resources
//! Research 33, 309-319.

use std::f64::consts::{PI, SQRT_2};

use crate::array2d::Array2D;
use crate::constants::{DINF_NO_DATA, DINF_NO_FLOW};
use crate::num::DemNum;

/// Table 1 of Tarboton (1997): `e1` neighbor Y offsets.
pub const DY_E1: [i32; 8] = [0, -1, -1, 0, 0, 1, 1, 0];
/// Table 1 of Tarboton (1997): `e1` neighbor X offsets.
pub const DX_E1: [i32; 8] = [1, 0, 0, -1, -1, 0, 0, 1];
/// Table 1 of Tarboton (1997): `e2` neighbor Y offsets.
pub const DY_E2: [i32; 8] = [-1, -1, -1, -1, 1, 1, 1, 1];
/// Table 1 of Tarboton (1997): `e2` neighbor X offsets.
pub const DX_E2: [i32; 8] = [1, 1, -1, -1, -1, -1, 1, 1];
/// Table 1 of Tarboton (1997): facet angle offsets `ac`.
pub const AC: [f64; 8] = [0.0, 1.0, 1.0, 2.0, 2.0, 3.0, 3.0, 4.0];
/// Table 1 of Tarboton (1997): facet angle signs `af`.
pub const AF: [f64; 8] = [1.0, -1.0, 1.0, -1.0, 1.0, -1.0, 1.0, -1.0];

fn dinf_flow_dir<T: DemNum>(elevations: &Array2D<T>, x: usize, y: usize) -> f32 {
    let w = elevations.width();
    let h = elevations.height();
    if elevations.is_edge_cell(x, y) {
        return if x == 0 && y == 0 {
            (3.0 * PI / 4.0) as f32
        } else if x == 0 && y == h - 1 {
            (5.0 * PI / 4.0) as f32
        } else if x == w - 1 && y == 0 {
            (1.0 * PI / 4.0) as f32
        } else if x == w - 1 && y == h - 1 {
            (7.0 * PI / 4.0) as f32
        } else if x == 0 {
            (4.0 * PI / 4.0) as f32
        } else if x == w - 1 {
            0.0
        } else if y == 0 {
            (2.0 * PI / 4.0) as f32
        } else {
            (6.0 * PI / 4.0) as f32
        };
    }

    let mut nmax: isize = -1;
    let mut smax = 0.0f64;
    let mut rmax = 0.0f64;
    let e0 = elevations.get(x, y).to_f64();
    if !e0.is_finite() {
        return DINF_NO_DATA;
    }
    let dang = 1.0f64.atan2(1.0);

    for n in 0..8 {
        let x1 = x as i32 + DX_E1[n];
        let y1 = y as i32 + DY_E1[n];
        let x2 = x as i32 + DX_E2[n];
        let y2 = y as i32 + DY_E2[n];
        if elevations.is_no_data_xy(x1, y1) || elevations.is_no_data_xy(x2, y2) {
            continue;
        }
        let e1 = elevations.get_xy(x1, y1).to_f64();
        let e2 = elevations.get_xy(x2, y2).to_f64();
        if !e1.is_finite() || !e2.is_finite() {
            continue;
        }

        let s1 = e0 - e1;
        let s2 = e1 - e2;
        let mut r = s2.atan2(s1);
        let s = if r < 0.0 {
            r = 0.0;
            s1
        } else if r > dang {
            r = dang;
            (e0 - e2) / SQRT_2
        } else {
            (s1 * s1 + s2 * s2).sqrt()
        };

        if s > smax {
            smax = s;
            nmax = n as isize;
            rmax = r;
        }
    }

    if nmax != -1 {
        let ni = nmax as usize;
        (AF[ni] * rmax + AC[ni] * PI / 2.0) as f32
    } else {
        DINF_NO_FLOW
    }
}

/// Calculates Tarboton (1997) D-infinity flow direction angles `[0, 2*pi)`.
pub fn dinf_flow_directions<T: DemNum>(elevations: &Array2D<T>) -> Array2D<f32> {
    let w = elevations.width();
    let h = elevations.height();
    let mut flowdirs = Array2D::<f32>::make_from_template(elevations, DINF_NO_FLOW);
    flowdirs.set_no_data(DINF_NO_DATA);

    if w == 0 || h == 0 {
        return flowdirs;
    }

    let process_row = |y: usize, row: &mut [f32]| {
        for (x, dst) in row.iter_mut().enumerate().take(w) {
            *dst = if elevations.is_no_data(x, y) {
                DINF_NO_DATA
            } else {
                dinf_flow_dir(elevations, x, y)
            };
        }
    };

    #[cfg(feature = "parallel")]
    {
        use rayon::prelude::*;
        flowdirs
            .data_mut()
            .par_chunks_mut(w)
            .enumerate()
            .for_each(|(y, row)| process_row(y, row));
    }

    #[cfg(not(feature = "parallel"))]
    {
        for (y, row) in flowdirs.data_mut().chunks_mut(w).enumerate() {
            process_row(y, row);
        }
    }

    flowdirs
}
