//! Barnes, Lehman, and Mulla (2014) flat resolution coupled with D8 flow directions.

use std::cmp::Ordering;
use std::collections::VecDeque;

use crate::array2d::Array2D;
use crate::constants::{D8Flowdir, D8X, D8Y, DINF_NO_FLOW, NO_FLOW};
use crate::error::{Error, Result};
use crate::flowmet::d8_flowdirs::d8_flow_directions;
use crate::grid_cell::GridCell;
use crate::num::DemNum;

#[inline]
fn no_flow_sentinel<U: DemNum>() -> U {
    if U::IS_FLOAT {
        U::from_f64(DINF_NO_FLOW as f64)
    } else {
        U::from_i64(NO_FLOW as i64)
    }
}

fn d8_masked_flow_dir(flat_mask: &Array2D<i32>, labels: &Array2D<i32>, x: usize, y: usize) -> u8 {
    let mut min_elev = flat_mask.get(x, y);
    let mut flowdir: usize = NO_FLOW as usize;
    let lbl = labels.get(x, y);

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
    flowdir as u8
}

/// Assigns D8 flow directions within flat regions using `flat_mask` and `labels`.
pub fn d8_flow_flats<U: DemNum>(
    flat_mask: &Array2D<i32>,
    labels: &Array2D<i32>,
    flowdirs: &mut Array2D<U>,
) {
    let w = flat_mask.width();
    let h = flat_mask.height();
    let no_flow = no_flow_sentinel::<U>();

    for y in 1..h.saturating_sub(1) {
        for x in 1..w.saturating_sub(1) {
            if flat_mask.is_no_data(x, y) {
                continue;
            }
            if flowdirs.get(x, y).rd_eq(no_flow) {
                let d = d8_masked_flow_dir(flat_mask, labels, x, y);
                flowdirs.set(x, y, U::from_i64(d as i64));
            }
        }
    }
}

fn build_away_gradient_fd<U: DemNum>(
    flowdirs: &Array2D<U>,
    flat_mask: &mut Array2D<i32>,
    mut edges: VecDeque<GridCell>,
    flat_height: &mut [i32],
    labels: &Array2D<i32>,
) {
    let no_flow = no_flow_sentinel::<U>();
    let mut loops = 1i32;
    let marker = GridCell::new(-1, -1);
    edges.push_back(marker);

    while edges.len() != 1 {
        let c = edges.pop_front().unwrap();
        if c.x == -1 {
            loops += 1;
            edges.push_back(marker);
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
                && flowdirs.get_xy(nx, ny).rd_eq(no_flow)
            {
                edges.push_back(GridCell::new(nx, ny));
            }
        }
    }
}

fn build_towards_combined_gradient_fd<U: DemNum>(
    flowdirs: &Array2D<U>,
    flat_mask: &mut Array2D<i32>,
    mut edges: VecDeque<GridCell>,
    flat_height: &[i32],
    labels: &Array2D<i32>,
) {
    let no_flow = no_flow_sentinel::<U>();
    for v in flat_mask.data_mut().iter_mut() {
        *v = -*v;
    }

    let mut loops = 1i32;
    let marker = GridCell::new(-1, -1);
    edges.push_back(marker);

    while edges.len() != 1 {
        let c = edges.pop_front().unwrap();
        if c.x == -1 {
            loops += 1;
            edges.push_back(marker);
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
                && flowdirs.get_xy(nx, ny).rd_eq(no_flow)
            {
                edges.push_back(GridCell::new(nx, ny));
            }
        }
    }
}

