//! Priority-Flood depression-filling, watershed-labeling, and pit-masking algorithms
//! from Barnes, Lehman, and Mulla (2014).
//!
//! Reference: Barnes, R., Lehman, C., Mulla, D., 2014. Priority-flood: An optimal
//! depression-filling and watershed-labeling algorithm for digital elevation models.
//! Computers & Geosciences 62, 117-127. doi:10.1016/j.cageo.2013.04.024

use std::collections::VecDeque;

use crate::array2d::Array2D;
use crate::constants::{D8_INVERSE, D8Flowdir, D8X, D8Y, NO_FLOW, Topology};
use crate::error::{Error, Result};
use crate::grid_cell::{GridCell, GridCellZ, GridCellZPq, GridCellZkLowPq};
use crate::num::DemNum;

fn seed_perimeter_pq<T: DemNum>(
    elevations: &Array2D<T>,
    open: &mut GridCellZPq<T>,
    closed: &mut Array2D<u8>,
) {
    let w = elevations.width();
    let h = elevations.height();
    if w == 0 || h == 0 {
        return;
    }
    for x in 0..w {
        open.push(GridCellZ::new(x as i32, 0, elevations.get(x, 0)));
        closed.set(x, 0, 1);
        if h > 1 {
            open.push(GridCellZ::new(
                x as i32,
                (h - 1) as i32,
                elevations.get(x, h - 1),
            ));
            closed.set(x, h - 1, 1);
        }
    }
    for y in 1..h.saturating_sub(1) {
        open.push(GridCellZ::new(0, y as i32, elevations.get(0, y)));
        closed.set(0, y, 1);
        if w > 1 {
            open.push(GridCellZ::new(
                (w - 1) as i32,
                y as i32,
                elevations.get(w - 1, y),
            ));
            closed.set(w - 1, y, 1);
        }
    }
}

/// Seeds valid perimeter cells and valid cells adjacent to NoData, while marking
/// all NoData cells as closed so they are never modified or treated as pits.
fn seed_nodata_aware_pq<T: DemNum>(
    elevations: &Array2D<T>,
    open: &mut GridCellZPq<T>,
    closed: &mut Array2D<u8>,
    topo: Topology,
) {
    let w = elevations.width();
    let h = elevations.height();
    if w == 0 || h == 0 {
        return;
    }
    let dx = topo.dx();
    let dy = topo.dy();
    let nmax = topo.nmax();
    let nd = elevations.no_data();

    for y in 0..h {
        for x in 0..w {
            let v = elevations.get(x, y);
            if v.rd_eq(nd) {
                closed.set(x, y, 1);
                continue;
            }
            if elevations.is_edge_cell(x, y) {
                open.push(GridCellZ::new(x as i32, y as i32, v));
                closed.set(x, y, 1);
                continue;
            }
            for n in 1..=nmax {
                let nx = x as i32 + dx[n];
                let ny = y as i32 + dy[n];
                if elevations.in_grid(nx, ny) && elevations.get_xy(nx, ny).rd_eq(nd) {
                    open.push(GridCellZ::new(x as i32, y as i32, v));
                    closed.set(x, y, 1);
                    break;
                }
            }
        }
    }
}

/// Returns `true` if `elevations` contains at least one depression under `topo`.
pub fn has_depressions<T: DemNum>(elevations: &Array2D<T>, topo: Topology) -> bool {
    let dx = topo.dx();
    let dy = topo.dy();
    let nmax = topo.nmax();

    let mut open = GridCellZPq::new();
    let mut closed = Array2D::<u8>::new(elevations.width(), elevations.height(), 0);
    seed_nodata_aware_pq(elevations, &mut open, &mut closed, topo);

    while let Some(c) = open.pop() {
        let cz = elevations.get_xy(c.x, c.y);
        for n in 1..=nmax {
            let nx = c.x + dx[n];
            let ny = c.y + dy[n];
            if !elevations.in_grid(nx, ny) || closed.get_xy(nx, ny) != 0 {
                continue;
            }
            closed.set_xy(nx, ny, 1);
            let nz = elevations.get_xy(nx, ny);
            if nz.rd_cmp(cz) == std::cmp::Ordering::Less {
                return true;
            }
            open.push(GridCellZ::new(nx, ny, nz));
        }
    }
    false
}

