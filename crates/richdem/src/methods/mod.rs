//! Flow accumulation, upslope area, and terrain attribute algorithms.

pub mod d8_methods;
pub mod dinf_methods;
pub mod flow_accumulation;
pub mod flow_accumulation_generic;
pub mod terrain_attributes;

pub use d8_methods::{d8_flow_accum, d8_upslope_cells};
pub use dinf_methods::dinf_upslope_area;
pub use flow_accumulation::{
    FlowAccumMethod, fa_d4, fa_d8, fa_dinfinity, fa_fairfield_leymarie_d4,
    fa_fairfield_leymarie_d8, fa_freeman, fa_holmgren, fa_ocallaghan_d4, fa_ocallaghan_d8,
    fa_quinn, fa_rho4, fa_rho8, fa_tarboton, flow_accumulation_by_method,
    flow_accumulation_from_d8, flow_proportions_by_method,
};
pub use flow_accumulation_generic::flow_accumulation;
pub use terrain_attributes::{
    TerrainAttr, ta_aspect, ta_cti, ta_curvature, ta_planform_curvature, ta_profile_curvature,
    ta_slope_degrees, ta_slope_percentage, ta_slope_radians, ta_slope_riserun, ta_spi,
    terrain_attribute,
};
