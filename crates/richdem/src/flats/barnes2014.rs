//! Barnes, Lehman, and Mulla (2014) flat resolution without requiring precomputed flow directions.
//!
//! Reference: Barnes, R., Lehman, C., Mulla, D., 2014a. An efficient assignment of drainage
//! direction over flat surfaces in raster digital elevation models. Computers & Geosciences
//! 62, 128-135. doi:10.1016/j.cageo.2013.01.009

use std::cmp::Ordering;
use std::collections::VecDeque;

use crate::array2d::{Array2D, for_each_row_mut, map_range_ordered};
use crate::array3d::{Array3D, FLOW_PROPS_DEPTH};
use crate::constants::{D8X, D8Y, HAS_FLOW_GEN, NO_FLOW_GEN};
use crate::error::{Error, Result};
use crate::flats::find_flats::{IS_A_FLAT, NOT_A_FLAT, find_flats};
use crate::grid_cell::GridCell;
use crate::num::DemNum;

fn build_away_gradient(
    flats: &Array2D<i8>,
    flat_mask: &mut Array2D<i32>,
    high_edges: &mut VecDeque<GridCell>,
    flat_height: &mut [i32],
    labels: &Array2D<i32>,
) {
    let mut loops = 1i32;
    let marker = GridCell::new(-1, -1);
    high_edges.push_back(marker);

    while high_edges.len() != 1 {
        let c = high_edges.pop_front().unwrap();
        if c.x == -1 {
            loops += 1;
            high_edges.push_back(marker);
            continue;
        }
        if flat_mask.get_xy(c.x, c.y) > 0 {
            continue;
        }

        let lbl = labels.get_xy(c.x, c.y);
        flat_mask.set_xy(c.x, c.y, loops);
        flat_height[lbl as usize] = loops;

        for n in 1..=8 {
            let nx = c.x + D8X[n];
            let ny = c.y + D8Y[n];
            if labels.in_grid(nx, ny)
                && labels.get_xy(nx, ny) == lbl
                && flats.get_xy(nx, ny) == IS_A_FLAT
            {
                high_edges.push_back(GridCell::new(nx, ny));
            }
        }
    }
}

fn build_towards_combined_gradient(
    flats: &Array2D<i8>,
    flat_mask: &mut Array2D<i32>,
    low_edges: &mut VecDeque<GridCell>,
    flat_height: &[i32],
    labels: &Array2D<i32>,
) {
    for v in flat_mask.data_mut().iter_mut() {
        *v = -*v;
    }

    let mut loops = 1i32;
    let marker = GridCell::new(-1, -1);
    low_edges.push_back(marker);

    while low_edges.len() != 1 {
        let c = low_edges.pop_front().unwrap();
        if c.x == -1 {
            loops += 1;
            low_edges.push_back(marker);
            continue;
        }
        let fm = flat_mask.get_xy(c.x, c.y);
        if fm > 0 {
            continue;
        }

        let lbl = labels.get_xy(c.x, c.y);
        if fm != 0 {
            flat_mask.set_xy(c.x, c.y, (flat_height[lbl as usize] + fm) + 2 * loops);
        } else {
            flat_mask.set_xy(c.x, c.y, 2 * loops);
        }

        for n in 1..=8 {
            let nx = c.x + D8X[n];
            let ny = c.y + D8Y[n];
            if labels.in_grid(nx, ny)
                && labels.get_xy(nx, ny) == lbl
                && flats.get_xy(nx, ny) == IS_A_FLAT
            {
                low_edges.push_back(GridCell::new(nx, ny));
            }
        }
    }
}

fn label_flat<T: DemNum>(
    x0: i32,
    y0: i32,
    label_id: i32,
    labels: &mut Array2D<i32>,
    elevations: &Array2D<T>,
) {
    let mut to_fill = VecDeque::new();
    to_fill.push_back(GridCell::new(x0, y0));
    let target_elevation = elevations.get_xy(x0, y0);

    while let Some(c) = to_fill.pop_front() {
        if !elevations.get_xy(c.x, c.y).rd_eq(target_elevation) {
            continue;
        }
        if labels.get_xy(c.x, c.y) > 0 {
            continue;
        }
        labels.set_xy(c.x, c.y, label_id);
        for n in 1..=8 {
            let nx = c.x + D8X[n];
            let ny = c.y + D8Y[n];
            if labels.in_grid(nx, ny) {
                to_fill.push_back(GridCell::new(nx, ny));
            }
        }
    }
}

fn find_flat_edges<T: DemNum>(
    low_edges: &mut VecDeque<GridCell>,
    high_edges: &mut VecDeque<GridCell>,
    flats: &Array2D<i8>,
    elevations: &Array2D<T>,
) {
    let w = flats.width();
    let h = flats.height();

    let scan_row = |y: usize| -> (Vec<GridCell>, Vec<GridCell>) {
        let mut row_low = Vec::new();
        let mut row_high = Vec::new();
        for x in 0..w {
            if flats.is_no_data(x, y) {
                continue;
            }
            let cf = flats.get(x, y);
            let ce = elevations.get(x, y);
            for n in 1..=8 {
                let nx = x as i32 + D8X[n];
                let ny = y as i32 + D8Y[n];
                if !flats.in_grid(nx, ny) {
                    continue;
                }
                let nf = flats.get_xy(nx, ny);
                let ne = elevations.get_xy(nx, ny);
                if cf == NOT_A_FLAT && nf == IS_A_FLAT && ne.rd_eq(ce) {
                    row_low.push(GridCell::new(x as i32, y as i32));
                    break;
                } else if cf == IS_A_FLAT && ce.rd_cmp(ne) == Ordering::Less {
                    row_high.push(GridCell::new(x as i32, y as i32));
                    break;
                }
            }
        }
        (row_low, row_high)
    };

    for (row_low, row_high) in map_range_ordered(h, scan_row) {
        low_edges.extend(row_low);
        high_edges.extend(row_high);
    }
}