/// Original Priority-Flood depression filling (Barnes et al., 2014, Algorithm 1).
pub fn priority_flood_original<T: DemNum>(elevations: &mut Array2D<T>, topo: Topology) {
    let dx = topo.dx();
    let dy = topo.dy();
    let nmax = topo.nmax();

    let mut open = GridCellZPq::new();
    let mut closed = Array2D::<u8>::new(elevations.width(), elevations.height(), 0);
    seed_nodata_aware_pq(elevations, &mut open, &mut closed, topo);

    while let Some(c) = open.pop() {
        let cz = elevations.get_xy(c.x, c.y);
        for n in 1..=nmax {
            let nx = c.x + dx[n];
            let ny = c.y + dy[n];
            if !elevations.in_grid(nx, ny) || closed.get_xy(nx, ny) != 0 {
                continue;
            }
            closed.set_xy(nx, ny, 1);
            let nz = elevations.get_xy(nx, ny).rd_max(cz);
            elevations.set_xy(nx, ny, nz);
            open.push(GridCellZ::new(nx, ny, nz));
        }
    }
}

/// Improved Priority-Flood with pit queue (Barnes et al., 2014, Algorithm 2).
pub fn priority_flood_barnes2014<T: DemNum>(elevations: &mut Array2D<T>, topo: Topology) {
    let dx = topo.dx();
    let dy = topo.dy();
    let nmax = topo.nmax();

    let mut open = GridCellZPq::new();
    let mut pit = VecDeque::<GridCellZ<T>>::new();
    let mut closed = Array2D::<u8>::new(elevations.width(), elevations.height(), 0);
    seed_nodata_aware_pq(elevations, &mut open, &mut closed, topo);

    while !open.is_empty() || !pit.is_empty() {
        let c = if let Some(p) = pit.pop_front() {
            p
        } else {
            open.pop().unwrap()
        };

        for n in 1..=nmax {
            let nx = c.x + dx[n];
            let ny = c.y + dy[n];
            if !elevations.in_grid(nx, ny) || closed.get_xy(nx, ny) != 0 {
                continue;
            }
            closed.set_xy(nx, ny, 1);
            let nz = elevations.get_xy(nx, ny);
            if nz.rd_cmp(c.z) != std::cmp::Ordering::Greater {
                if nz.rd_cmp(c.z) == std::cmp::Ordering::Less {
                    elevations.set_xy(nx, ny, c.z);
                }
                pit.push_back(GridCellZ::new(nx, ny, c.z));
            } else {
                open.push(GridCellZ::new(nx, ny, nz));
            }
        }
    }
}

