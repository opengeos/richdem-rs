//! Depression Hierarchy construction (Barnes, Callaghan, and Wickert, 2020).
//!
//! Reference: Barnes, R., Callaghan, K.L., Wickert, A.D., 2020. Computing water flow
//! through complex landscapes - Part 2: Finding hierarchies in depressions and
//! morphological segmentations. Earth Surface Dynamics 8, 431-445.
//! doi:10.5194/esurf-8-431-2020

use std::cmp::Ordering;
use std::collections::{BinaryHeap, HashMap};
use std::fmt::Write as _;

use crate::array2d::{Array2D, STANDARD_GEOTRANSFORM};
use crate::constants::{Flowdir, Topology, fp_le};
use crate::disjoint_set::DisjointDenseIntSet;
use crate::error::{Error, Result};
use crate::grid_cell::KeyValEntry;
use crate::num::DemNum;

/// 32-bit label identifying a depression in a [`DepressionHierarchy`].
pub type DhLabel = u32;
/// 32-bit flat cell index into a 2D raster.
pub type FlatIdx = u32;

/// Sentinel indicating no parent depression.
pub const NO_PARENT: DhLabel = u32::MAX;
/// Sentinel indicating no cell/depression value.
pub const NO_VALUE: DhLabel = u32::MAX;
/// Sentinel indicating a cell has not yet been assigned to a depression.
pub const NO_DEP: DhLabel = u32::MAX;
/// Label `0` is always reserved for the ocean.
pub const OCEAN: DhLabel = 0;

/// Metadata for a single depression or meta-depression in a [`DepressionHierarchy`].
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Depression<T: DemNum> {
    /// Flat index of the lowest (pit) cell in the depression.
    pub pit_cell: FlatIdx,
    /// Flat index of the outlet (sill) cell of the depression.
    pub out_cell: FlatIdx,
    /// Parent depression ID in the hierarchy.
    pub parent: DhLabel,
    /// Sibling/overflow depression ID (`NO_VALUE` for ocean-linked depressions).
    pub odep: DhLabel,
    /// Geographic leaf depression ID into which overflow enters `odep`.
    pub geolink: DhLabel,
    /// Elevation of `pit_cell` (initialized to `+infinity`).
    pub pit_elev: T,
    /// Elevation of `out_cell` (initialized to `+infinity`).
    pub out_elev: T,
    /// Left child depression ID in the binary tree (`NO_VALUE` for leaves).
    pub lchild: DhLabel,
    /// Right child depression ID in the binary tree (`NO_VALUE` for leaves).
    pub rchild: DhLabel,
    /// Whether this depression links directly to the ocean (or an ocean-draining depression).
    pub ocean_parent: bool,
    /// Depressions that spill into this depression via an ocean link.
    pub ocean_linked: Vec<DhLabel>,
    /// Label ID of this depression.
    pub dep_label: DhLabel,
    /// Total number of cells contained within this depression and its children.
    pub cell_count: u32,
    /// Total storage volume of this depression and its children.
    pub dep_vol: f64,
    /// Water volume currently stored in this depression.
    pub water_vol: f64,
    /// Sum of elevations of all cells contained within this depression and its children.
    pub total_elevation: f64,
}

impl<T: DemNum> Default for Depression<T> {
    fn default() -> Self {
        Self {
            pit_cell: NO_VALUE,
            out_cell: NO_VALUE,
            parent: NO_PARENT,
            odep: NO_VALUE,
            geolink: NO_VALUE,
            pit_elev: T::INFINITY,
            out_elev: T::INFINITY,
            lchild: NO_VALUE,
            rchild: NO_VALUE,
            ocean_parent: false,
            ocean_linked: Vec::new(),
            dep_label: 0,
            cell_count: 0,
            dep_vol: 0.0,
            water_vol: 0.0,
            total_elevation: 0.0,
        }
    }
}

/// Ordered binary tree / forest of depressions rooted at [`OCEAN`] (`index 0`).
pub type DepressionHierarchy<T> = Vec<Depression<T>>;

#[derive(Debug, Clone, Copy)]
struct Outlet<T: DemNum> {
    depa: DhLabel,
    depb: DhLabel,
    out_cell: FlatIdx,
    out_elev: T,
}

