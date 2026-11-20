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
    /// Whether this type is signed (`i8`, `i16`, `i32`, `i64`, `f32`, `f64`).
    const IS_SIGNED: bool;
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

    /// Convert from `f64` using Rust's saturating `as` conversion.
    fn from_f64(v: f64) -> Self;

    /// Convert a NoData sentinel from `f64`, preserving C++ RichDEM's unsigned
    /// wrap for negative sentinel values (e.g. `-1.0` -> `255u8`) while saturating
    /// signed integer and float targets.
    #[inline]
    fn from_nodata_f64(v: f64) -> Self {
        Self::from_f64(v)
    }

    /// Convert a NoData sentinel from exact `i128`, preserving the same unsigned
    /// negative-sentinel wrap as [`Self::from_nodata_f64`] while avoiding `f64`
    /// rounding for 64-bit integer sentinels.
    #[inline]
    fn from_nodata_i128(v: i128) -> Self {
        Self::from_i128_clamped(v)
    }

    /// Convert to `i64`.
    fn to_i64(self) -> i64;

    /// Convert from `i64`.
    fn from_i64(v: i64) -> Self;

    /// Convert to `u64`.
    fn to_u64(self) -> u64;

    /// Convert from `u64`.
    fn from_u64(v: u64) -> Self;

    /// Convert to `i128` losslessly for all integer types.
    fn to_i128(self) -> i128;

    /// Convert from `i128` with clamping to the target type's range (exact for integers).
    fn from_i128_clamped(v: i128) -> Self;

    /// Convert `self` to another `DemNum` type without passing integer-to-integer
    /// conversions through `f64`.
    #[inline]
    fn cast_to<U: DemNum>(self) -> U {
        if !Self::IS_FLOAT && !U::IS_FLOAT {
            U::from_i128_clamped(self.to_i128())
        } else {
            U::from_f64(self.to_f64())
        }
    }

    /// Default ESRI ASCII Grid NoData value for this pixel type (`-9999` when representable).
    fn esri_default_no_data() -> Self;

    /// Type name string for diagnostics and formatting.
    fn type_name() -> &'static str;
}

macro_rules! impl_signed_int_dem_num {
    ($t:ty, $name:expr, $esri_nd:expr) => {
        impl DemNum for $t {
            const IS_FLOAT: bool = false;
            const IS_SIGNED: bool = true;
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
            fn to_u64(self) -> u64 {
                self as u64
            }

            #[inline]
            fn from_u64(v: u64) -> Self {
                v as $t
            }

            #[inline]
            fn to_i128(self) -> i128 {
                self as i128
            }

            #[inline]
            fn from_i128_clamped(v: i128) -> Self {
                v.clamp(<$t>::MIN as i128, <$t>::MAX as i128) as $t
            }

            #[inline]
            fn esri_default_no_data() -> Self {
                $esri_nd
            }

            #[inline]
            fn type_name() -> &'static str {
                $name
            }
        }
    };
}

macro_rules! impl_unsigned_int_dem_num {
    ($t:ty, $name:expr) => {
        impl DemNum for $t {
            const IS_FLOAT: bool = false;
            const IS_SIGNED: bool = false;
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
                v as $t
            }

            #[inline]
            fn from_nodata_f64(v: f64) -> Self {
                if v < 0.0 { (v as i64) as $t } else { v as $t }
            }

            #[inline]
            fn from_nodata_i128(v: i128) -> Self {
                if v < 0 {
                    (v.clamp(i64::MIN as i128, i64::MAX as i128) as i64) as $t
                } else {
                    Self::from_i128_clamped(v)
                }
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
            fn to_u64(self) -> u64 {
                self as u64
            }

            #[inline]
            fn from_u64(v: u64) -> Self {
                v as $t
            }

            #[inline]
            fn to_i128(self) -> i128 {
                self as i128
            }

            #[inline]
            fn from_i128_clamped(v: i128) -> Self {
                v.clamp(<$t>::MIN as i128, <$t>::MAX as i128) as $t
            }

            #[inline]
            fn esri_default_no_data() -> Self {
                <$t>::MAX
            }

            #[inline]
            fn type_name() -> &'static str {
                $name
            }
        }
    };
}

impl_unsigned_int_dem_num!(u8, "uint8");
impl_signed_int_dem_num!(i8, "int8", i8::MIN);
impl_unsigned_int_dem_num!(u16, "uint16");
impl_signed_int_dem_num!(i16, "int16", -9999);
impl_unsigned_int_dem_num!(u32, "uint32");
impl_signed_int_dem_num!(i32, "int32", -9999);
impl_unsigned_int_dem_num!(u64, "uint64");
impl_signed_int_dem_num!(i64, "int64", -9999);

macro_rules! impl_float_dem_num {
    ($t:ty, $name:expr) => {
        impl DemNum for $t {
            const IS_FLOAT: bool = true;
            const IS_SIGNED: bool = true;
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
            fn to_u64(self) -> u64 {
                self as u64
            }

            #[inline]
            fn from_u64(v: u64) -> Self {
                v as $t
            }

            #[inline]
            fn to_i128(self) -> i128 {
                self as i128
            }

            #[inline]
            fn from_i128_clamped(v: i128) -> Self {
                v as $t
            }

            #[inline]
            fn esri_default_no_data() -> Self {
                -9999.0
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