/// Priority-Flood+Epsilon (Barnes et al., 2014, Algorithm 3).
///
/// Raises depression cells by minimal representable floating-point increments (`next_up`)
/// to guarantee drainage without flats. Returns the count of `false_pit_cells` warnings,
/// or [`Error::UnsupportedDataType`] for integer DEMs.
pub fn priority_flood_epsilon_barnes2014<T: DemNum>(
    elevations: &mut Array2D<T>,
    topo: Topology,
) -> Result<usize> {
    if !T::IS_FLOAT {
        return Err(Error::UnsupportedDataType(
            "Priority-Flood+Epsilon is only available for floating-point data types!".into(),
        ));
    }

    let dx = topo.dx();
    let dy = topo.dy();
    let nmax = topo.nmax();

    let mut open = GridCellZPq::new();
    let mut pit = VecDeque::<GridCellZ<T>>::new();
    let mut closed = Array2D::<u8>::new(elevations.width(), elevations.height(), 0);
    let no_data = elevations.no_data();
    let mut pit_top = no_data;
    let mut false_pit_cells: usize = 0;

    seed_perimeter_pq(elevations, &mut open, &mut closed);

    while !open.is_empty() || !pit.is_empty() {
        let c = if let (Some(p), Some(o)) = (pit.front(), open.peek()) {
            if o.z.rd_eq(p.z) {
                pit_top = no_data;
                open.pop().unwrap()
            } else {
                let cell = pit.pop_front().unwrap();
                if pit_top.rd_eq(no_data) {
                    pit_top = elevations.get_xy(cell.x, cell.y);
                }
                cell
            }
        } else if let Some(cell) = pit.pop_front() {
            if pit_top.rd_eq(no_data) {
                pit_top = elevations.get_xy(cell.x, cell.y);
            }
            cell
        } else {
            pit_top = no_data;
            open.pop().unwrap()
        };

        let next_cz = c.z.next_up_float().unwrap();

        for n in 1..=nmax {
            let nx = c.x + dx[n];
            let ny = c.y + dy[n];
            if !elevations.in_grid(nx, ny) || closed.get_xy(nx, ny) != 0 {
                continue;
            }
            closed.set_xy(nx, ny, 1);

            let nz = elevations.get_xy(nx, ny);
            if nz.rd_eq(no_data) {
                pit.push_back(GridCellZ::new(nx, ny, no_data));
            } else if nz.rd_cmp(next_cz) != std::cmp::Ordering::Greater {
                if !pit_top.rd_eq(no_data)
                    && pit_top.rd_cmp(nz) == std::cmp::Ordering::Less
                    && next_cz.rd_cmp(nz) != std::cmp::Ordering::Less
                {
                    false_pit_cells += 1;
                }
                elevations.set_xy(nx, ny, next_cz);
                pit.push_back(GridCellZ::new(nx, ny, next_cz));
            } else {
                open.push(GridCellZ::new(nx, ny, nz));
            }
        }
    }

    Ok(false_pit_cells)
}

/// Determines D8 flow directions directly via Priority-Flood, implicitly filling pits
/// (Barnes et al., 2014, Algorithm 4; based on Metz et al., 2011).
pub fn priority_flood_flowdirs_barnes2014<T: DemNum>(
    elevations: &Array2D<T>,
) -> Array2D<D8Flowdir> {
    let w = elevations.width();
    let h = elevations.height();
    let mut open = GridCellZkLowPq::<T>::new();
    let mut closed = Array2D::<u8>::new(w, h, 0);
    let mut flowdirs = Array2D::<D8Flowdir>::make_from_template(elevations, NO_FLOW as u8);
    flowdirs.set_no_data(NO_FLOW as u8);

    if w == 0 || h == 0 {
        return flowdirs;
    }

    let no_data = elevations.no_data();
    let is_missing =
        |v: T| -> bool { v.rd_eq(no_data) || (T::IS_FLOAT && !v.to_f64().is_finite()) };
    let prio = |v: T| -> T { if is_missing(v) { T::NEG_INFINITY } else { v } };

    for x in 0..w {
        let v = elevations.get(x, 0);
        open.emplace(x as i32, 0, prio(v));
        if !is_missing(v) {
            flowdirs.set(x, 0, 3);
        }
        closed.set(x, 0, 1);
        if h > 1 {
            let vb = elevations.get(x, h - 1);
            open.emplace(x as i32, (h - 1) as i32, prio(vb));
            if !is_missing(vb) {
                flowdirs.set(x, h - 1, 7);
            }
            closed.set(x, h - 1, 1);
        }
    }
    for y in 1..h.saturating_sub(1) {
        let vl = elevations.get(0, y);
        open.emplace(0, y as i32, prio(vl));
        if !is_missing(vl) {
            flowdirs.set(0, y, 1);
        }
        closed.set(0, y, 1);
        if w > 1 {
            let vr = elevations.get(w - 1, y);
            open.emplace((w - 1) as i32, y as i32, prio(vr));
            if !is_missing(vr) {
                flowdirs.set(w - 1, y, 5);
            }
            closed.set(w - 1, y, 1);
        }
    }

    if !is_missing(elevations.get(0, 0)) {
        flowdirs.set(0, 0, 2);
    }
    if !is_missing(elevations.get(w - 1, 0)) {
        flowdirs.set(w - 1, 0, 4);
    }
    if !is_missing(elevations.get(0, h - 1)) {
        flowdirs.set(0, h - 1, 8);
    }
    if !is_missing(elevations.get(w - 1, h - 1)) {
        flowdirs.set(w - 1, h - 1, 6);
    }

    const D8_ORDER: [usize; 9] = [0, 1, 3, 5, 7, 2, 4, 6, 8];
    let fd_nd = flowdirs.no_data();

    while let Some(c) = open.pop() {
        for &n in &D8_ORDER[1..=8] {
            let nx = c.x + D8X[n];
            let ny = c.y + D8Y[n];
            if !elevations.in_grid(nx, ny) || closed.get_xy(nx, ny) != 0 {
                continue;
            }
            closed.set_xy(nx, ny, 1);
            let nz = elevations.get_xy(nx, ny);
            if is_missing(nz) {
                flowdirs.set_xy(nx, ny, fd_nd);
                open.emplace(nx, ny, T::NEG_INFINITY);
            } else {
                flowdirs.set_xy(nx, ny, D8_INVERSE[n]);
                open.emplace(nx, ny, nz);
            }
        }
    }

    flowdirs
}