impl<T: DemNum> Outlet<T> {
    fn new(mut depa: DhLabel, mut depb: DhLabel, out_cell: FlatIdx, out_elev: T) -> Self {
        if depa > depb {
            std::mem::swap(&mut depa, &mut depb);
        }
        Self {
            depa,
            depb,
            out_cell,
            out_elev,
        }
    }
}

#[inline]
fn outlet_key(a: DhLabel, b: DhLabel) -> (DhLabel, DhLabel) {
    if a < b { (a, b) } else { (b, a) }
}

/// Creates a new depression hierarchy label array with [`OCEAN`] on the perimeter
/// edges and [`NO_DEP`] in the interior.
pub fn get_new_depression_hierarchy_labels(
    width: usize,
    height: usize,
    geotransform: Option<[f64; 6]>,
) -> Array2D<DhLabel> {
    let mut labels = Array2D::new(width, height, NO_DEP);
    labels.set_edges(OCEAN);
    labels.geotransform = geotransform.unwrap_or(STANDARD_GEOTRANSFORM);
    labels
}

/// Computes the [`DepressionHierarchy`] for `dem`, populating `label` (leaf depression IDs)
/// and `flowdirs` (downhill flow direction `1..=nmax`, or `NO_FLOW` for pits and ocean).
///
/// Precondition: `label` must contain only [`OCEAN`] (at least one cell) and [`NO_DEP`].
pub fn get_depression_hierarchy<T: DemNum>(
    dem: &Array2D<T>,
    label: &mut Array2D<DhLabel>,
    flowdirs: &mut Array2D<Flowdir>,
    topo: Topology,
) -> Result<DepressionHierarchy<T>> {
    if dem.width() != label.width()
        || dem.height() != label.height()
        || dem.width() != flowdirs.width()
        || dem.height() != flowdirs.height()
    {
        return Err(Error::DimensionMismatch(
            "dem, label, and flowdirs must have identical dimensions".into(),
        ));
    }

    let w = dem.width();
    let h = dem.height();
    let dx = topo.dx();
    let dy = topo.dy();
    let dinverse = topo.dinverse();
    let neighbours = topo.nmax();

    let mut depressions: DepressionHierarchy<T> = Vec::new();
    let mut ocean_seeds: Vec<FlatIdx> = Vec::with_capacity((w * h) / 40);
    let mut land_seeds: Vec<FlatIdx> = Vec::with_capacity((w * h) / 40);

    for y in 0..h {
        for x in 0..w {
            let l = label.get(x, y);
            if l != OCEAN {
                if l != NO_DEP {
                    return Err(Error::InvalidArgument(
                        "Label array given to get_depression_hierarchy must contain only NO_DEP and OCEAN labels!".into(),
                    ));
                }
                continue;
            }
            let mut has_non_ocean = false;
            for n in 1..=neighbours {
                let nx = x as i32 + dx[n];
                let ny = y as i32 + dy[n];
                if label.in_grid(nx, ny) && label.get_xy(nx, ny) != OCEAN {
                    has_non_ocean = true;
                    break;
                }
            }
            if has_non_ocean {
                ocean_seeds.push(dem.xy_to_i(x, y) as FlatIdx);
            }
        }
    }

    if ocean_seeds.is_empty() {
        return Err(Error::InvalidArgument(
            "No OCEAN cells found, could not make a DepressionHierarchy!".into(),
        ));
    }

    // Depression 0 is always the ocean.
    depressions.push(Depression {
        pit_elev: T::NEG_INFINITY,
        pit_cell: NO_VALUE,
        dep_label: OCEAN,
        ..Default::default()
    });

    // Find pit cells (cells without any strictly lower neighbor).
    for y in 0..h {
        for x in 0..w {
            if label.get(x, y) == OCEAN {
                continue;
            }
            let my_elev = dem.get(x, y);
            let mut has_lower = false;
            for n in 1..=neighbours {
                let nx = x as i32 + dx[n];
                let ny = y as i32 + dy[n];
                if !dem.in_grid(nx, ny) {
                    continue;
                }
                if dem.get_xy(nx, ny).rd_cmp(my_elev) == Ordering::Less {
                    has_lower = true;
                    break;
                }
            }
            if !has_lower {
                land_seeds.push(dem.xy_to_i(x, y) as FlatIdx);
            }
        }
    }

    ocean_seeds.sort_unstable();
    land_seeds.sort_unstable();

    let mut pq: BinaryHeap<KeyValEntry<T, FlatIdx>> =
        BinaryHeap::with_capacity(ocean_seeds.len() + land_seeds.len());
    let mut seq: u64 = 0;

    for x in ocean_seeds {
        seq += 1;
        pq.push(KeyValEntry {
            key: dem.get_i(x as usize),
            value: x,
            seq,
        });
    }
    let pit_cell_count = land_seeds.len();
    for x in land_seeds {
        seq += 1;
        pq.push(KeyValEntry {
            key: dem.get_i(x as usize),
            value: x,
            seq,
        });
    }

    let mut outlet_database: HashMap<(DhLabel, DhLabel), Outlet<T>> =
        HashMap::with_capacity(3 * (pit_cell_count + 1));

    while let Some(entry) = pq.pop() {
        let ci = entry.value;
        let celev = entry.key;
        let mut clabel = label.get_i(ci as usize);
        let (cx, cy) = dem.i_to_xy(ci as usize);

        if clabel == NO_DEP {
            clabel = depressions.len() as DhLabel;
            depressions.push(Depression {
                pit_cell: ci,
                pit_elev: celev,
                dep_label: clabel,
                ..Default::default()
            });
            label.set_i(ci as usize, clabel);
        }

        for n in 1..=neighbours {
            let nx = cx as i32 + dx[n];
            let ny = cy as i32 + dy[n];
            if !dem.in_grid(nx, ny) {
                continue;
            }
            let ni = dem.xy_to_i(nx as usize, ny as usize) as FlatIdx;
            let nlabel = label.get_i(ni as usize);

            if nlabel == NO_DEP {
                label.set_i(ni as usize, clabel);
                seq += 1;
                pq.push(KeyValEntry {
                    key: dem.get_i(ni as usize),
                    value: ni,
                    seq,
                });
                flowdirs.set_xy(nx, ny, dinverse[n] as Flowdir);
            } else if nlabel != clabel {
                let mut out_cell = ci;
                let mut out_elev = celev;
                let ne = dem.get_i(ni as usize);
                if ne.rd_cmp(out_elev) == Ordering::Greater {
                    out_cell = ni;
                    out_elev = ne;
                }

                let olink = outlet_key(clabel, nlabel);
                outlet_database
                    .entry(olink)
                    .and_modify(|outlet| {
                        if outlet.out_elev.rd_cmp(out_elev) == Ordering::Greater {
                            outlet.out_cell = out_cell;
                            outlet.out_elev = out_elev;
                        }
                    })
                    .or_insert_with(|| Outlet::new(clabel, nlabel, out_cell, out_elev));
            }
        }
    }

    // Sort outlets from lowest to highest elevation (with deterministic tie-breaking).
    let mut outlets: Vec<Outlet<T>> = outlet_database.into_values().collect();
    outlets.sort_by(|a, b| {
        a.out_elev
            .rd_cmp(b.out_elev)
            .then_with(|| a.depa.cmp(&b.depa))
            .then_with(|| a.depb.cmp(&b.depb))
    });

    let mut djset = DisjointDenseIntSet::with_size(depressions.len());

    for mut outlet in outlets {
        let mut depa_set = djset.find_set(outlet.depa);
        let mut depb_set = djset.find_set(outlet.depb);

        if depa_set == depb_set {
            continue;
        }

        if depa_set == OCEAN || depb_set == OCEAN {
            if depa_set == OCEAN {
                std::mem::swap(&mut outlet.depa, &mut outlet.depb);
                std::mem::swap(&mut depa_set, &mut depb_set);
            }

            {
                let dep = &mut depressions[depa_set as usize];
                debug_assert_eq!(dep.out_cell, NO_VALUE);
                debug_assert_eq!(dep.odep, NO_VALUE);
                dep.parent = outlet.depb;
                dep.out_elev = outlet.out_elev;
                dep.out_cell = outlet.out_cell;
                dep.odep = NO_VALUE;
                dep.ocean_parent = true;
                dep.geolink = outlet.depb;
            }
            depressions[outlet.depb as usize]
                .ocean_linked
                .push(depa_set);
            djset.merge_a_into_b(depa_set, OCEAN);
        } else {
            let newlabel = depressions.len() as DhLabel;
            let depa_pit_cell = depressions[depa_set as usize].pit_cell;

            {
                let depa = &mut depressions[depa_set as usize];
                debug_assert_eq!(depa.odep, NO_VALUE);
                depa.parent = newlabel;
                depa.out_cell = outlet.out_cell;
                depa.out_elev = outlet.out_elev;
                depa.odep = depb_set;
                depa.geolink = outlet.depb;
            }
            {
                let depb = &mut depressions[depb_set as usize];
                debug_assert_eq!(depb.odep, NO_VALUE);
                depb.parent = newlabel;
                depb.out_cell = outlet.out_cell;
                depb.out_elev = outlet.out_elev;
                depb.odep = depa_set;
                depb.geolink = outlet.depa;
            }

            depressions.push(Depression {
                lchild: depa_set,
                rchild: depb_set,
                dep_label: newlabel,
                pit_cell: depa_pit_cell,
                ..Default::default()
            });

            djset.merge_a_into_b(depa_set, newlabel);
            djset.merge_a_into_b(depb_set, newlabel);
        }
    }

    calculate_marginal_volumes(&mut depressions, dem, label);
    calculate_total_volumes(&mut depressions);

    Ok(depressions)
}

