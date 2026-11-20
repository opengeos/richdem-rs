//! Flow direction and flow proportion (flow metric) algorithms.

pub mod d8_flowdirs;
pub mod dinf_flowdirs;
pub mod fairfield1991;
pub mod freeman1991;
pub mod holmgren1994;
pub mod ocallaghan1984;
pub mod quinn1991;
pub mod tarboton1997;

pub use d8_flowdirs::d8_flow_directions;
pub use dinf_flowdirs::dinf_flow_directions;
pub use fairfield1991::{fm_fairfield_leymarie, fm_rho4, fm_rho8};
pub use freeman1991::fm_freeman;
pub use holmgren1994::fm_holmgren;
pub use ocallaghan1984::{fm_d4, fm_d8, fm_ocallaghan};
pub use quinn1991::fm_quinn;
pub use tarboton1997::{fm_dinfinity, fm_tarboton};
