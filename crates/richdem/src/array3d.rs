//! 3D array storing 9 flow-proportion values per cell (`width x height x 9`),
//! matching RichDEM's `Array3D<T>`.

use std::collections::BTreeMap;

use crate::array2d::{Array2D, STANDARD_GEOTRANSFORM};
use crate::error::{Error, Result};
use crate::num::DemNum;

/// Number of flow-proportion slots per cell (slot 0 is status flag, slots 1..=8 are D8 neighbors).
pub const FLOW_PROPS_DEPTH: usize = 9;

/// A `(height, width, 9)` row-major 3D array used for generic flow proportions.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(
    feature = "serde",
    derive(serde::Serialize, serde::Deserialize),
    serde(try_from = "Array3DWire<T>")
)]
pub struct Array3D<T: DemNum> {
    width: usize,
    height: usize,
    data: Vec<T>,
    no_data: T,
    /// GDAL-style 6-element geotransform.
    pub geotransform: [f64; 6],
    /// CRS projection string.
    pub projection: String,
    /// Raster metadata tags.
    pub metadata: BTreeMap<String, String>,
}

#[cfg(feature = "serde")]
#[derive(serde::Deserialize)]
struct Array3DWire<T> {
    width: usize,
    height: usize,
    data: Vec<T>,
    no_data: T,
    geotransform: [f64; 6],
    projection: String,
    metadata: BTreeMap<String, String>,
}

#[cfg(feature = "serde")]
impl<T: DemNum> TryFrom<Array3DWire<T>> for Array3D<T> {
    type Error = Error;

    fn try_from(w: Array3DWire<T>) -> Result<Self> {
        let mut arr = Array3D::from_vec(w.width, w.height, w.data)?;
        arr.no_data = w.no_data;
        arr.geotransform = w.geotransform;
        arr.projection = w.projection;
        arr.metadata = w.metadata;
        Ok(arr)
    }
}

impl<T: DemNum> Default for Array3D<T> {
    fn default() -> Self {
        Self {
            width: 0,
            height: 0,
            data: Vec::new(),
            no_data: T::ZERO,
            geotransform: STANDARD_GEOTRANSFORM,
            projection: String::new(),
            metadata: BTreeMap::new(),
        }
    }
}

impl<T: DemNum> Array3D<T> {
    /// Creates a `width x height x 9` array filled with `fill`.
    pub fn new(width: usize, height: usize, fill: T) -> Self {
        let total = width
            .checked_mul(height)
            .and_then(|c| c.checked_mul(FLOW_PROPS_DEPTH))
            .expect("Array3D dimensions overflow usize");
        Self {
            width,
            height,
            data: vec![fill; total],
            no_data: T::ZERO,
            geotransform: STANDARD_GEOTRANSFORM,
            projection: String::new(),
            metadata: BTreeMap::new(),
        }
    }

    /// Creates a `width x height x 9` array matching `template`'s dimensions and metadata.
    pub fn make_from_template<U: DemNum>(template: &Array2D<U>, fill: T) -> Self {
        let total = template
            .width()
            .checked_mul(template.height())
            .and_then(|c| c.checked_mul(FLOW_PROPS_DEPTH))
            .expect("Array3D dimensions overflow usize");
        Self {
            width: template.width(),
            height: template.height(),
            data: vec![fill; total],
            no_data: T::ZERO,
            geotransform: template.geotransform,
            projection: template.projection.clone(),
            metadata: template.metadata.clone(),
        }
    }

    /// Creates an `Array3D` from a flat buffer of length `width * height * 9`.
    pub fn from_vec(width: usize, height: usize, data: Vec<T>) -> Result<Self> {
        let expected = width
            .checked_mul(height)
            .and_then(|c| c.checked_mul(FLOW_PROPS_DEPTH))
            .ok_or_else(|| {
                Error::DimensionMismatch(format!(
                    "Array3D dimensions overflow usize ({width}x{height}x{FLOW_PROPS_DEPTH})"
                ))
            })?;
        if data.len() != expected {
            return Err(Error::DimensionMismatch(format!(
                "expected {expected} elements ({width}x{height}x9), got {}",
                data.len()
            )));
        }
        Ok(Self {
            width,
            height,
            data,
            no_data: T::ZERO,
            geotransform: STANDARD_GEOTRANSFORM,
            projection: String::new(),
            metadata: BTreeMap::new(),
        })
    }

    /// Width in cells.
    #[inline]
    pub fn width(&self) -> usize {
        self.width
    }

    /// Height in cells.
    #[inline]
    pub fn height(&self) -> usize {
        self.height
    }

    /// Number of 2D cells (`width * height`).
    #[inline]
    pub fn size(&self) -> usize {
        self.width * self.height
    }

    /// Returns `true` if empty.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.data.is_empty()
    }

    /// Underlying flat slice of length `width * height * 9`.
    #[inline]
    pub fn data(&self) -> &[T] {
        &self.data
    }

    /// Underlying mutable flat slice of length `width * height * 9`.
    #[inline]
    pub fn data_mut(&mut self) -> &mut [T] {
        &mut self.data
    }

    /// Consumes the array and returns its flat storage vector.
    #[inline]
    pub fn into_vec(self) -> Vec<T> {
        self.data
    }

    /// Current NoData sentinel value.
    #[inline]
    pub fn no_data(&self) -> T {
        self.no_data
    }

    /// Sets the NoData sentinel value.
    #[inline]
    pub fn set_no_data(&mut self, val: T) {
        self.no_data = val;
    }

    /// Sets every entry `(x, y, n)` to `val`.
    pub fn set_all(&mut self, val: T) {
        self.data.fill(val);
    }

    /// Gets `(x, y, n)` where `n in 0..9`.
    #[inline]
    pub fn get(&self, x: usize, y: usize, n: usize) -> T {
        self.data[(y * self.width + x) * FLOW_PROPS_DEPTH + n]
    }

    /// Sets `(x, y, n)` where `n in 0..9`.
    #[inline]
    pub fn set(&mut self, x: usize, y: usize, n: usize, val: T) {
        self.data[(y * self.width + x) * FLOW_PROPS_DEPTH + n] = val;
    }

    /// Gets slot `n` (`0..9`) for flat 2D cell index `i`.
    #[inline]
    pub fn get_in(&self, i: usize, n: usize) -> T {
        self.data[i * FLOW_PROPS_DEPTH + n]
    }

    /// Sets slot `n` (`0..9`) for flat 2D cell index `i`.
    #[inline]
    pub fn set_in(&mut self, i: usize, n: usize, val: T) {
        self.data[i * FLOW_PROPS_DEPTH + n] = val;
    }

    /// Returns `true` if cell `(x, y)` is marked NoData (slot 0 == `no_data`).
    #[inline]
    pub fn is_no_data(&self, x: usize, y: usize) -> bool {
        self.get(x, y, 0).rd_eq(self.no_data)
    }

    /// Returns `true` if flat cell `i` is marked NoData (slot 0 == `no_data`).
    #[inline]
    pub fn is_no_data_i(&self, i: usize) -> bool {
        self.get_in(i, 0).rd_eq(self.no_data)
    }
}
