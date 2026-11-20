//! Miscellaneous DEM utilities and flow direction format conversions.

pub mod conversion;
pub mod misc_methods;

pub use conversion::{convert_arc_flowdirs_to_richdem_d8, convert_taudem_d8_to_richdem_d8};
pub use misc_methods::{
    PerimType, bucket_fill, bucket_fill_from_edges, dem_surface_area, find_flow_loops, perimeter,
};