struct CachingOutletChecker<T: DemNum> {
    previous_leaf_label: DhLabel,
    outlets: Vec<(T, DhLabel)>,
}

impl<T: DemNum> CachingOutletChecker<T> {
    fn new() -> Self {
        Self {
            previous_leaf_label: NO_DEP,
            outlets: Vec::new(),
        }
    }

    fn reset(&mut self, leaf_label: DhLabel) {
        self.previous_leaf_label = leaf_label;
        self.outlets.clear();
    }

    fn record(&mut self, this_label: DhLabel, this_elev: T) {
        self.outlets.push((this_elev, this_label));
    }

    fn find_start(&mut self, leaf_label: DhLabel, this_elev: T) -> DhLabel {
        if self.previous_leaf_label != leaf_label {
            self.reset(leaf_label);
            return leaf_label;
        }
        while let Some(&(out_elev, _)) = self.outlets.last() {
            if this_elev.rd_cmp(out_elev) != Ordering::Greater {
                self.outlets.pop();
            } else {
                break;
            }
        }
        if let Some(&(_, lbl)) = self.outlets.last() {
            return lbl;
        }
        self.reset(leaf_label);
        leaf_label
    }
}

/// Calculates marginal cell counts and elevation sums for every node in `deps`.
pub fn calculate_marginal_volumes<T: DemNum>(
    deps: &mut DepressionHierarchy<T>,
    dem: &Array2D<T>,
    label: &Array2D<DhLabel>,
) {
    let mut cell_counts = vec![0u32; deps.len()];
    let mut total_elevations = vec![0.0f64; deps.len()];
    let mut coc = CachingOutletChecker::<T>::new();

    for i in 0..dem.size() {
        let my_elev = dem.get_i(i);
        let mut clabel = coc.find_start(label.get_i(i), my_elev);

        while clabel != OCEAN {
            let this_out_elev = deps[clabel as usize].out_elev;
            coc.record(clabel, this_out_elev);

            if my_elev.rd_cmp(this_out_elev) != Ordering::Greater {
                break;
            }
            if deps[clabel as usize].ocean_parent {
                clabel = OCEAN;
                break;
            }
            clabel = deps[clabel as usize].parent;
        }

        if clabel == OCEAN {
            continue;
        }
        cell_counts[clabel as usize] += 1;
        total_elevations[clabel as usize] += my_elev.to_f64();
    }

    for (d, dep) in deps.iter_mut().enumerate() {
        dep.cell_count += cell_counts[d];
        dep.total_elevation += total_elevations[d];
    }
}