/// Creates a mask indicating which cells belong to depressions.
///
/// Output values: `1` = cell in a depression, `0` = cell not in a depression, `3` = NoData.
pub fn pit_mask<T: DemNum>(elevations: &Array2D<T>, topo: Topology) -> Array2D<u8> {
    let dx = topo.dx();
    let dy = topo.dy();
    let nmax = topo.nmax();

    let mut open = GridCellZPq::new();
    let mut pit = VecDeque::<GridCellZ<T>>::new();
    let mut closed = Array2D::<u8>::new(elevations.width(), elevations.height(), 0);
    let mut mask = Array2D::<u8>::make_from_template(elevations, 0);
    mask.set_no_data(3);

    seed_perimeter_pq(elevations, &mut open, &mut closed);

    while !open.is_empty() || !pit.is_empty() {
        let c = if let Some(p) = pit.pop_front() {
            p
        } else {
            open.pop().unwrap()
        };

        for n in 1..=nmax {
            let nx = c.x + dx[n];
            let ny = c.y + dy[n];
            if !elevations.in_grid(nx, ny) || closed.get_xy(nx, ny) != 0 {
                continue;
            }
            closed.set_xy(nx, ny, 1);
            let nz = elevations.get_xy(nx, ny);
            if nz.rd_cmp(c.z) != std::cmp::Ordering::Greater {
                if nz.rd_cmp(c.z) == std::cmp::Ordering::Less {
                    mask.set_xy(nx, ny, 1);
                }
                pit.push_back(GridCellZ::new(nx, ny, c.z));
            } else {
                mask.set_xy(nx, ny, 0);
                open.push(GridCellZ::new(nx, ny, nz));
            }
        }

        if elevations.is_no_data_xy(c.x, c.y) {
            mask.set_xy(c.x, c.y, mask.no_data());
        }
    }

    mask
}

