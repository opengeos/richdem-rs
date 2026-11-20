//! Tarboton (1997) D-infinity upslope area calculation.

use std::collections::VecDeque;
use std::f64::consts::PI;

use crate::array2d::Array2D;
use crate::constants::{DINF_NO_DATA, DINF_NO_FLOW};
use crate::grid_cell::GridCell;
use crate::num::DemNum;

const DINF_DX: [i32; 9] = [1, 1, 0, -1, -1, -1, 0, 1, 1];
const DINF_DY: [i32; 9] = [0, -1, -1, -1, 0, 1, 1, 1, 0];

fn where_do_i_flow(flowdir: f64) -> (usize, Option<usize>) {
    let scaled = flowdir / (PI / 4.0);
    if (scaled - scaled.trunc()).abs() < 1e-6 {
        let nhigh = (scaled.round() as usize).min(8);
        (nhigh, None)
    } else {
        let nlow = (scaled.floor() as usize).min(7);
        (nlow + 1, Some(nlow))
    }
}

fn area_proportion(flowdir: f64, nhigh: usize, nlow: Option<usize>) -> (f64, f64) {
    if nlow.is_none() {
        (1.0, 0.0)
    } else {
        let plow = ((nhigh as f64) * (PI / 4.0) - flowdir) / (PI / 4.0);
        let phigh = 1.0 - plow;
        (phigh, plow)
    }
}

/// Calculates each cell's D-infinity upslope contributing area from a D-infinity flow
/// direction raster (produced by [`crate::flowmet::dinf_flow_directions`]).
///
/// Output type `U` should be a floating-point type (`f32` or `f64`) because D-infinity
/// proportions fractional areas across neighbor facets and uses `-1.0` (`DINF_NO_DATA`)
/// as its NoData sentinel.
pub fn dinf_upslope_area<T: DemNum, U: DemNum>(flowdirs: &Array2D<T>) -> Array2D<U> {
    let w = flowdirs.width();
    let h = flowdirs.height();

    let mut dependency = Array2D::<i8>::new(w, h, 0);
    let mut area = Array2D::<U>::make_from_template(flowdirs, U::ZERO);
    area.set_no_data(U::from_nodata_f64(DINF_NO_DATA as f64));
    let area_nd = area.no_data();

    for y in 0..h {
        for x in 0..w {
            if flowdirs.is_no_data(x, y) {
                area.set(x, y, area_nd);
                dependency.set(x, y, 9);
                continue;
            }
            let fd = flowdirs.get(x, y).to_f64();
            if fd == DINF_NO_FLOW as f64 || fd < 0.0 {
                continue;
            }
            let (n_high, n_low) = where_do_i_flow(fd);
            let nhx = x as i32 + DINF_DX[n_high];
            let nhy = y as i32 + DINF_DY[n_high];
            if let Some(nl) = n_low {
                let nlx = x as i32 + DINF_DX[nl];
                let nly = y as i32 + DINF_DY[nl];
                if flowdirs.in_grid(nlx, nly) && !flowdirs.is_no_data_xy(nlx, nly) {
                    let cur = dependency.get_xy(nlx, nly);
                    dependency.set_xy(nlx, nly, cur + 1);
                }
            }
            if flowdirs.in_grid(nhx, nhy) && !flowdirs.is_no_data_xy(nhx, nhy) {
                let cur = dependency.get_xy(nhx, nhy);
                dependency.set_xy(nhx, nhy, cur + 1);
            }
        }
    }

    let mut sources = VecDeque::<GridCell>::new();
    for y in 0..h {
        for x in 0..w {
            if flowdirs.is_no_data(x, y) {
                continue;
            }
            if dependency.get(x, y) == 0 {
                sources.push_back(GridCell::new(x as i32, y as i32));
            }
        }
    }

    while let Some(c) = sources.pop_front() {
        if flowdirs.is_no_data_xy(c.x, c.y) {
            continue;
        }
        let cur_area = area.get_xy(c.x, c.y).to_f64() + 1.0;
        area.set_xy(c.x, c.y, U::from_f64(cur_area));

        let fd = flowdirs.get_xy(c.x, c.y).to_f64();
        if fd == DINF_NO_FLOW as f64 || fd < 0.0 {
            continue;
        }

        let (n_high, n_low) = where_do_i_flow(fd);
        let (phigh, plow) = area_proportion(fd, n_high, n_low);

        let nhx = c.x + DINF_DX[n_high];
        let nhy = c.y + DINF_DY[n_high];
        if flowdirs.in_grid(nhx, nhy) && !flowdirs.is_no_data_xy(nhx, nhy) {
            let next = area.get_xy(nhx, nhy).to_f64() + cur_area * phigh;
            area.set_xy(nhx, nhy, U::from_f64(next));
        }

        if let Some(nl) = n_low {
            let nlx = c.x + DINF_DX[nl];
            let nly = c.y + DINF_DY[nl];
            if flowdirs.in_grid(nlx, nly) && !flowdirs.is_no_data_xy(nlx, nly) {
                let next = area.get_xy(nlx, nly).to_f64() + cur_area * plow;
                area.set_xy(nlx, nly, U::from_f64(next));
                let rem = dependency.get_xy(nlx, nly) - 1;
                dependency.set_xy(nlx, nly, rem);
                if rem == 0 {
                    sources.push_back(GridCell::new(nlx, nly));
                }
            }
        }

        if flowdirs.in_grid(nhx, nhy) && !flowdirs.is_no_data_xy(nhx, nhy) {
            let rem = dependency.get_xy(nhx, nhy) - 1;
            dependency.set_xy(nhx, nhy, rem);
            if rem == 0 {
                sources.push_back(GridCell::new(nhx, nhy));
            }
        }
    }

    area
}