/// Accumulates total cell counts, total elevations, and depression storage volumes bottom-up.
pub fn calculate_total_volumes<T: DemNum>(deps: &mut DepressionHierarchy<T>) {
    for d in 0..deps.len() {
        let lchild = deps[d].lchild;
        let rchild = deps[d].rchild;
        if lchild != NO_VALUE {
            debug_assert_ne!(rchild, NO_VALUE);
            debug_assert!((lchild as usize) < d);
            debug_assert!((rchild as usize) < d);
            let lc_count = deps[lchild as usize].cell_count;
            let lc_elev = deps[lchild as usize].total_elevation;
            let rc_count = deps[rchild as usize].cell_count;
            let rc_elev = deps[rchild as usize].total_elevation;
            deps[d].cell_count += lc_count + rc_count;
            deps[d].total_elevation += lc_elev + rc_elev;
        }
        let dep = &mut deps[d];
        dep.dep_vol = (dep.cell_count as f64) * dep.out_elev.to_f64() - dep.total_elevation;

        if lchild != NO_VALUE {
            let child_sum = deps[lchild as usize].dep_vol + deps[rchild as usize].dep_vol;
            debug_assert!(fp_le(child_sum, deps[d].dep_vol));
        }
    }
}

/// Relabels cells according to whether they lie below their containing depression's outlet.
pub fn last_layer<T: DemNum>(
    label: &mut Array2D<DhLabel>,
    dem: &Array2D<T>,
    depressions: &DepressionHierarchy<T>,
) {
    for y in 0..label.height() {
        for x in 0..label.width() {
            let mut mylabel = label.get(x, y);
            loop {
                if dem.get(x, y).rd_cmp(depressions[mylabel as usize].out_elev) != Ordering::Less {
                    mylabel = depressions[mylabel as usize].parent;
                } else {
                    if mylabel != OCEAN {
                        mylabel = (-3i32) as DhLabel;
                    }
                    break;
                }
            }
            label.set(x, y, mylabel);
        }
    }
}

