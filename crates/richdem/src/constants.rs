//! Constants and neighborhood topology definitions used throughout RichDEM.
//!
//! RichDEM uses the following D8 neighborhood (used by [`D8X`] and [`D8Y`]):
//!
//! ```text
//! 2 3 4
//! 1 0 5
//! 8 7 6
//! ```
//!
//! ArcGIS uses the following bit flags for flow toward a neighbor:
//!
//! ```text
//!  32  64 128
//!  16   0   1
//!   8   4   2
//! ```
//!
//! D4 directions:
//!
//! ```text
//!   2
//! 1 0 3
//!   4
//! ```

/// `sqrt(2)`, used for diagonal neighbor distances.
pub const SQRT2: f64 = std::f64::consts::SQRT_2;

/// D8 flow direction storage type (`0..=8`, or [`FLOWDIR_NO_DATA`]).
pub type D8Flowdir = u8;

/// Signed flow direction storage type (used in DepressionHierarchy / Fill-Spill-Merge).
pub type Flowdir = i8;

/// X offsets of D8 neighbors from a central cell (index 0 is center, 1..=8 are neighbors).
pub const D8X: [i32; 9] = [0, -1, -1, 0, 1, 1, 1, 0, -1];
/// Y offsets of D8 neighbors from a central cell (index 0 is center, 1..=8 are neighbors).
pub const D8Y: [i32; 9] = [0, 0, -1, -1, -1, 0, 1, 1, 1];
/// Distances from a central cell to each of its D8 neighbors.
pub const D8R: [f64; 9] = [0.0, 1.0, SQRT2, 1.0, SQRT2, 1.0, SQRT2, 1.0, SQRT2];
/// Alias for [`D8R`] matching RichDEM's `dr`.
pub const DR: [f64; 9] = D8R;
/// True along diagonal D8 directions (2, 4, 6, 8), false along cardinal directions.
pub const N8_DIAG: [bool; 9] = [false, false, true, false, true, false, true, false, true];

/// D8 West neighbor index.
pub const D8_WEST: usize = 1;
/// D8 North neighbor index.
pub const D8_NORTH: usize = 3;
/// D8 East neighbor index.
pub const D8_EAST: usize = 5;
/// D8 South neighbor index.
pub const D8_SOUTH: usize = 7;

/// X offsets of D4 neighbors from a central cell (index 0 is center, 1..=4 are neighbors).
pub const D4X: [i32; 5] = [0, -1, 0, 1, 0];
/// Y offsets of D4 neighbors from a central cell (index 0 is center, 1..=4 are neighbors).
pub const D4Y: [i32; 5] = [0, 0, -1, 0, 1];
/// Distances from a central cell to each of its D4 neighbors.
pub const D4R: [f64; 5] = [0.0, 1.0, 1.0, 1.0, 1.0];
/// Diagonal flags for D4 neighbors (all false).
pub const N4_DIAG: [bool; 5] = [false, false, false, false, false];

/// D4 West neighbor index.
pub const D4_WEST: usize = 1;
/// D4 North neighbor index.
pub const D4_NORTH: usize = 2;
/// D4 East neighbor index.
pub const D4_EAST: usize = 3;
/// D4 South neighbor index.
pub const D4_SOUTH: usize = 4;

/// Inverse D8 direction leading from neighbor `n` back to the central cell.
pub const D8_INVERSE: [u8; 9] = [0, 5, 6, 7, 8, 1, 2, 3, 4];

/// Inverse D4 direction leading from neighbor `n` back to the central cell.
pub const D4_INVERSE: [u8; 5] = [0, 3, 4, 1, 2];

/// Convert from RichDEM D8 flow directions (`0..=8`) to ArcGIS D8 bit flags.
pub const D8_ARCGIS: [u8; 9] = [0, 16, 32, 64, 128, 1, 2, 4, 8];

/// Value indicating that a D8 flow direction cell is NoData.
pub const FLOWDIR_NO_DATA: u8 = 255;

/// Value indicating that a cell has no defined flow direction (flat or pit).
pub const NO_FLOW: i8 = 0;

/// Value indicating NoFlow in generic flow metric outputs (`Array3D<f32>`).
pub const NO_FLOW_GEN: f32 = -1.0;
/// Value indicating HasFlow in generic flow metric outputs (`Array3D<f32>`).
pub const HAS_FLOW_GEN: f32 = 0.0;
/// Value indicating NoData in generic flow metric outputs (`Array3D<f32>`).
pub const NO_DATA_GEN: f32 = -2.0;

/// Value indicating that a flow accumulation cell is NoData.
pub const ACCUM_NO_DATA: i32 = -1;

/// Value indicating that a D-infinity flow direction or upslope area cell is NoData.
pub const DINF_NO_DATA: f32 = -1.0;

/// Grid connectivity topology (8-neighbor or 4-neighbor).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Topology {
    /// 8-neighbor connectivity (cardinal + diagonal).
    D8,
    /// 4-neighbor connectivity (cardinal only).
    D4,
}

impl Topology {
    /// Neighbor X offsets (index 1..=nmax).
    #[inline]
    pub const fn dx(self) -> &'static [i32] {
        match self {
            Topology::D8 => &D8X,
            Topology::D4 => &D4X,
        }
    }

    /// Neighbor Y offsets (index 1..=nmax).
    #[inline]
    pub const fn dy(self) -> &'static [i32] {
        match self {
            Topology::D8 => &D8Y,
            Topology::D4 => &D4Y,
        }
    }

    /// Number of neighbors (8 for D8, 4 for D4).
    #[inline]
    pub const fn nmax(self) -> usize {
        match self {
            Topology::D8 => 8,
            Topology::D4 => 4,
        }
    }

    /// Inverse direction table.
    #[inline]
    pub const fn dinverse(self) -> &'static [u8] {
        match self {
            Topology::D8 => &D8_INVERSE,
            Topology::D4 => &D4_INVERSE,
        }
    }

    /// Diagonal indicator flags for neighbors.
    #[inline]
    pub const fn n_diag(self) -> &'static [bool] {
        match self {
            Topology::D8 => &N8_DIAG,
            Topology::D4 => &N4_DIAG,
        }
    }

    /// Human-readable topology name (`"D8"` or `"D4"`).
    #[inline]
    pub const fn name(self) -> &'static str {
        match self {
            Topology::D8 => "D8",
            Topology::D4 => "D4",
        }
    }
}

/// Floating-point comparison tolerance used in RichDEM (`1e-6`).
pub const RICHDEM_FP_COMPARISON_ERROR: f64 = 1e-6;

/// Returns `true` if `a <= b` within [`RICHDEM_FP_COMPARISON_ERROR`].
#[inline]
pub fn fp_le(a: f64, b: f64) -> bool {
    a < b || (a - b).abs() < RICHDEM_FP_COMPARISON_ERROR
}

/// Returns `true` if `a >= b` within [`RICHDEM_FP_COMPARISON_ERROR`].
#[inline]
pub fn fp_ge(a: f64, b: f64) -> bool {
    a > b || (a - b).abs() < RICHDEM_FP_COMPARISON_ERROR
}

/// Returns `true` if `a == b` within [`RICHDEM_FP_COMPARISON_ERROR`].
#[inline]
pub fn fp_eq(a: f64, b: f64) -> bool {
    (a - b).abs() < RICHDEM_FP_COMPARISON_ERROR
}
