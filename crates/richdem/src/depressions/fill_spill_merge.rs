//! Fill-Spill-Merge hydrological water routing in depression hierarchies
//! (Barnes, Callaghan, and Wickert, 2021).
//!
//! Reference: Barnes, R., Callaghan, K.L., Wickert, A.D., 2021. Computing water flow
//! through complex landscapes - Part 3: Fill-Spill-Merge: Flow routing in depression
//! hierarchies. Earth Surface Dynamics 9, 105-121. doi:10.5194/esurf-9-105-2021

use std::cmp::Ordering;
use std::collections::{HashMap, HashSet, VecDeque};

use crate::array2d::Array2D;
use crate::constants::{Flowdir, NO_FLOW, Topology, fp_eq, fp_ge, fp_le};
use crate::depressions::hierarchy::{DepressionHierarchy, DhLabel, FlatIdx, NO_VALUE, OCEAN};
use crate::error::{Error, Result};
use crate::grid_cell::GridCellZkHighPq;
use crate::num::DemNum;

/// Resets water volumes in all depressions of `deps` to `0.0`.
pub fn reset_dh<T: DemNum>(deps: &mut DepressionHierarchy<T>) {
    for dep in deps.iter_mut() {
        dep.water_vol = 0.0;
    }
}

/// Calculates the volume of a depression dammed by a sill of elevation `sill_elevation`.
#[inline]
pub fn depression_volume<T: DemNum>(
    sill_elevation: T,
    cells_in_depression: usize,
    total_elevation: f64,
) -> f64 {
    (cells_in_depression as f64) * sill_elevation.to_f64() - total_elevation
}

/// The Lake-Level Equation: computes water surface elevation and updates `sill_wtd`
/// if water spills slightly into the sill's groundwater storage.
pub fn determine_water_level<W: DemNum>(
    sill_wtd: &mut W,
    mut water_vol: f64,
    sill_elevation: f64,
    cells_to_spread_across: usize,
    total_elevation: f64,
) -> f64 {
    let current_dep_volume =
        depression_volume(sill_elevation, cells_to_spread_across, total_elevation);

    if water_vol > current_dep_volume {
        if fp_eq(water_vol, current_dep_volume) {
            water_vol = current_dep_volume;
        }
        debug_assert!(fp_le(water_vol, current_dep_volume - sill_wtd.to_f64()));

        let fill_amount = water_vol - current_dep_volume;
        debug_assert!(fill_amount >= 0.0);
        debug_assert!(fill_amount <= -sill_wtd.to_f64() + 1e-6);
        *sill_wtd = W::from_f64(sill_wtd.to_f64() + fill_amount);
        sill_elevation
    } else if fp_eq(water_vol, current_dep_volume) {
        sill_elevation
    } else {
        let nominal_level = (water_vol + total_elevation) / (cells_to_spread_across as f64);
        if fp_eq(nominal_level, sill_elevation) {
            sill_elevation
        } else {
            nominal_level
        }
    }
}

/// Backfills standing surface water into `cells_affected` up to `water_level`.
pub fn backfill_depression<T: DemNum, W: DemNum>(
    water_level: f64,
    topo: &Array2D<T>,
    wtd: &mut Array2D<W>,
    cells_affected: &[FlatIdx],
) {
    for &c in cells_affected {
        let ci = c as usize;
        debug_assert_eq!(wtd.get_i(ci).to_f64(), 0.0);
        let elev = topo.get_i(ci).to_f64();
        if water_level < elev {
            debug_assert!(fp_ge(water_level, elev));
        }
        let depth = (water_level - elev).max(0.0);
        wtd.set_i(ci, W::from_f64(depth));
    }
}

