//! D8 steepest-descent flow directions.

use std::cmp::Ordering;

use crate::array2d::Array2D;
use crate::constants::{D8Flowdir, D8X, D8Y, FLOWDIR_NO_DATA, NO_FLOW};
use crate::num::DemNum;

#[inline]
fn d8_flow_dir<T: DemNum>(elevations: &Array2D<T>, x: usize, y: usize) -> D8Flowdir {
    if elevations.is_edge_cell(x, y) {
        if elevations.is_top_left(x, y) {
            return 2;
        } else if elevations.is_bottom_left(x, y) {
            return 8;
        } else if elevations.is_top_right(x, y) {
            return 4;
        } else if elevations.is_bottom_right(x, y) {
            return 6;
        } else if elevations.is_left_col(x) {
            return 1;
        } else if elevations.is_right_col(x) {
            return 5;
        } else if elevations.is_top_row(y) {
            return 3;
        } else if elevations.is_bottom_row(y) {
            return 7;
        }
    }

    let mut min_elev = elevations.get(x, y);
    let mut flowdir: usize = NO_FLOW as usize;

    for n in 1..=8 {
        let nx = x as i32 + D8X[n];
        let ny = y as i32 + D8Y[n];
        if elevations.is_no_data_xy(nx, ny) {
            continue;
        }
        let ne = elevations.get_xy(nx, ny);
        if T::IS_FLOAT && !ne.to_f64().is_finite() {
            continue;
        }
        let cmp = ne.rd_cmp(min_elev);
        if cmp == Ordering::Less
            || (cmp == Ordering::Equal && flowdir > 0 && flowdir.is_multiple_of(2) && n % 2 == 1)
        {
            min_elev = ne;
            flowdir = n;
        }
    }

    flowdir as D8Flowdir
}

/// Calculates D8 steepest-descent flow directions (`0..=8`, [`FLOWDIR_NO_DATA`] for NoData).
pub fn d8_flow_directions<T: DemNum>(elevations: &Array2D<T>) -> Array2D<D8Flowdir> {
    let w = elevations.width();
    let h = elevations.height();
    let mut flowdirs = Array2D::<D8Flowdir>::make_from_template(elevations, NO_FLOW as u8);
    flowdirs.set_no_data(FLOWDIR_NO_DATA);

    for y in 0..h {
        for x in 0..w {
            if elevations.is_no_data(x, y) {
                flowdirs.set(x, y, FLOWDIR_NO_DATA);
            } else {
                flowdirs.set(x, y, d8_flow_dir(elevations, x, y));
            }
        }
    }

    flowdirs
}
