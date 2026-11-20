//! Numeric element trait for RichDEM rasters.

use std::cmp::Ordering;
use std::fmt::{Debug, Display};
use std::ops::{Add, AddAssign, Sub, SubAssign};
use std::str::FromStr;

/// Trait implemented for all numeric pixel types supported by RichDEM
/// (`u8`, `i8`, `u16`, `i16`, `u32`, `i32`, `u64`, `i64`, `f32`, `f64`).
pub trait DemNum:
    Copy
    + Default
    + PartialEq
    + PartialOrd
    + Add<Output = Self>
    + AddAssign
    + Sub<Output = Self>
    + SubAssign
    + Debug
    + Display
    + FromStr
    + Send
    + Sync
    + 'static
{
    /// Whether this type is floating point (`f32` or `f64`).
    const IS_FLOAT: bool;
    /// Zero value.
    const ZERO: Self;
    /// One value.
    const ONE: Self;
    /// Maximum finite value.
    const MAX: Self;
    /// Minimum (lowest) finite value.
    const LOWEST: Self;
    /// Positive infinity for floats, [`Self::MAX`] for integers.
    const INFINITY: Self;
    /// Negative infinity for floats, [`Self::LOWEST`] for integers.
    const NEG_INFINITY: Self;

    /// Returns true if this value is NaN (always false for integers).
    fn is_nan(self) -> bool;

    /// Total ordering matching RichDEM's `GridCellZ`: NaNs are treated as
    /// infinitely small (`NaN < any finite value`, `NaN == NaN`).
    fn rd_cmp(self, other: Self) -> Ordering;

    /// Minimum of two values using [`Self::rd_cmp`].
    #[inline]
    fn rd_min(self, other: Self) -> Self {
        if self.rd_cmp(other) == Ordering::Less {
            self
        } else {
            other
        }
    }

    /// Maximum of two values using [`Self::rd_cmp`].
    #[inline]
    fn rd_max(self, other: Self) -> Self {
        if self.rd_cmp(other) == Ordering::Greater {
            self
        } else {
            other
        }
    }

    /// Equality where two NaNs compare equal (used for NoData comparison).
    #[inline]
    fn rd_eq(self, other: Self) -> bool {
        (self.is_nan() && other.is_nan()) || self == other
    }

    /// Next representable value toward `+infinity` (like `std::nextafter(x, +inf)`).
    /// Returns `None` for integer types.
    fn next_up_float(self) -> Option<Self>;

    /// Next representable value toward `-infinity` (like `std::nextafter(x, -inf)`).
    /// Returns `None` for integer types.
    fn next_down_float(self) -> Option<Self>;

    /// Convert to `f64`.
    fn to_f64(self) -> f64;

    /// Convert from `f64` (saturating/casting as in C++ `static_cast<T>`).
    fn from_f64(v: f64) -> Self;

    /// Convert to `i64`.
    fn to_i64(self) -> i64;

    /// Convert from `i64`.
    fn from_i64(v: i64) -> Self;

    /// Type name string for diagnostics and formatting.
    fn type_name() -> &'static str;
}

macro_rules! impl_int_dem_num {
    ($t:ty, $name:expr) => {
        impl DemNum for $t {
            const IS_FLOAT: bool = false;
            const ZERO: Self = 0;
            const ONE: Self = 1;
            const MAX: Self = <$t>::MAX;
            const LOWEST: Self = <$t>::MIN;
            const INFINITY: Self = <$t>::MAX;
            const NEG_INFINITY: Self = <$t>::MIN;

            #[inline]
            fn is_nan(self) -> bool {
                false
            }

            #[inline]
            fn rd_cmp(self, other: Self) -> Ordering {
                self.cmp(&other)
            }

            #[inline]
            fn next_up_float(self) -> Option<Self> {
                None
            }

            #[inline]
            fn next_down_float(self) -> Option<Self> {
                None
            }

            #[inline]
            fn to_f64(self) -> f64 {
                self as f64
            }

            #[inline]
            fn from_f64(v: f64) -> Self {
                if v < 0.0 { (v as i64) as $t } else { v as $t }
            }

            #[inline]
            fn to_i64(self) -> i64 {
                self as i64
            }

            #[inline]
            fn from_i64(v: i64) -> Self {
                v as $t
            }

            #[inline]
            fn type_name() -> &'static str {
                $name
            }
        }
    };
}

impl_int_dem_num!(u8, "uint8");
impl_int_dem_num!(i8, "int8");
impl_int_dem_num!(u16, "uint16");
impl_int_dem_num!(i16, "int16");
impl_int_dem_num!(u32, "uint32");
impl_int_dem_num!(i32, "int32");
impl_int_dem_num!(u64, "uint64");
impl_int_dem_num!(i64, "int64");

macro_rules! impl_float_dem_num {
    ($t:ty, $name:expr) => {
        impl DemNum for $t {
            const IS_FLOAT: bool = true;
            const ZERO: Self = 0.0;
            const ONE: Self = 1.0;
            const MAX: Self = <$t>::MAX;
            const LOWEST: Self = <$t>::MIN;
            const INFINITY: Self = <$t>::INFINITY;
            const NEG_INFINITY: Self = <$t>::NEG_INFINITY;

            #[inline]
            fn is_nan(self) -> bool {
                <$t>::is_nan(self)
            }

            #[inline]
            fn rd_cmp(self, other: Self) -> Ordering {
                match (self.is_nan(), other.is_nan()) {
                    (true, true) => Ordering::Equal,
                    (true, false) => Ordering::Less,
                    (false, true) => Ordering::Greater,
                    (false, false) => self.partial_cmp(&other).unwrap_or(Ordering::Equal),
                }
            }

            #[inline]
            fn next_up_float(self) -> Option<Self> {
                Some(self.next_up())
            }

            #[inline]
            fn next_down_float(self) -> Option<Self> {
                Some(self.next_down())
            }

            #[inline]
            fn to_f64(self) -> f64 {
                self as f64
            }

            #[inline]
            fn from_f64(v: f64) -> Self {
                v as $t
            }

            #[inline]
            fn to_i64(self) -> i64 {
                self as i64
            }

            #[inline]
            fn from_i64(v: i64) -> Self {
                v as $t
            }

            #[inline]
            fn type_name() -> &'static str {
                $name
            }
        }
    };
}

impl_float_dem_num!(f32, "float32");
impl_float_dem_num!(f64, "float64");