/// Validates dimensions, flow directions, and depression labels before mutating state.
fn validate_fsm_inputs<T: DemNum, W: DemNum>(
    topo: &Array2D<T>,
    label: &Array2D<DhLabel>,
    flowdirs: &Array2D<Flowdir>,
    deps: &DepressionHierarchy<T>,
    wtd: &Array2D<W>,
    topology: Topology,
) -> Result<()> {
    let w = topo.width();
    let h = topo.height();
    if label.width() != w
        || label.height() != h
        || flowdirs.width() != w
        || flowdirs.height() != h
        || wtd.width() != w
        || wtd.height() != h
    {
        return Err(Error::DimensionMismatch(
            "topo, label, flowdirs, and wtd must have identical dimensions".into(),
        ));
    }
    if deps.is_empty() {
        return Err(Error::InvalidArgument(
            "DepressionHierarchy must contain at least the OCEAN entry".into(),
        ));
    }
    let dx = topology.dx();
    let dy = topology.dy();
    let nmax = topology.nmax() as Flowdir;
    for y in 0..h {
        for x in 0..w {
            let lbl = label.get(x, y) as usize;
            if lbl >= deps.len() {
                return Err(Error::InvalidArgument(format!(
                    "label {lbl} at ({x}, {y}) exceeds DepressionHierarchy length {}",
                    deps.len()
                )));
            }
            let dir = flowdirs.get(x, y);
            if dir == NO_FLOW {
                continue;
            }
            if dir < 1 || dir > nmax {
                return Err(Error::InvalidArgument(format!(
                    "invalid flow direction {dir} at ({x}, {y}) for {topology:?} topology"
                )));
            }
            let nx = x as i32 + dx[dir as usize];
            let ny = y as i32 + dy[dir as usize];
            if !topo.in_grid(nx, ny) {
                return Err(Error::InvalidArgument(format!(
                    "flow direction {dir} at ({x}, {y}) points outside grid ({w}x{h})"
                )));
            }
        }
    }
    Ok(())
}

/// Routes standing surface water downhill along `flowdirs` into leaf depression pits
/// (or into `OCEAN`), saturating negative groundwater storage along the way.
pub fn move_water_into_pits<T: DemNum, W: DemNum>(
    topo: &Array2D<T>,
    label: &Array2D<DhLabel>,
    flowdirs: &Array2D<Flowdir>,
    deps: &mut DepressionHierarchy<T>,
    wtd: &mut Array2D<W>,
) {
    let topology = deps.first().map_or(Topology::D8, |d| d.topology);
    move_water_into_pits_with_topology(topo, label, flowdirs, deps, wtd, topology);
}

fn move_water_into_pits_with_topology<T: DemNum, W: DemNum>(
    topo: &Array2D<T>,
    label: &Array2D<DhLabel>,
    flowdirs: &Array2D<Flowdir>,
    deps: &mut DepressionHierarchy<T>,
    wtd: &mut Array2D<W>,
    topology: Topology,
) {
    let w = topo.width();
    let h = topo.height();
    let dx = topology.dx();
    let dy = topology.dy();
    let dinverse = topology.dinverse();
    let nmax = topology.nmax();

    let mut dependencies = Array2D::<i8>::new(w, h, 0);
    for y in 0..h {
        for x in 0..w {
            let mut dep_count: i8 = 0;
            for n in 1..=nmax {
                let nx = x as i32 + dx[n];
                let ny = y as i32 + dy[n];
                if !topo.in_grid(nx, ny) {
                    continue;
                }
                if flowdirs.get_xy(nx, ny) == dinverse[n] as Flowdir {
                    dep_count += 1;
                }
            }
            dependencies.set(x, y, dep_count);
        }
    }

    let mut q = VecDeque::<FlatIdx>::new();
    for i in 0..topo.size() {
        if dependencies.get_i(i) == 0 {
            q.push_back(i as FlatIdx);
        }
    }

    while let Some(c) = q.pop_front() {
        let ci = c as usize;
        let ndir = flowdirs.get_i(ci);
        let downstream: Option<usize> = if ndir != NO_FLOW && ndir >= 1 && (ndir as usize) <= nmax {
            let (x, y) = topo.i_to_xy(ci);
            let nx = x as i32 + dx[ndir as usize];
            let ny = y as i32 + dy[ndir as usize];
            if topo.in_grid(nx, ny) {
                Some(topo.xy_to_i(nx as usize, ny as usize))
            } else {
                None
            }
        } else {
            None
        };

        match downstream {
            None => {
                let cw = wtd.get_i(ci).to_f64();
                if cw > 0.0 {
                    deps[label.get_i(ci) as usize].water_vol += cw;
                    wtd.set_i(ci, W::ZERO);
                }
            }
            Some(ni) => {
                let cw = wtd.get_i(ci).to_f64();
                if cw > 0.0 {
                    let nw = wtd.get_i(ni).to_f64();
                    wtd.set_i(ni, W::from_f64(nw + cw));
                    wtd.set_i(ci, W::ZERO);
                }
                let rem = dependencies.get_i(ni) - 1;
                dependencies.set_i(ni, rem);
                debug_assert!(rem >= 0);
                if rem == 0 {
                    q.push_back(ni as FlatIdx);
                }
            }
        }
    }
}

