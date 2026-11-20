//! Garbrecht and Martz (1997) flat resolution algorithm.
//!
//! Reference: Garbrecht, J., Martz, L.W., 1997. The assignment of drainage direction
//! over flat surfaces in raster digital elevation models. Journal of Hydrology 193, 204-213.

use std::cmp::Ordering;

use crate::array2d::Array2D;
use crate::constants::{D8Flowdir, D8X, D8Y, NO_FLOW};
use crate::flowmet::d8_flowdirs::d8_flow_directions;
use crate::grid_cell::GridCell;
use crate::num::DemNum;

fn collect_flats(flowdirs: &Array2D<D8Flowdir>) -> Vec<GridCell> {
    let mut flats = Vec::new();
    for x in 0..flowdirs.width() {
        for y in 0..flowdirs.height() {
            if flowdirs.get(x, y) == NO_FLOW as u8 {
                flats.push(GridCell::new(x as i32, y as i32));
            }
        }
    }
    flats
}

fn gradient_towards_lower<T: DemNum>(
    elevations: &Array2D<T>,
    flowdirs: &Array2D<D8Flowdir>,
    flats: &[GridCell],
) -> Array2D<i32> {
    let mut inc1 = Array2D::<i32>::make_from_template(elevations, 0);
    let mut loops = 0i32;
    let max_iters = flats.len().saturating_add(1);

    for _ in 0..max_iters {
        let mut number_incremented = 0usize;
        for c in flats {
            let mut increment = true;
            let ce = elevations.get_xy(c.x, c.y);
            for n in 1..=8 {
                let nx = c.x + D8X[n];
                let ny = c.y + D8Y[n];
                if !elevations.in_grid(nx, ny) {
                    continue;
                }
                let ne = elevations.get_xy(nx, ny);
                let nfd = flowdirs.get_xy(nx, ny);
                if (ne.rd_cmp(ce) == Ordering::Less
                    && nfd != NO_FLOW as u8
                    && !flowdirs.is_no_data_xy(nx, ny))
                    || (inc1.get_xy(nx, ny) < loops && ne.rd_eq(ce))
                {
                    increment = false;
                    break;
                }
            }
            if increment {
                let cur = inc1.get_xy(c.x, c.y);
                inc1.set_xy(c.x, c.y, cur + 1);
                number_incremented += 1;
            }
        }
        if number_incremented == 0 {
            break;
        }
        loops += 1;
    }
    inc1
}

fn gradient_away_from_higher<T: DemNum>(
    elevations: &Array2D<T>,
    flats: &[GridCell],
) -> Array2D<i32> {
    let mut inc2 = Array2D::<i32>::make_from_template(elevations, 0);
    let mut number_incremented = 0usize;
    let max_iters = flats.len().saturating_add(1);

    for _ in 0..max_iters {
        if number_incremented >= flats.len() {
            break;
        }
        for c in flats {
            let cur = inc2.get_xy(c.x, c.y);
            if cur > 0 {
                inc2.set_xy(c.x, c.y, cur + 1);
            }
        }
        let mut changed = 0usize;
        for c in flats {
            if inc2.get_xy(c.x, c.y) > 0 {
                continue;
            }
            let ce = elevations.get_xy(c.x, c.y);
            let mut has_higher = false;
            let mut has_lower = false;
            for n in 1..=8 {
                let nx = c.x + D8X[n];
                let ny = c.y + D8Y[n];
                if !elevations.in_grid(nx, ny) {
                    continue;
                }
                let ne = elevations.get_xy(nx, ny);
                if !has_higher && (ne.rd_cmp(ce) == Ordering::Greater || inc2.get_xy(nx, ny) == 2) {
                    has_higher = true;
                } else if !has_lower && ne.rd_cmp(ce) == Ordering::Less {
                    has_lower = true;
                }
            }
            if has_higher && !has_lower {
                let cur = inc2.get_xy(c.x, c.y);
                inc2.set_xy(c.x, c.y, cur + 1);
                number_incremented += 1;
                changed += 1;
            }
        }
        if changed == 0 {
            break;
        }
    }
    inc2
}

/// Resolves flat surfaces in `elevations` using Garbrecht and Martz (1997) with
/// increment `epsilon` (default `0.001` in RichDEM) and updates `flowdirs`.
pub fn garbrecht_alg<T: DemNum>(
    elevations: &mut Array2D<T>,
    flowdirs: &mut Array2D<D8Flowdir>,
    epsilon: f64,
) {
    let flats = collect_flats(flowdirs);
    if flats.is_empty() {
        return;
    }
    let inc1 = gradient_towards_lower(elevations, flowdirs, &flats);
    let inc2 = gradient_away_from_higher(elevations, &flats);

    for x in 0..elevations.width() {
        for y in 0..elevations.height() {
            if elevations.is_no_data(x, y) {
                continue;
            }
            let delta = (inc1.get(x, y) + inc2.get(x, y)) as f64 * epsilon;
            if delta != 0.0 {
                let v = elevations.get(x, y).to_f64() + delta;
                elevations.set(x, y, T::from_f64(v));
            }
        }
    }

    *flowdirs = d8_flow_directions(elevations);
}
