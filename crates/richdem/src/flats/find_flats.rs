//! Identification of flat cells (cells with no local downhill gradient under D8).

use crate::array2d::Array2D;
use crate::constants::{D8X, D8Y};
use crate::num::DemNum;

/// Indicator value for NoData cells in flat masks.
pub const FLAT_NO_DATA: i8 = -1;
/// Indicator value for cells that have a local downhill gradient.
pub const NOT_A_FLAT: i8 = 0;
/// Indicator value for cells that belong to a flat (no strictly lower D8 neighbor).
pub const IS_A_FLAT: i8 = 1;

/// Identifies flat cells under D8 connectivity.
///
/// Output values: [`IS_A_FLAT`] (`1`), [`NOT_A_FLAT`] (`0`), or [`FLAT_NO_DATA`] (`-1`).
pub fn find_flats<T: DemNum>(elevations: &Array2D<T>) -> Array2D<i8> {
    let w = elevations.width();
    let h = elevations.height();
    let mut flats = Array2D::<i8>::make_from_template(elevations, NOT_A_FLAT);
    flats.set_no_data(FLAT_NO_DATA);

    for y in 0..h {
        for x in 0..w {
            if elevations.is_no_data(x, y) {
                flats.set(x, y, FLAT_NO_DATA);
                continue;
            }
            if elevations.is_edge_cell(x, y) {
                flats.set(x, y, NOT_A_FLAT);
                continue;
            }

            let ce = elevations.get(x, y);
            let mut is_flat = true;
            for n in 1..=8 {
                let nx = x as i32 + D8X[n];
                let ny = y as i32 + D8Y[n];
                if elevations.get_xy(nx, ny).rd_cmp(ce) == std::cmp::Ordering::Less
                    || elevations.is_no_data_xy(nx, ny)
                {
                    is_flat = false;
                    break;
                }
            }
            flats.set(x, y, if is_flat { IS_A_FLAT } else { NOT_A_FLAT });
        }
    }

    flats
}