fn overflow_into<T: DemNum>(
    root: DhLabel,
    stop_node: DhLabel,
    deps: &mut DepressionHierarchy<T>,
    jump_table: &mut HashMap<DhLabel, DhLabel>,
    mut extra_water: f64,
) -> DhLabel {
    let r = root as usize;
    if deps[r].water_vol > deps[r].dep_vol {
        extra_water += deps[r].water_vol - deps[r].dep_vol;
        deps[r].water_vol = deps[r].dep_vol;
    }

    if root == stop_node || root == OCEAN {
        deps[r].water_vol += extra_water;
        return root;
    }

    // 1. Stash water in this depression
    if deps[r].water_vol < deps[r].dep_vol {
        let capacity = deps[r].dep_vol - deps[r].water_vol;
        if extra_water < capacity {
            deps[r].water_vol = (deps[r].water_vol + extra_water).min(deps[r].dep_vol);
            extra_water = 0.0;
        } else {
            deps[r].water_vol = deps[r].dep_vol;
            extra_water -= capacity;
        }
    }

    if fp_eq(extra_water, 0.0) {
        return root;
    }

    if let Some(&target) = jump_table.get(&root) {
        let dest = overflow_into(target, stop_node, deps, jump_table, extra_water);
        jump_table.insert(root, dest);
        return dest;
    }

    // 2. Overflow into sibling/neighbor depression
    let odep_id = deps[r].odep;
    if odep_id != NO_VALUE {
        let oi = odep_id as usize;
        if deps[oi].water_vol < deps[oi].dep_vol {
            let geolink = deps[r].geolink;
            let dest = overflow_into(geolink, stop_node, deps, jump_table, extra_water);
            jump_table.insert(root, dest);
            return dest;
        } else if deps[oi].water_vol > deps[oi].dep_vol {
            extra_water += deps[oi].water_vol - deps[oi].dep_vol;
            deps[oi].water_vol = deps[oi].dep_vol;
        }
    }

    // 3. Overflow into parent
    let parent_id = deps[r].parent;
    let ocean_parent = deps[r].ocean_parent;
    let this_water = deps[r].water_vol;
    if deps[parent_id as usize].water_vol == 0.0 && !ocean_parent {
        deps[parent_id as usize].water_vol += this_water;
        if odep_id != NO_VALUE {
            let odep_water = deps[odep_id as usize].water_vol;
            deps[parent_id as usize].water_vol += odep_water;
        }
    }

    let dest = overflow_into(parent_id, stop_node, deps, jump_table, extra_water);
    jump_table.insert(root, dest);
    dest
}

fn move_water_in_dep_hier<T: DemNum>(
    current_depression: DhLabel,
    deps: &mut DepressionHierarchy<T>,
    jump_table: &mut HashMap<DhLabel, DhLabel>,
) {
    if current_depression == NO_VALUE {
        return;
    }
    let idx = current_depression as usize;
    let ocean_linked = deps[idx].ocean_linked.clone();
    let lchild = deps[idx].lchild;
    let rchild = deps[idx].rchild;

    for c in ocean_linked {
        move_water_in_dep_hier(c, deps, jump_table);
    }

    move_water_in_dep_hier(lchild, deps, jump_table);
    move_water_in_dep_hier(rchild, deps, jump_table);

    if current_depression == OCEAN {
        return;
    }

    if lchild != NO_VALUE
        && deps[lchild as usize].water_vol == deps[lchild as usize].dep_vol
        && deps[rchild as usize].water_vol == deps[rchild as usize].dep_vol
        && deps[idx].water_vol == 0.0
    {
        let sum = deps[lchild as usize].water_vol + deps[rchild as usize].water_vol;
        deps[idx].water_vol += sum;
    }

    if deps[idx].water_vol > deps[idx].dep_vol {
        let parent = deps[idx].parent;
        overflow_into(current_depression, parent, deps, jump_table, 0.0);
    }
}

