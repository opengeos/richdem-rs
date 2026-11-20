//! D8 flow accumulation and upslope contributing area tracing.

use std::collections::VecDeque;

use crate::array2d::Array2D;
use crate::constants::{ACCUM_NO_DATA, D8_INVERSE, D8X, D8Y, FLOWDIR_NO_DATA, NO_FLOW};
use crate::grid_cell::GridCell;
use crate::num::DemNum;

/// Calculates full-grid D8 flow accumulation from a D8 flow-direction raster
/// (matching RichDEM's `d8_flow_accum`).
pub fn d8_flow_accum<T: DemNum, U: DemNum>(flowdirs: &Array2D<T>) -> Array2D<U> {
    let w = flowdirs.width();
    let h = flowdirs.height();

    let mut dependency = Array2D::<i8>::new(w, h, 0);
    let mut area = Array2D::<U>::make_from_template(flowdirs, U::ZERO);
    area.set_no_data(U::from_i64(ACCUM_NO_DATA as i64));
    let area_nd = area.no_data();

    for y in 0..h {
        for x in 0..w {
            if flowdirs.is_no_data(x, y) {
                area.set(x, y, area_nd);
                continue;
            }
            let n = flowdirs.get(x, y).to_i64();
            if n == NO_FLOW as i64 || !(1..=8).contains(&n) {
                continue;
            }
            let nu = n as usize;
            let nx = x as i32 + D8X[nu];
            let ny = y as i32 + D8Y[nu];
            if !flowdirs.in_grid(nx, ny) {
                continue;
            }
            let cur = dependency.get_xy(nx, ny);
            dependency.set_xy(nx, ny, cur + 1);
        }
    }

    let mut sources = VecDeque::<GridCell>::new();
    for y in 0..h {
        for x in 0..w {
            if dependency.get(x, y) == 0 && !flowdirs.is_no_data(x, y) {
                sources.push_back(GridCell::new(x as i32, y as i32));
            }
        }
    }

    while let Some(c) = sources.pop_front() {
        let cur_area = area.get_xy(c.x, c.y) + U::ONE;
        area.set_xy(c.x, c.y, cur_area);

        let n = flowdirs.get_xy(c.x, c.y).to_i64();
        if n == NO_FLOW as i64 || !(1..=8).contains(&n) {
            continue;
        }
        let nu = n as usize;
        let nx = c.x + D8X[nu];
        let ny = c.y + D8Y[nu];
        if !flowdirs.in_grid(nx, ny) || flowdirs.is_no_data_xy(nx, ny) {
            continue;
        }
        let next_area = area.get_xy(nx, ny) + cur_area;
        area.set_xy(nx, ny, next_area);
        let rem = dependency.get_xy(nx, ny) - 1;
        dependency.set_xy(nx, ny, rem);
        if rem == 0 {
            sources.push_back(GridCell::new(nx, ny));
        }
    }

    area
}

/// Identifies all cells whose D8 flow passes through the Bresenham line from `(x0, y0)` to `(x1, y1)`.
///
/// Output values: `1` = upslope cell, `2` = cell on initializing line, [`FLOWDIR_NO_DATA`] = other.
pub fn d8_upslope_cells<T: DemNum>(
    x0: i32,
    y0: i32,
    x1: i32,
    y1: i32,
    flowdirs: &Array2D<T>,
) -> Array2D<u8> {
    let mut upslope = Array2D::<u8>::make_from_template(flowdirs, FLOWDIR_NO_DATA);
    upslope.set_no_data(FLOWDIR_NO_DATA);
    let mut expansion = VecDeque::<GridCell>::new();

    // Integer Bresenham line algorithm handling shallow, steep, horizontal, and vertical segments.
    let dx = (x1 - x0).abs();
    let dy = -(y1 - y0).abs();
    let sx = if x0 < x1 { 1 } else { -1 };
    let sy = if y0 < y1 { 1 } else { -1 };
    let mut err = dx + dy;
    let mut x = x0;
    let mut y = y0;

    loop {
        if flowdirs.in_grid(x, y) && upslope.get_xy(x, y) != 2 {
            expansion.push_back(GridCell::new(x, y));
            upslope.set_xy(x, y, 2);
        }
        if x == x1 && y == y1 {
            break;
        }
        let e2 = 2 * err;
        if e2 >= dy {
            err += dy;
            x += sx;
        }
        if e2 <= dx {
            err += dx;
            y += sy;
        }
    }

    while let Some(c) = expansion.pop_front() {
        for n in 1..=8 {
            let nx = c.x + D8X[n];
            let ny = c.y + D8Y[n];
            if !flowdirs.in_grid(nx, ny) || flowdirs.is_no_data_xy(nx, ny) {
                continue;
            }
            let nfd = flowdirs.get_xy(nx, ny).to_i64();
            if nfd == NO_FLOW as i64 || !(1..=8).contains(&nfd) {
                continue;
            }
            if upslope.get_xy(nx, ny) == FLOWDIR_NO_DATA && n == D8_INVERSE[nfd as usize] as usize {
                expansion.push_back(GridCell::new(nx, ny));
                upslope.set_xy(nx, ny, 1);
            }
        }
    }

    upslope
}
