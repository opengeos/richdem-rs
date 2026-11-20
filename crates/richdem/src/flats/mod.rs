//! Flat surface detection and resolution algorithms (Barnes et al., 2014; Garbrecht & Martz, 1997).

pub mod barnes2014;
pub mod find_flats;
pub mod flat_resolution;
pub mod flat_resolution_dinf;
pub mod garbrecht;

pub use barnes2014::{
    get_flat_mask, resolve_flats_epsilon_barnes2014, resolve_flats_flowdirs_barnes2014,
};
pub use find_flats::{FLAT_NO_DATA, IS_A_FLAT, NOT_A_FLAT, find_flats};
pub use flat_resolution::{
    barnes_flat_resolution_d8, d8_flats_alter_dem, d8_flow_flats, resolve_flats_barnes,
};
pub use flat_resolution_dinf::{dinf_flow_flats, resolve_flats_barnes_dinf};
pub use garbrecht::garbrecht_alg;

use crate::array2d::Array2D;
use crate::error::Result;
use crate::num::DemNum;

/// Resolves all drainable flat surfaces in `elevations` by imposing minimal epsilon
/// gradients (Barnes et al., 2014). Requires a floating-point DEM.
pub fn resolve_flats_epsilon<T: DemNum>(elevations: &mut Array2D<T>) -> Result<usize> {
    let (flat_mask, labels) = get_flat_mask(elevations);
    resolve_flats_epsilon_barnes2014(&flat_mask, &labels, elevations)
}
