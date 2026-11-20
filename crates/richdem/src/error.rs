//! Error types for RichDEM operations.

use thiserror::Error;

/// Result alias for RichDEM operations.
pub type Result<T> = std::result::Result<T, Error>;

/// Errors produced by RichDEM algorithms and I/O routines.
#[derive(Debug, Error)]
pub enum Error {
    /// Input arrays have mismatched dimensions.
    #[error("dimension mismatch: {0}")]
    DimensionMismatch(String),

    /// Invalid argument or unsupported configuration.
    #[error("invalid argument: {0}")]
    InvalidArgument(String),

    /// Operation is only valid for floating-point elevation types.
    #[error("unsupported data type: {0}")]
    UnsupportedDataType(String),

    /// Algorithm invariant or precondition failure.
    #[error("algorithm error: {0}")]
    Algorithm(String),

    /// Underlying standard I/O error.
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    /// Parsing error (e.g., ASCII grid header or numeric value).
    #[error("parse error: {0}")]
    Parse(String),

    /// Geospatial raster I/O error from `terrio`.
    #[cfg(feature = "terrio")]
    #[error("terrio error: {0}")]
    Terrio(#[from] terrio::Error),
}
