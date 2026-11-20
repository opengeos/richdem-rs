//! Garbrecht and Martz (1997) flat resolution algorithm.
//!
//! Reference: Garbrecht, J., Martz, L.W., 1997. The assignment of drainage direction
//! over flat surfaces in raster digital elevation models. Journal of Hydrology 193, 204-213.

use std::cmp::Ordering;
use std::collections::VecDeque;

use crate::array2d::Array2D;
use crate::constants::{D8Flowdir, D8X, D8Y, NO_FLOW};
use crate::error::{Error, Result};
use crate::flowmet::d8_flowdirs::d8_flow_directions;
use crate::grid_cell::GridCell;
use crate::num::DemNum;

fn collect_flats(flowdirs: &Array2D<D8Flowdir>) -> Vec<GridCell> {
    let mut flats = Vec::new();
    for x in 0..flowdirs.width() {
        for y in 0..flowdirs.height() {
            if flowdirs.get(x, y) == NO_FLOW as u8 && !flowdirs.is_no_data(x, y) {
                flats.push(GridCell::new(x as i32, y as i32));
            }
        }
    }
    flats
}

/// Computes the Garbrecht-Martz gradient towards lower terrain via O(N) multi-source BFS
/// seeded from unresolved flat cells adjacent to equal-or-lower flowing outlets.
/// Undrainable flat components (with no outlet) are never visited and remain at `0`.
fn gradient_towards_lower<T: DemNum>(
    elevations: &Array2D<T>,
    flowdirs: &Array2D<D8Flowdir>,
    flats: &[GridCell],
) -> (Array2D<i32>, Vec<GridCell>) {
    let w = elevations.width();
    let h = elevations.height();
    let mut inc1 = Array2D::<i32>::make_from_template(elevations, 0);
    let mut visited = Array2D::<u8>::new(w, h, 0);
    let mut queue = VecDeque::<GridCell>::new();
    let mut drainable_flats = Vec::new();

    for c in flats {
        let ce = elevations.get_xy(c.x, c.y);
        let mut touches_outlet = false;
        for n in 1..=8 {
            let nx = c.x + D8X[n];
            let ny = c.y + D8Y[n];
            if !elevations.in_grid(nx, ny) {
                continue;
            }
            let ne = elevations.get_xy(nx, ny);
            let nfd = flowdirs.get_xy(nx, ny);
            if ne.rd_cmp(ce) != Ordering::Greater
                && nfd != NO_FLOW as u8
                && !flowdirs.is_no_data_xy(nx, ny)
            {
                touches_outlet = true;
                break;
            }
        }
        if touches_outlet {
            visited.set_xy(c.x, c.y, 1);
            // Positive initial increment so recomputed D8 directions flow into the equal-elevation outlet.
            inc1.set_xy(c.x, c.y, 1);
            queue.push_back(*c);
            drainable_flats.push(*c);
        }
    }

    while let Some(c) = queue.pop_front() {
        let ce = elevations.get_xy(c.x, c.y);
        let dist = inc1.get_xy(c.x, c.y);
        for n in 1..=8 {
            let nx = c.x + D8X[n];
            let ny = c.y + D8Y[n];
            if !elevations.in_grid(nx, ny) || visited.get_xy(nx, ny) != 0 {
                continue;
            }
            if flowdirs.get_xy(nx, ny) == NO_FLOW as u8
                && !flowdirs.is_no_data_xy(nx, ny)
                && elevations.get_xy(nx, ny).rd_eq(ce)
            {
                visited.set_xy(nx, ny, 1);
                inc1.set_xy(nx, ny, dist + 1);
                let nc = GridCell::new(nx, ny);
                queue.push_back(nc);
                drainable_flats.push(nc);
            }
        }
    }

    (inc1, drainable_flats)
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
///
/// Requires a floating-point DEM; returns [`Error::UnsupportedDataType`] for integer DEMs.
pub fn garbrecht_alg<T: DemNum>(
    elevations: &mut Array2D<T>,
    flowdirs: &mut Array2D<D8Flowdir>,
    epsilon: f64,
) -> Result<()> {
    if !T::IS_FLOAT {
        return Err(Error::UnsupportedDataType(
            "garbrecht_alg requires a floating-point DEM to apply fractional epsilon increments"
                .into(),
        ));
    }
    let flats = collect_flats(flowdirs);
    if flats.is_empty() {
        return Ok(());
    }
    let (inc1, drainable_flats) = gradient_towards_lower(elevations, flowdirs, &flats);
    if drainable_flats.is_empty() {
        return Ok(());
    }
    let inc2 = gradient_away_from_higher(elevations, &drainable_flats);

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
    Ok(())
}
