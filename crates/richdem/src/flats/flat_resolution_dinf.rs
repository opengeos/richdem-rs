//! Couples Barnes (2014) flat resolution with Tarboton (1997) D-infinity flow directions.

use std::f64::consts::PI;

use crate::array2d::{Array2D, for_each_row_mut};
use crate::constants::{D8X, D8Y, DINF_NO_FLOW};
use crate::flats::flat_resolution::resolve_flats_barnes;
use crate::flowmet::dinf_flowdirs::{AC, AF, DX_E1, DX_E2, DY_E1, DY_E2, dinf_flow_directions};
use crate::num::DemNum;

const D8_TO_DINF: [f32; 9] = [
    -1.0,
    (4.0 * PI / 4.0) as f32,
    (3.0 * PI / 4.0) as f32,
    (2.0 * PI / 4.0) as f32,
    (1.0 * PI / 4.0) as f32,
    0.0,
    (7.0 * PI / 4.0) as f32,
    (6.0 * PI / 4.0) as f32,
    (5.0 * PI / 4.0) as f32,
];

fn dinf_masked_flow_dir(
    flat_resolution_mask: &Array2D<i32>,
    groups: &Array2D<i32>,
    x: usize,
    y: usize,
) -> f32 {
    let mut smax = 0.0f64;
    let mut nmax: isize = -1;
    let mut rmax = 0.0f64;
    let g = groups.get(x, y);
    let e0 = flat_resolution_mask.get(x, y) as f64;
    let dang = 1.0f64.atan2(1.0);

    for n in 0..8 {
        let x1 = x as i32 + DX_E1[n];
        let y1 = y as i32 + DY_E1[n];
        let x2 = x as i32 + DX_E2[n];
        let y2 = y as i32 + DY_E2[n];
        if groups.get_xy(x1, y1) != g || groups.get_xy(x2, y2) != g {
            continue;
        }
        let e1 = flat_resolution_mask.get_xy(x1, y1) as f64;
        let e2 = flat_resolution_mask.get_xy(x2, y2) as f64;
        let s1 = e0 - e1;
        let s2 = e1 - e2;
        let mut r = s2.atan2(s1);
        let mut s = (s1 * s1 + s2 * s2).sqrt();
        if r < 0.0 {
            r = 0.0;
            s = s1;
        } else if r > dang {
            r = dang;
            s = (e0 - e2) / std::f64::consts::SQRT_2;
        }
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
        let cfm = flat_resolution_mask.get(x, y);
        for n in 1..=8 {
            let nx = x as i32 + D8X[n];
            let ny = y as i32 + D8Y[n];
            if groups.get_xy(nx, ny) == g && flat_resolution_mask.get_xy(nx, ny) < cfm {
                return D8_TO_DINF[n];
            }
        }
        DINF_NO_FLOW
    }
}

/// Resolves flat cells in a D-infinity flow direction raster using `flat_resolution_mask` and `groups`.
pub fn dinf_flow_flats(
    flat_resolution_mask: &Array2D<i32>,
    groups: &Array2D<i32>,
    flowdirs: &mut Array2D<f32>,
) {
    let w = flat_resolution_mask.width();
    let h = flat_resolution_mask.height();
    if w < 3 || h < 3 {
        return;
    }

    for_each_row_mut(flowdirs.data_mut(), w, |y, row| {
        if y == 0 || y + 1 >= h {
            return;
        }
        for (x, dst) in row.iter_mut().enumerate().take(w - 1).skip(1) {
            if flat_resolution_mask.is_no_data(x, y) {
                continue;
            }
            if *dst == DINF_NO_FLOW {
                *dst = dinf_masked_flow_dir(flat_resolution_mask, groups, x, y);
            }
        }
    });
}

/// Computes D-infinity flow directions with Barnes (2014) flat resolution.
pub fn resolve_flats_barnes_dinf<T: DemNum>(elevations: &Array2D<T>) -> Array2D<f32> {
    let mut flowdirs = dinf_flow_directions(elevations);
    let (flat_mask, labels) = resolve_flats_barnes(elevations, &flowdirs);
    dinf_flow_flats(&flat_mask, &labels, &mut flowdirs);
    flowdirs
}
