//! Depression filling, depression breaching, depression hierarchies, and Fill-Spill-Merge.

pub mod barnes2014;
pub mod fill_spill_merge;
pub mod hierarchy;
pub mod lindsay2016;
pub mod wei2018;
pub mod zhou2016;

pub use barnes2014::{
    has_depressions, pit_mask, priority_flood_barnes2014, priority_flood_barnes2014_max_dep,
    priority_flood_epsilon_barnes2014, priority_flood_flowdirs_barnes2014, priority_flood_original,
    priority_flood_watersheds_barnes2014,
};
pub use fill_spill_merge::{
    backfill_depression, depression_volume, determine_water_level, fill_depressions_fsm,
    fill_spill_merge, move_water_into_pits, reset_dh,
};
pub use hierarchy::{
    Depression, DepressionHierarchy, DhLabel, FlatIdx, NO_DEP, NO_PARENT, NO_VALUE, OCEAN,
    calculate_marginal_volumes, calculate_total_volumes, format_depression_hierarchy,
    get_depression_hierarchy, get_new_depression_hierarchy_labels, last_layer,
};
pub use lindsay2016::{LindsayMode, complete_breaching_lindsay2016, lindsay2016};
pub use wei2018::priority_flood_wei2018;
pub use zhou2016::priority_flood_zhou2016;

use crate::array2d::Array2D;
use crate::constants::Topology;
use crate::error::Result;
use crate::num::DemNum;

/// Fills all depressions in `dem` in place using the fastest algorithm for `topo`
/// (Zhou 2016 for D8, Barnes 2014 for D4).
pub fn fill_depressions<T: DemNum>(dem: &mut Array2D<T>, topo: Topology) {
    match topo {
        Topology::D8 => priority_flood_zhou2016(dem),
        Topology::D4 => priority_flood_barnes2014(dem, Topology::D4),
    }
}

/// Fills all depressions in `dem` in place and enforces an epsilon downhill gradient
/// so every cell has a defined drainage direction (floating-point DEMs only).
pub fn fill_depressions_epsilon<T: DemNum>(dem: &mut Array2D<T>, topo: Topology) -> Result<usize> {
    priority_flood_epsilon_barnes2014(dem, topo)
}

/// Breaches all depressions in `dem` in place using Lindsay (2016) complete breaching.
pub fn breach_depressions<T: DemNum>(dem: &mut Array2D<T>, topo: Topology) {
    complete_breaching_lindsay2016(dem, topo);
}