/// Labels watersheds draining to common perimeter outlets via Priority-Flood
/// (Barnes et al., 2014, Algorithm 5).
///
/// If `alter_elevations` is `true`, depressions in `elevations` are filled in place.
pub fn priority_flood_watersheds_barnes2014<T: DemNum>(
    elevations: &mut Array2D<T>,
    alter_elevations: bool,
    topo: Topology,
) -> Array2D<i32> {
    let dx = topo.dx();
    let dy = topo.dy();
    let nmax = topo.nmax();

    let mut open = GridCellZPq::new();
    let mut pit = VecDeque::<GridCellZ<T>>::new();
    let mut closed = Array2D::<u8>::new(elevations.width(), elevations.height(), 0);
    let mut labels = Array2D::<i32>::make_from_template(elevations, -1);
    labels.set_no_data(-1);

    seed_nodata_aware_pq(elevations, &mut open, &mut closed, topo);

    let mut clabel: i32 = 1;
    let no_data = elevations.no_data();

    while !open.is_empty() || !pit.is_empty() {
        let c = if let Some(p) = pit.pop_front() {
            p
        } else {
            open.pop().unwrap()
        };

        if labels.get_xy(c.x, c.y) == labels.no_data()
            && !elevations.get_xy(c.x, c.y).rd_eq(no_data)
        {
            labels.set_xy(c.x, c.y, clabel);
            clabel += 1;
        }

        let cur_label = labels.get_xy(c.x, c.y);

        for n in 1..=nmax {
            let nx = c.x + dx[n];
            let ny = c.y + dy[n];
            if !elevations.in_grid(nx, ny) || closed.get_xy(nx, ny) != 0 {
                continue;
            }
            closed.set_xy(nx, ny, 1);

            let nz = elevations.get_xy(nx, ny);
            if nz.rd_eq(no_data) {
                continue;
            }
            labels.set_xy(nx, ny, cur_label);
            if nz.rd_cmp(c.z) != std::cmp::Ordering::Greater {
                if alter_elevations {
                    elevations.set_xy(nx, ny, c.z);
                }
                pit.push_back(GridCellZ::new(nx, ny, c.z));
            } else {
                open.push(GridCellZ::new(nx, ny, nz));
            }
        }
    }

    labels
}

/// Fills only depressions whose cell count is `<= max_dep_size`.
pub fn priority_flood_barnes2014_max_dep<T: DemNum>(
    elevations: &mut Array2D<T>,
    max_dep_size: usize,
    topo: Topology,
) {
    let dx = topo.dx();
    let dy = topo.dy();
    let nmax = topo.nmax();

    let mut open = GridCellZPq::new();
    let mut pit = VecDeque::<GridCellZ<T>>::new();
    let mut closed = Array2D::<u8>::new(elevations.width(), elevations.height(), 0);
    seed_nodata_aware_pq(elevations, &mut open, &mut closed, topo);

    let mut dep_elev = T::ZERO;
    let mut dep_cells: Vec<GridCell> = Vec::new();

    while !open.is_empty() || !pit.is_empty() {
        let c = if let Some(p) = pit.pop_front() {
            dep_cells.push(GridCell::new(p.x, p.y));
            p
        } else {
            let cell = open.pop().unwrap();
            if dep_cells.len() <= max_dep_size {
                for pc in &dep_cells {
                    elevations.set_xy(pc.x, pc.y, dep_elev);
                }
            }
            dep_cells.clear();
            cell
        };

        for n in 1..=nmax {
            let nx = c.x + dx[n];
            let ny = c.y + dy[n];
            if !elevations.in_grid(nx, ny) || closed.get_xy(nx, ny) != 0 {
                continue;
            }
            closed.set_xy(nx, ny, 1);
            let nz = elevations.get_xy(nx, ny);
            if nz.rd_cmp(c.z) == std::cmp::Ordering::Less {
                pit.push_back(GridCellZ::new(nx, ny, c.z));
                dep_elev = c.z;
            } else {
                open.push(GridCellZ::new(nx, ny, nz));
            }
        }
    }

    if !dep_cells.is_empty() && dep_cells.len() <= max_dep_size {
        for pc in &dep_cells {
            elevations.set_xy(pc.x, pc.y, dep_elev);
        }
    }
}