#[derive(Default)]
struct SubtreeDepressionInfo {
    leaf_label: DhLabel,
    top_label: DhLabel,
    my_labels: HashSet<DhLabel>,
}

impl SubtreeDepressionInfo {
    fn empty() -> Self {
        Self {
            leaf_label: NO_VALUE,
            top_label: NO_VALUE,
            my_labels: HashSet::new(),
        }
    }
}

/// Spreads `water_vol` across cells belonging to `dep_labels` starting from `pit_cell`
/// up to `out_cell`, updating `wtd` in place (D8 topology).
pub fn fill_depressions_fsm<T: DemNum, W: DemNum>(
    pit_cell: FlatIdx,
    out_cell: FlatIdx,
    dep_labels: &HashSet<DhLabel>,
    water_vol: f64,
    topo: &Array2D<T>,
    label: &Array2D<DhLabel>,
    wtd: &mut Array2D<W>,
) -> Result<()> {
    fill_depressions_fsm_with_topology(
        pit_cell,
        out_cell,
        dep_labels,
        water_vol,
        topo,
        label,
        wtd,
        Topology::D8,
    )
}

#[allow(clippy::too_many_arguments)]
fn fill_depressions_fsm_with_topology<T: DemNum, W: DemNum>(
    pit_cell: FlatIdx,
    out_cell: FlatIdx,
    dep_labels: &HashSet<DhLabel>,
    mut water_vol: f64,
    topo: &Array2D<T>,
    label: &Array2D<DhLabel>,
    wtd: &mut Array2D<W>,
    topology: Topology,
) -> Result<()> {
    if water_vol == 0.0 {
        return Ok(());
    }

    let dx = topology.dx();
    let dy = topology.dy();
    let nmax = topology.nmax();

    let mut visited = HashSet::<FlatIdx>::with_capacity(2048);
    let mut flood_q = GridCellZkHighPq::<T>::new();

    let (px, py) = topo.i_to_xy(pit_cell as usize);
    flood_q.emplace(px as i32, py as i32, topo.get_i(pit_cell as usize));
    visited.insert(pit_cell);

    let mut cells_affected: Vec<FlatIdx> = Vec::new();
    let mut saved_wtd: Vec<(usize, W)> = Vec::new();
    let mut total_elevation = 0.0f64;
    let mut out_cell_fallback_len: Option<usize> = None;
    let out_elev = topo.get_i(out_cell as usize);

    let restore_wtd = |wtd: &mut Array2D<W>, saved: &[(usize, W)]| {
        for &(ci, orig) in saved {
            wtd.set_i(ci, orig);
        }
    };

    while let Some(c) = flood_q.pop() {
        let ci = topo.xy_to_i(c.x as usize, c.y as usize) as FlatIdx;
        let celev = topo.get_i(ci as usize);
        let current_volume = depression_volume(celev, cells_affected.len(), total_elevation);

        if dep_labels.contains(&label.get_i(ci as usize)) && wtd.get_i(ci as usize).to_f64() > 0.0 {
            restore_wtd(wtd, &saved_wtd);
            return Err(Error::Algorithm(
                "A cell was discovered in an unfilled depression with wtd > 0!".into(),
            ));
        }

        let cwtd = wtd.get_i(ci as usize).to_f64();
        if fp_le(water_vol, current_volume - cwtd) {
            let mut sill_wtd = wtd.get_i(ci as usize);
            let mut water_level = determine_water_level(
                &mut sill_wtd,
                water_vol,
                celev.to_f64(),
                cells_affected.len(),
                total_elevation,
            );
            wtd.set_i(ci as usize, sill_wtd);
            if fp_eq(water_level, out_elev.to_f64()) {
                water_level = out_elev.to_f64();
            }

            backfill_depression(water_level, topo, wtd, &cells_affected);
            return Ok(());
        }

        if ci != out_cell {
            cells_affected.push(ci);
            saved_wtd.push((ci as usize, wtd.get_i(ci as usize)));
            water_vol += cwtd;
            wtd.set_i(ci as usize, W::ZERO);
            total_elevation += celev.to_f64();
        }

        for n in 1..=nmax {
            let nx = c.x + dx[n];
            let ny = c.y + dy[n];
            if !topo.in_grid(nx, ny) {
                continue;
            }
            let ni = topo.xy_to_i(nx as usize, ny as usize) as FlatIdx;
            if !dep_labels.contains(&label.get_i(ni as usize)) && ni != out_cell {
                continue;
            }
            if topo.get_i(ni as usize).rd_cmp(out_elev) == Ordering::Greater {
                continue;
            }
            if visited.insert(ni) {
                flood_q.emplace(nx, ny, topo.get_i(ni as usize));
            }
        }

        if flood_q.is_empty() {
            if out_cell_fallback_len != Some(cells_affected.len()) {
                out_cell_fallback_len = Some(cells_affected.len());
                let (ox, oy) = topo.i_to_xy(out_cell as usize);
                flood_q.emplace(ox as i32, oy as i32, out_elev);
                visited.insert(out_cell);
            } else {
                restore_wtd(wtd, &saved_wtd);
                return Err(Error::Algorithm(
                    "Depression cannot accommodate remaining water volume up to out_cell!".into(),
                ));
            }
        }
    }

    restore_wtd(wtd, &saved_wtd);
    Err(Error::Algorithm(
        "PQ loop exited without filling a depression!".into(),
    ))
}

