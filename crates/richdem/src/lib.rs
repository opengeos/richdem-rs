//! # richdem-rs
//!
//! High-performance Digital Elevation Model (DEM) hydrologic and terrain analysis
//! library in pure Rust, ported from Richard Barnes's
//! [RichDEM](https://github.com/r-barnes/richdem) and powered by
//! [`terrio`](https://github.com/opengeos/terrio) for GDAL-free geospatial raster I/O.
//!
//! ## Features
//!
//! - **Depression handling:** Priority-Flood (Barnes 2014, Zhou 2016, Wei 2018),
//!   Priority-Flood+Epsilon, depression breaching (Lindsay 2016: complete, selective,
//!   constrained), pit masks, watershed labeling, [`DepressionHierarchy`](depressions::DepressionHierarchy),
//!   and [`fill_spill_merge`](depressions::fill_spill_merge).
//! - **Flat resolution:** Barnes (2014) flat masks, epsilon flat resolution,
//!   D8/D-infinity flat flow resolution, and Garbrecht & Martz (1997).
//! - **Flow metrics & accumulation:** D8/D4 (O'Callaghan & Mark 1984),
//!   Rho8/Rho4 (Fairfield & Leymarie 1991), D-infinity (Tarboton 1997),
//!   Quinn (1991), Holmgren (1994), Freeman (1991), plus direct D8/D-infinity accumulation.
//! - **Terrain attributes:** Slope (rise/run, percentage, degrees, radians), aspect,
//!   total/planform/profile curvature, SPI, CTI, surface area (Jenness 2004), and perimeter.
//! - **Pure-Rust I/O:** Reads and writes GeoTIFF, COG, AAIGrid, ENVI, HFA, VRT, and more
//!   via `terrio` without installing GDAL.
//!
//! ## Credits
//!
//! All credit for the original library design, algorithms, and test suite goes to
//! Richard Barnes (<https://github.com/r-barnes/richdem>) and the original publication
//! authors. Licensed under GPL-3.0-only.

pub mod array2d;
pub mod array3d;
pub mod constants;
pub mod depressions;
pub mod disjoint_set;
pub mod error;
pub mod flats;
pub mod flowmet;
pub mod grid_cell;
#[cfg(feature = "terrio")]
pub mod io;
pub mod methods;
pub mod misc;
pub mod num;
pub mod random;
pub mod terrain_gen;

pub use array2d::{Array2D, DynamicArray2D, STANDARD_GEOTRANSFORM};
pub use array3d::{Array3D, FLOW_PROPS_DEPTH};
pub use constants::{
    ACCUM_NO_DATA, D4_EAST, D4_INVERSE, D4_NORTH, D4_SOUTH, D4_WEST, D4R, D4X, D4Y, D8_ARCGIS,
    D8_EAST, D8_INVERSE, D8_NORTH, D8_SOUTH, D8_WEST, D8Flowdir, D8R, D8X, D8Y, DINF_NO_DATA,
    DINF_NO_FLOW, DR, FLOWDIR_NO_DATA, Flowdir, HAS_FLOW_GEN, N4_DIAG, N8_DIAG, NO_DATA_GEN,
    NO_FLOW, NO_FLOW_GEN, RICHDEM_FP_COMPARISON_ERROR, SQRT2, Topology, fp_eq, fp_ge, fp_le,
};
pub use depressions::{
    Depression, DepressionHierarchy, DhLabel, FlatIdx, LindsayMode, NO_DEP, NO_PARENT, NO_VALUE,
    OCEAN, backfill_depression, breach_depressions, calculate_marginal_volumes,
    calculate_total_volumes, complete_breaching_lindsay2016, depression_volume,
    determine_water_level, fill_depressions, fill_depressions_epsilon, fill_depressions_fsm,
    fill_spill_merge, format_depression_hierarchy, get_depression_hierarchy,
    get_new_depression_hierarchy_labels, has_depressions, last_layer, lindsay2016,
    move_water_into_pits, pit_mask, priority_flood_barnes2014, priority_flood_barnes2014_max_dep,
    priority_flood_epsilon_barnes2014, priority_flood_flowdirs_barnes2014, priority_flood_original,
    priority_flood_watersheds_barnes2014, priority_flood_wei2018, priority_flood_zhou2016,
    reset_dh,
};
pub use disjoint_set::DisjointDenseIntSet;
pub use error::{Error, Result};
pub use flats::{
    FLAT_NO_DATA, IS_A_FLAT, NOT_A_FLAT, barnes_flat_resolution_d8, d8_flats_alter_dem,
    d8_flow_flats, dinf_flow_flats, find_flats, garbrecht_alg, get_flat_mask, resolve_flats_barnes,
    resolve_flats_barnes_dinf, resolve_flats_epsilon, resolve_flats_epsilon_barnes2014,
    resolve_flats_flowdirs_barnes2014,
};
pub use flowmet::{
    d8_flow_directions, dinf_flow_directions, fm_d4, fm_d8, fm_dinfinity, fm_fairfield_leymarie,
    fm_freeman, fm_holmgren, fm_ocallaghan, fm_quinn, fm_rho4, fm_rho8, fm_tarboton,
};
pub use grid_cell::{
    GridCell, GridCellZ, GridCellZPq, GridCellZkHigh, GridCellZkHighPq, GridCellZkLow,
    GridCellZkLowPq,
};
#[cfg(feature = "terrio")]
pub use io::{
    array2d_to_dataset, dataset_to_dynamic, load_dynamic, load_raster, load_raster_padded,
    save_dynamic, save_raster,
};
pub use methods::{
    FlowAccumMethod, TerrainAttr, d8_flow_accum, d8_upslope_cells, dinf_upslope_area, fa_d4, fa_d8,
    fa_dinfinity, fa_fairfield_leymarie_d4, fa_fairfield_leymarie_d8, fa_freeman, fa_holmgren,
    fa_ocallaghan_d4, fa_ocallaghan_d8, fa_quinn, fa_rho4, fa_rho8, fa_tarboton, flow_accumulation,
    flow_accumulation_by_method, flow_accumulation_from_d8, flow_proportions_by_method, ta_aspect,
    ta_cti, ta_curvature, ta_planform_curvature, ta_profile_curvature, ta_slope_degrees,
    ta_slope_percentage, ta_slope_radians, ta_slope_riserun, ta_spi, terrain_attribute,
};
pub use misc::{
    PerimType, bucket_fill, bucket_fill_from_edges, convert_arc_flowdirs_to_richdem_d8,
    convert_taudem_d8_to_richdem_d8, dem_surface_area, find_flow_loops, perimeter,
};
pub use num::DemNum;
pub use random::{seed_rand, uniform_rand_int, uniform_rand_real};
pub use terrain_gen::{
    PerlinNoise, generate_perlin_terrain, generate_perlin_terrain_into, try_generate_perlin_terrain,
};

/// Crate version string.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
