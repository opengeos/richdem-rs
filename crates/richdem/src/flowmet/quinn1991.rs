//! Quinn et al. (1991) multiple-flow-direction proportions (Holmgren with exponent = 1.0).
//!
//! Reference: Quinn, P., Beven, K., Chevallier, P., Planchon, O., 1991.
//! The prediction of hillslope flow paths for distributed hydrological modelling
//! using digital terrain models. Hydrological Processes 5, 59-79.

use crate::array2d::Array2D;
use crate::array3d::Array3D;
use crate::flowmet::holmgren1994::fm_holmgren;
use crate::num::DemNum;

/// Computes Quinn (1991) flow proportions (`fm_holmgren` with `xparam = 1.0`).
pub fn fm_quinn<T: DemNum>(elevations: &Array2D<T>) -> Array3D<f32> {
    fm_holmgren(elevations, 1.0)
}