fn find_depressions_to_fill<T: DemNum, W: DemNum>(
    current_depression: DhLabel,
    deps: &DepressionHierarchy<T>,
    topo: &Array2D<T>,
    label: &Array2D<DhLabel>,
    wtd: &mut Array2D<W>,
    topology: Topology,
) -> Result<SubtreeDepressionInfo> {
    if current_depression == NO_VALUE {
        return Ok(SubtreeDepressionInfo::empty());
    }

    let this_dep = &deps[current_depression as usize];
    for &c in &this_dep.ocean_linked {
        find_depressions_to_fill(c, deps, topo, label, wtd, topology)?;
    }

    if current_depression == OCEAN {
        return Ok(SubtreeDepressionInfo::empty());
    }

    let left_info = find_depressions_to_fill(this_dep.lchild, deps, topo, label, wtd, topology)?;
    let right_info = find_depressions_to_fill(this_dep.rchild, deps, topo, label, wtd, topology)?;

    let mut combined = SubtreeDepressionInfo::empty();
    combined.my_labels.insert(current_depression);
    combined.my_labels.extend(left_info.my_labels);
    combined.my_labels.extend(right_info.my_labels);

    combined.leaf_label = if left_info.leaf_label == NO_VALUE {
        current_depression
    } else {
        left_info.leaf_label
    };
    combined.top_label = current_depression;

    if this_dep.water_vol < this_dep.dep_vol
        || this_dep.ocean_parent
        || (this_dep.water_vol == this_dep.dep_vol
            && deps[this_dep.parent as usize].water_vol == 0.0)
    {
        fill_depressions_fsm_with_topology(
            deps[combined.leaf_label as usize].pit_cell,
            deps[combined.top_label as usize].out_cell,
            &combined.my_labels,
            this_dep.water_vol,
            topo,
            label,
            wtd,
            topology,
        )?;
        Ok(SubtreeDepressionInfo::empty())
    } else {
        Ok(combined)
    }
}

/// Executes the complete Fill-Spill-Merge algorithm (Barnes et al., 2021).
///
/// Routes standing surface water (`wtd > 0`) downhill into depressions, cascades
/// overflows across the depression hierarchy (and to the ocean), and backfills
/// lakes to their equilibrium levels in `wtd`.
pub fn fill_spill_merge<T: DemNum, W: DemNum>(
    topo: &Array2D<T>,
    label: &Array2D<DhLabel>,
    flowdirs: &Array2D<Flowdir>,
    deps: &mut DepressionHierarchy<T>,
    wtd: &mut Array2D<W>,
) -> Result<()> {
    let topology = deps.first().map_or(Topology::D8, |d| d.topology);
    validate_fsm_inputs(topo, label, flowdirs, deps, wtd, topology)?;

    reset_dh(deps);
    move_water_into_pits_with_topology(topo, label, flowdirs, deps, wtd, topology);

    let mut jump_table = HashMap::<DhLabel, DhLabel>::new();
    move_water_in_dep_hier(OCEAN, deps, &mut jump_table);
    drop(jump_table);

    find_depressions_to_fill(OCEAN, deps, topo, label, wtd, topology)?;
    Ok(())
}