fn label_this<T: DemNum>(
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

fn find_flat_edges_fd<T: DemNum, U: DemNum>(
    low_edges: &mut VecDeque<GridCell>,
    high_edges: &mut VecDeque<GridCell>,
    flowdirs: &Array2D<U>,
    elevations: &Array2D<T>,
) {
    let no_flow = no_flow_sentinel::<U>();
    for x in 0..flowdirs.width() {
        for y in 0..flowdirs.height() {
            if flowdirs.is_no_data(x, y) {
                continue;
            }
            let fd = flowdirs.get(x, y);
            let ce = elevations.get(x, y);
            for n in 1..=8 {
                let nx = x as i32 + D8X[n];
                let ny = y as i32 + D8Y[n];
                if !flowdirs.in_grid(nx, ny) || flowdirs.is_no_data_xy(nx, ny) {
                    continue;
                }
                let nfd = flowdirs.get_xy(nx, ny);
                let ne = elevations.get_xy(nx, ny);
                if !fd.rd_eq(no_flow) && nfd.rd_eq(no_flow) && ne.rd_eq(ce) {
                    low_edges.push_back(GridCell::new(x as i32, y as i32));
                    break;
                } else if fd.rd_eq(no_flow) && ce.rd_cmp(ne) == Ordering::Less {
                    high_edges.push_back(GridCell::new(x as i32, y as i32));
                    break;
                }
            }
        }
    }
}

/// Computes the Barnes (2014) flat mask and flat labels from `elevations` and `flowdirs`.
pub fn resolve_flats_barnes<T: DemNum, U: DemNum>(
    elevations: &Array2D<T>,
    flowdirs: &Array2D<U>,
) -> (Array2D<i32>, Array2D<i32>) {
    let mut labels = Array2D::<i32>::make_from_template(elevations, 0);
    let mut flat_mask = Array2D::<i32>::make_from_template(elevations, 0);
    flat_mask.set_no_data(-1);

    let mut low_edges = VecDeque::new();
    let mut high_edges = VecDeque::new();
    find_flat_edges_fd(&mut low_edges, &mut high_edges, flowdirs, elevations);

    if low_edges.is_empty() {
        return (flat_mask, labels);
    }

    let mut group_number = 1i32;
    for c in &low_edges {
        if labels.get_xy(c.x, c.y) == 0 {
            label_this(c.x, c.y, group_number, &mut labels, elevations);
            group_number += 1;
        }
    }

    high_edges.retain(|c| labels.get_xy(c.x, c.y) != 0);

    let mut flat_height = vec![0i32; group_number as usize];
    build_away_gradient_fd(
        flowdirs,
        &mut flat_mask,
        high_edges,
        &mut flat_height,
        &labels,
    );
    build_towards_combined_gradient_fd(flowdirs, &mut flat_mask, low_edges, &flat_height, &labels);

    (flat_mask, labels)
}

/// Alters floating-point elevations in place using `flat_mask` and `labels`.
pub fn d8_flats_alter_dem<T: DemNum>(
    flat_mask: &Array2D<i32>,
    labels: &Array2D<i32>,
    elevations: &mut Array2D<T>,
) -> Result<()> {
    if !T::IS_FLOAT {
        return Err(Error::UnsupportedDataType(
            "d8_flats_alter_dem requires a floating-point DEM".into(),
        ));
    }
    let w = flat_mask.width();
    let h = flat_mask.height();
    for x in 1..w.saturating_sub(1) {
        for y in 1..h.saturating_sub(1) {
            if labels.get(x, y) == 0 {
                continue;
            }
            let mut cur = elevations.get(x, y);
            let steps = flat_mask.get(x, y).max(0);
            for _ in 0..steps {
                cur = cur.next_up_float().unwrap();
            }
            elevations.set(x, y, cur);
        }
    }
    Ok(())
}

/// Computes D8 flow directions with Barnes (2014) flat resolution.
///
/// If `alter` is `true`, alters `elevations` in place (floating-point only) and recomputes
/// D8 flow directions; otherwise resolves flat flow directions directly from the flat mask.
pub fn barnes_flat_resolution_d8<T: DemNum>(
    elevations: &mut Array2D<T>,
    alter: bool,
) -> Result<Array2D<D8Flowdir>> {
    let mut flowdirs = d8_flow_directions(elevations);
    let (flat_mask, labels) = resolve_flats_barnes(elevations, &flowdirs);
    if alter {
        d8_flats_alter_dem(&flat_mask, &labels, elevations)?;
        flowdirs = d8_flow_directions(elevations);
    } else {
        d8_flow_flats(&flat_mask, &labels, &mut flowdirs);
    }
    flowdirs.copy_metadata_from(elevations);
    Ok(flowdirs)
}
