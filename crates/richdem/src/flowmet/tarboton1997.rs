//! Tarboton (1997) D-infinity flow proportions.
//!
//! Reference: Tarboton, D.G., 1997. A new method for the determination of flow
//! directions and upslope areas in grid digital elevation models. Water Resources
//! Research 33, 309-319.

use std::f64::consts::{PI, SQRT_2};

use crate::array2d::Array2D;
use crate::array3d::Array3D;
use crate::constants::{HAS_FLOW_GEN, NO_DATA_GEN, NO_FLOW_GEN};
use crate::num::DemNum;

#[inline]
fn nwrap(n: usize) -> usize {
    if n == 9 { 1 } else { n }
}

/// Computes Tarboton (1997) D-infinity flow proportions into `Array3D<f32>`.
pub fn fm_tarboton<T: DemNum>(elevations: &Array2D<T>) -> Array3D<f32> {
    let w = elevations.width();
    let h = elevations.height();
    let mut props = Array3D::<f32>::make_from_template(elevations, NO_FLOW_GEN);
    props.set_no_data(NO_DATA_GEN);

    let dang: f64 = 1.0f64.atan2(1.0);
    let dang_f32 = dang as f32;

    // Remapped Table 1 of Tarboton (1997) for RichDEM's D8 neighborhood
    const DY_E1: [i32; 9] = [0, 0, -1, -1, 0, 0, 1, 1, 0];
    const DX_E1: [i32; 9] = [0, -1, 0, 0, 1, 1, 0, 0, -1];
    const DY_E2: [i32; 9] = [0, -1, -1, -1, -1, 1, 1, 1, 1];
    const DX_E2: [i32; 9] = [0, -1, -1, 1, 1, 1, 1, -1, -1];
    const AF: [f64; 9] = [0.0, -1.0, 1.0, -1.0, 1.0, -1.0, 1.0, -1.0, 1.0];

    for y in 0..h {
        for x in 0..w {
            if elevations.is_no_data(x, y) {
                props.set(x, y, 0, NO_DATA_GEN);
                continue;
            }
            if elevations.is_edge_cell(x, y) {
                continue;
            }

            let mut nmax: usize = 0;
            let mut smax = 0.0f64;
            let mut rmax = 0.0f32;
            let e0 = elevations.get(x, y).to_f64();

            for n in 1..=8 {
                let x1 = x as i32 + DX_E1[n];
                let y1 = y as i32 + DY_E1[n];
                let x2 = x as i32 + DX_E2[n];
                let y2 = y as i32 + DY_E2[n];

                if !elevations.in_grid(x1, y1)
                    || elevations.is_no_data_xy(x1, y1)
                    || !elevations.in_grid(x2, y2)
                    || elevations.is_no_data_xy(x2, y2)
                {
                    continue;
                }

                let e1 = elevations.get_xy(x1, y1).to_f64();
                let e2 = elevations.get_xy(x2, y2).to_f64();

                let s1 = e0 - e1;
                let s2 = e1 - e2;
                let mut r = s2.atan2(s1);
                let s = if r < 1e-7 {
                    r = 0.0;
                    s1
                } else if r > dang - 1e-7 {
                    r = dang;
                    (e0 - e2) / SQRT_2
                } else {
                    (s1 * s1 + s2 * s2).sqrt()
                };

                if s > smax {
                    smax = s;
                    nmax = n;
                    rmax = r as f32;
                }
            }

            if nmax == 0 {
                continue;
            }

            props.set(x, y, 0, HAS_FLOW_GEN);

            if AF[nmax] == 1.0 && rmax == 0.0 {
                rmax = dang_f32;
            } else if AF[nmax] == 1.0 && rmax == dang_f32 {
                rmax = 0.0;
            } else if AF[nmax] == 1.0 {
                rmax = (PI / 4.0) as f32 - rmax;
            }

            if rmax == 0.0 {
                props.set(x, y, nmax, 1.0);
            } else if rmax == dang_f32 {
                props.set(x, y, nwrap(nmax + 1), 1.0);
            } else {
                let p = (rmax as f64 / (PI / 4.0)) as f32;
                props.set(x, y, nmax, p);
                props.set(x, y, nwrap(nmax + 1), 1.0 - p);
            }
        }
    }

    props
}

/// Alias for [`fm_tarboton`].
pub fn fm_dinfinity<T: DemNum>(elevations: &Array2D<T>) -> Array3D<f32> {
    fm_tarboton(elevations)
}