/// Computes the Barnes (2014) flat resolution mask and flat labels directly from `elevations`.
///
/// Returns `(flat_mask, labels)`.
pub fn get_flat_mask<T: DemNum>(elevations: &Array2D<T>) -> (Array2D<i32>, Array2D<i32>) {
    let flats = find_flats(elevations);
    let mut labels = Array2D::<i32>::make_from_template(elevations, 0);
    let mut flat_mask = Array2D::<i32>::make_from_template(elevations, 0);
    flat_mask.set_no_data(-1);

    let mut low_edges = VecDeque::new();
    let mut high_edges = VecDeque::new();
    find_flat_edges(&mut low_edges, &mut high_edges, &flats, elevations);

    if low_edges.is_empty() {
        return (flat_mask, labels);
    }

    let mut group_number = 1i32;
    for c in &low_edges {
        if labels.get_xy(c.x, c.y) == 0 {
            label_flat(c.x, c.y, group_number, &mut labels, elevations);
            group_number += 1;
        }
    }

    high_edges.retain(|c| labels.get_xy(c.x, c.y) != 0);

    let mut flat_height = vec![0i32; group_number as usize];
    build_away_gradient(
        &flats,
        &mut flat_mask,
        &mut high_edges,
        &mut flat_height,
        &labels,
    );
    build_towards_combined_gradient(
        &flats,
        &mut flat_mask,
        &mut low_edges,
        &flat_height,
        &labels,
    );

    (flat_mask, labels)
}

/// Modifies `elevations` in place using minimal floating-point increments (`next_up`)
/// proportional to `flat_mask` so all drainable flats acquire a convergent downhill gradient.
///
/// Returns the number of cells whose raised elevation reached or exceeded an originally higher
/// non-flat neighbor (a warning indicator that a wider float type may be needed).
pub fn resolve_flats_epsilon_barnes2014<T: DemNum>(
    flat_mask: &Array2D<i32>,
    labels: &Array2D<i32>,
    elevations: &mut Array2D<T>,
) -> Result<usize> {
    if !T::IS_FLOAT {
        return Err(Error::UnsupportedDataType(
            "resolve_flats_epsilon_barnes2014 requires a floating-point DEM".into(),
        ));
    }

    let w = flat_mask.width();
    let h = flat_mask.height();
    let mut raise_warn = 0usize;

    for y in 1..h.saturating_sub(1) {
        for x in 1..w.saturating_sub(1) {
            let lbl = labels.get(x, y);
            if lbl == 0 {
                continue;
            }
            let orig = elevations.get(x, y);
            let mut lower = [false; 9];
            for n in 1..=8 {
                let nx = x as i32 + D8X[n];
                let ny = y as i32 + D8Y[n];
                lower[n] = orig.rd_cmp(elevations.get_xy(nx, ny)) == Ordering::Less;
            }

            let mut cur = orig;
            let steps = flat_mask.get(x, y).max(0);
            for _ in 0..steps {
                cur = cur.next_up_float().unwrap();
            }
            elevations.set(x, y, cur);

            for n in 1..=8 {
                let nx = x as i32 + D8X[n];
                let ny = y as i32 + D8Y[n];
                if labels.get_xy(nx, ny) == lbl {
                    continue;
                }
                if cur.rd_cmp(elevations.get_xy(nx, ny)) == Ordering::Less {
                    continue;
                }
                if lower[n] {
                    raise_warn += 1;
                }
            }
        }
    }

    Ok(raise_warn)
}

/// Resolves flat cells in a generic flow-proportion `Array3D<f32>` using `flat_mask` and `labels`.
pub fn resolve_flats_flowdirs_barnes2014(
    flat_mask: &Array2D<i32>,
    labels: &Array2D<i32>,
    flowdirs: &mut Array3D<f32>,
) {
    let w = flat_mask.width();
    let h = flat_mask.height();
    assert_eq!(
        flowdirs.width(),
        w,
        "flowdirs width ({}) must match flat_mask width ({w})",
        flowdirs.width()
    );
    assert_eq!(
        flowdirs.height(),
        h,
        "flowdirs height ({}) must match flat_mask height ({h})",
        flowdirs.height()
    );
    if w < 3 || h < 3 {
        return;
    }

    for_each_row_mut(flowdirs.data_mut(), w * FLOW_PROPS_DEPTH, |y, row| {
        if y == 0 || y + 1 >= h {
            return;
        }
        for x in 1..w - 1 {
            if flat_mask.is_no_data(x, y) {
                continue;
            }
            let cell = &mut row[x * FLOW_PROPS_DEPTH..(x + 1) * FLOW_PROPS_DEPTH];
            if cell[0] != NO_FLOW_GEN {
                continue;
            }
            let lbl = labels.get(x, y);
            let mut min_elev = flat_mask.get(x, y);
            let mut flowdir: usize = 0;

            for n in 1..=8 {
                let nx = x as i32 + D8X[n];
                let ny = y as i32 + D8Y[n];
                if labels.get_xy(nx, ny) != lbl {
                    continue;
                }
                let fm = flat_mask.get_xy(nx, ny);
                if fm < min_elev
                    || (fm == min_elev && flowdir > 0 && flowdir.is_multiple_of(2) && n % 2 == 1)
                {
                    min_elev = fm;
                    flowdir = n;
                }
            }

            if flowdir > 0 {
                cell[0] = HAS_FLOW_GEN;
                cell[flowdir] = 1.0;
            }
        }
    });
}