/// Formats a [`DepressionHierarchy`] as an ASCII tree, matching RichDEM's `operator<<`.
pub fn format_depression_hierarchy<T: DemNum>(deps: &DepressionHierarchy<T>) -> String {
    fn helper<T: DemNum>(
        deps: &DepressionHierarchy<T>,
        root: usize,
        depth: usize,
        child_count: &mut Vec<usize>,
        out: &mut String,
    ) {
        let dep = &deps[root];
        let count = usize::from(dep.lchild != NO_VALUE)
            + usize::from(dep.rchild != NO_VALUE)
            + dep.ocean_linked.len();
        child_count.push(count);

        for (i, &cc) in child_count.iter().take(depth).enumerate() {
            if cc > 1 && i == depth - 1 {
                out.push_str(if dep.ocean_parent { "╠═" } else { "├─" });
            } else if cc == 1 && i == depth - 1 {
                out.push_str(if dep.ocean_parent { "╚═" } else { "└─" });
            } else if cc > 2 {
                out.push_str("║ ");
            } else if cc > 1 {
                out.push_str("│ ");
            } else {
                out.push_str("  ");
            }
        }

        let _ = writeln!(
            out,
            "Id={root}, dep_vol={}, water_vol={}, pit_cell={}, out_cell={}, out_elev={}, parent={}, odep={}",
            dep.dep_vol,
            dep.water_vol,
            dep.pit_cell,
            dep.out_cell,
            dep.out_elev,
            dep.parent,
            dep.odep
        );

        if dep.lchild != NO_VALUE {
            helper(deps, dep.lchild as usize, depth + 1, child_count, out);
            *child_count.last_mut().unwrap() -= 1;
        }
        if dep.rchild != NO_VALUE {
            helper(deps, dep.rchild as usize, depth + 1, child_count, out);
            *child_count.last_mut().unwrap() -= 1;
        }
        for &x in &dep.ocean_linked {
            helper(deps, x as usize, depth + 1, child_count, out);
            *child_count.last_mut().unwrap() -= 1;
        }
        child_count.pop();
    }

    let mut out = String::new();
    if !deps.is_empty() {
        let mut child_count = Vec::new();
        helper(deps, 0, 0, &mut child_count, &mut out);
    }
    out
}
