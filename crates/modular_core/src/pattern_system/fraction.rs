//! Exact rational number type for precise time representation.
//!
//! Uses i64/i64 rational numbers to avoid floating-point drift over time,
//! enabling precise subdivisions (triplets, quintuplets, etc.) and
//! exact cycle boundary computation.
//!
//! All values are stored in reduced form (GCD normalized) after each operation.

use std::cmp::Ordering;
use std::fmt;
use std::ops::{Add, Div, Mul, Neg, Sub};

/// GCD via Euclidean algorithm (always returns non-negative).
#[inline]
fn gcd(a: i64, b: i64) -> i64 {
    let mut a = a.abs();
    let mut b = b.abs();
    while b != 0 {
        let t = b;
        b = a % b;
        a = t;
    }
    a
}

/// Exact rational number for precise time representation.
///
/// Invariants maintained after every operation:
/// - `den > 0` (denominator is always positive)
/// - `gcd(|num|, den) == 1` (always in reduced form)
/// - Zero is represented as `0/1`
#[derive(Clone, Debug, Eq, Hash)]
pub struct Fraction {
    num: i64,
    den: i64,
}

impl Fraction {
    /// Create a new fraction from numerator and denominator (normalizes).
    #[inline]
    pub fn new(numer: i64, denom: i64) -> Self {
        assert!(denom != 0, "Fraction denominator cannot be zero");
        let g = gcd(numer, denom);
        let (n, d) = (numer / g, denom / g);
        // Ensure denominator is positive
        if d < 0 {
            Fraction { num: -n, den: -d }
        } else {
            Fraction { num: n, den: d }
        }
    }

    /// Create a fraction from an integer (no normalization needed).
    #[inline]
    pub fn from_integer(n: i64) -> Self {
        Fraction { num: n, den: 1 }
    }

    /// Start of the cycle containing this time (floor to nearest integer).
    /// In Strudel terminology, this is "sam" (from Hindustani "sam" meaning "downbeat").
    #[inline]
    pub fn sam(&self) -> Fraction {
        Fraction {
            num: self.floor_value(),
            den: 1,
        }
    }

    /// Start of the next cycle (sam + 1).
    #[inline]
    pub fn next_sam(&self) -> Fraction {
        Fraction {
            num: self.floor_value() + 1,
            den: 1,
        }
    }

    /// Position within the current cycle [0, 1).
    /// Returns the fractional part after subtracting sam.
    #[inline]
    pub fn cycle_pos(&self) -> Fraction {
        let f = self.floor_value();
        Fraction::new(self.num - f * self.den, self.den)
    }

    /// TimeSpan representing the full cycle containing this time.
    pub fn whole_cycle(&self) -> super::TimeSpan {
        super::TimeSpan::new(self.sam(), self.next_sam())
    }

    /// Convert to f64 (lossy).
    #[inline]
    pub fn to_f64(&self) -> f64 {
        self.num as f64 / self.den as f64
    }

    /// Floor value as i64 (Euclidean division toward negative infinity).
    #[inline]
    fn floor_value(&self) -> i64 {
        // For positive den (our invariant), floor division:
        // if num >= 0: num / den
        // if num < 0: (num - den + 1) / den
        if self.num >= 0 {
            self.num / self.den
        } else {
            (self.num - self.den + 1) / self.den
        }
    }

    /// Floor to nearest integer.
    #[inline]
    pub fn floor(&self) -> Fraction {
        Fraction {
            num: self.floor_value(),
            den: 1,
        }
    }

    /// Ceiling to nearest integer.
    #[inline]
    pub fn ceil(&self) -> Fraction {
        // ceil(a/b) for b > 0: if a%b == 0 then a/b else floor(a/b) + 1
        let f = self.floor_value();
        if f * self.den == self.num {
            Fraction { num: f, den: 1 }
        } else {
            Fraction { num: f + 1, den: 1 }
        }
    }

    /// Maximum of two fractions (by reference).
    #[inline]
    pub fn max_of(&self, other: &Fraction) -> Fraction {
        if self >= other {
            self.clone()
        } else {
            other.clone()
        }
    }

    /// Minimum of two fractions (by reference).
    #[inline]
    pub fn min_of(&self, other: &Fraction) -> Fraction {
        if self <= other {
            self.clone()
        } else {
            other.clone()
        }
    }

    /// Check if this fraction is zero.
    #[inline]
    pub fn is_zero(&self) -> bool {
        self.num == 0
    }

    /// Check if this fraction is one.
    #[inline]
    pub fn is_one(&self) -> bool {
        self.num == 1 && self.den == 1
    }

    /// Returns the numerator.
    #[inline]
    pub fn numer(&self) -> i64 {
        self.num
    }

    /// Returns the denominator.
    #[inline]
    pub fn denom(&self) -> i64 {
        self.den
    }

    /// Absolute value.
    #[inline]
    pub fn abs(&self) -> Fraction {
        Fraction {
            num: self.num.abs(),
            den: self.den,
        }
    }
}

impl Default for Fraction {
    #[inline]
    fn default() -> Self {
        Fraction { num: 0, den: 1 }
    }
}

impl From<i64> for Fraction {
    #[inline]
    fn from(n: i64) -> Self {
        Fraction::from_integer(n)
    }
}

impl From<i32> for Fraction {
    #[inline]
    fn from(n: i32) -> Self {
        Fraction::from_integer(n as i64)
    }
}

impl From<u32> for Fraction {
    #[inline]
    fn from(n: u32) -> Self {
        Fraction::from_integer(n as i64)
    }
}

impl From<usize> for Fraction {
    #[inline]
    fn from(n: usize) -> Self {
        Fraction::from_integer(n as i64)
    }
}

impl From<f64> for Fraction {
    fn from(f: f64) -> Self {
        // Convert float to a fraction with reasonable precision
        // This is lossy but necessary for interop
        if f.is_nan() || f.is_infinite() {
            return Fraction::from_integer(0);
        }

        // Use a reasonable precision (1/10000)
        let precision = 10000i64;
        let numer = (f * precision as f64).round() as i64;
        Fraction::new(numer, precision)
    }
}

impl PartialEq for Fraction {
    #[inline]
    fn eq(&self, other: &Self) -> bool {
        // Since fractions are always in reduced form with positive denominator,
        // equality is just component-wise comparison.
        self.num == other.num && self.den == other.den
    }
}

impl PartialOrd for Fraction {
    #[inline]
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Fraction {
    #[inline]
    fn cmp(&self, other: &Self) -> Ordering {
        // Cross-multiply to compare: a/b vs c/d  →  a*d vs c*b
        // Both denominators are positive (invariant), so sign is preserved.
        let lhs = self.num as i128 * other.den as i128;
        let rhs = other.num as i128 * self.den as i128;
        lhs.cmp(&rhs)
    }
}

// ===== Arithmetic Operations =====
// All operations normalize the result via Fraction::new.

impl Add for Fraction {
    type Output = Fraction;
    #[inline]
    fn add(self, other: Fraction) -> Fraction {
        Fraction::new(
            self.num * other.den + other.num * self.den,
            self.den * other.den,
        )
    }
}

impl Add<&Fraction> for Fraction {
    type Output = Fraction;
    #[inline]
    fn add(self, other: &Fraction) -> Fraction {
        Fraction::new(
            self.num * other.den + other.num * self.den,
            self.den * other.den,
        )
    }
}

impl Add<Fraction> for &Fraction {
    type Output = Fraction;
    #[inline]
    fn add(self, other: Fraction) -> Fraction {
        Fraction::new(
            self.num * other.den + other.num * self.den,
            self.den * other.den,
        )
    }
}

impl Add<&Fraction> for &Fraction {
    type Output = Fraction;
    #[inline]
    fn add(self, other: &Fraction) -> Fraction {
        Fraction::new(
            self.num * other.den + other.num * self.den,
            self.den * other.den,
        )
    }
}

impl Sub for Fraction {
    type Output = Fraction;
    #[inline]
    fn sub(self, other: Fraction) -> Fraction {
        Fraction::new(
            self.num * other.den - other.num * self.den,
            self.den * other.den,
        )
    }
}

impl Sub<&Fraction> for Fraction {
    type Output = Fraction;
    #[inline]
    fn sub(self, other: &Fraction) -> Fraction {
        Fraction::new(
            self.num * other.den - other.num * self.den,
            self.den * other.den,
        )
    }
}

impl Sub<Fraction> for &Fraction {
    type Output = Fraction;
    #[inline]
    fn sub(self, other: Fraction) -> Fraction {
        Fraction::new(
            self.num * other.den - other.num * self.den,
            self.den * other.den,
        )
    }
}

impl Sub<&Fraction> for &Fraction {
    type Output = Fraction;
    #[inline]
    fn sub(self, other: &Fraction) -> Fraction {
        Fraction::new(
            self.num * other.den - other.num * self.den,
            self.den * other.den,
        )
    }
}

impl Mul for Fraction {
    type Output = Fraction;
    #[inline]
    fn mul(self, other: Fraction) -> Fraction {
        Fraction::new(self.num * other.num, self.den * other.den)
    }
}

impl Mul<&Fraction> for Fraction {
    type Output = Fraction;
    #[inline]
    fn mul(self, other: &Fraction) -> Fraction {
        Fraction::new(self.num * other.num, self.den * other.den)
    }
}

impl Mul<Fraction> for &Fraction {
    type Output = Fraction;
    #[inline]
    fn mul(self, other: Fraction) -> Fraction {
        Fraction::new(self.num * other.num, self.den * other.den)
    }
}

impl Mul<&Fraction> for &Fraction {
    type Output = Fraction;
    #[inline]
    fn mul(self, other: &Fraction) -> Fraction {
        Fraction::new(self.num * other.num, self.den * other.den)
    }
}

impl Div for Fraction {
    type Output = Fraction;
    #[inline]
    fn div(self, other: Fraction) -> Fraction {
        assert!(other.num != 0, "Division by zero fraction");
        Fraction::new(self.num * other.den, self.den * other.num)
    }
}

impl Div<&Fraction> for Fraction {
    type Output = Fraction;
    #[inline]
    fn div(self, other: &Fraction) -> Fraction {
        assert!(other.num != 0, "Division by zero fraction");
        Fraction::new(self.num * other.den, self.den * other.num)
    }
}

impl Div<Fraction> for &Fraction {
    type Output = Fraction;
    #[inline]
    fn div(self, other: Fraction) -> Fraction {
        assert!(other.num != 0, "Division by zero fraction");
        Fraction::new(self.num * other.den, self.den * other.num)
    }
}

impl Div<&Fraction> for &Fraction {
    type Output = Fraction;
    #[inline]
    fn div(self, other: &Fraction) -> Fraction {
        assert!(other.num != 0, "Division by zero fraction");
        Fraction::new(self.num * other.den, self.den * other.num)
    }
}

impl Neg for Fraction {
    type Output = Fraction;
    #[inline]
    fn neg(self) -> Fraction {
        Fraction {
            num: -self.num,
            den: self.den,
        }
    }
}

impl Neg for &Fraction {
    type Output = Fraction;
    #[inline]
    fn neg(self) -> Fraction {
        Fraction {
            num: -self.num,
            den: self.den,
        }
    }
}

impl fmt::Display for Fraction {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.den == 1 {
            write!(f, "{}", self.num)
        } else {
            write!(f, "{}/{}", self.num, self.den)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sam() {
        assert_eq!(Fraction::new(5, 3).sam(), Fraction::from_integer(1));
        assert_eq!(Fraction::new(7, 4).sam(), Fraction::from_integer(1));
        assert_eq!(Fraction::new(3, 1).sam(), Fraction::from_integer(3));
        assert_eq!(Fraction::new(-1, 2).sam(), Fraction::from_integer(-1));
    }

    #[test]
    fn test_next_sam() {
        assert_eq!(Fraction::new(5, 3).next_sam(), Fraction::from_integer(2));
        assert_eq!(Fraction::new(0, 1).next_sam(), Fraction::from_integer(1));
    }

    #[test]
    fn test_cycle_pos() {
        assert_eq!(Fraction::new(5, 3).cycle_pos(), Fraction::new(2, 3));
        assert_eq!(Fraction::new(7, 4).cycle_pos(), Fraction::new(3, 4));
        assert_eq!(Fraction::new(3, 1).cycle_pos(), Fraction::from_integer(0));
    }

    #[test]
    fn test_arithmetic() {
        let a = Fraction::new(1, 2);
        let b = Fraction::new(1, 3);

        assert_eq!(&a + &b, Fraction::new(5, 6));
        assert_eq!(&a - &b, Fraction::new(1, 6));
        assert_eq!(&a * &b, Fraction::new(1, 6));
        assert_eq!(&a / &b, Fraction::new(3, 2));
    }

    #[test]
    fn test_from_f64() {
        let f = Fraction::from(0.5);
        assert_eq!(f, Fraction::new(1, 2));

        let f = Fraction::from(0.25);
        assert_eq!(f, Fraction::new(1, 4));
    }

    #[test]
    fn test_normalization() {
        // 4/6 should reduce to 2/3
        assert_eq!(Fraction::new(4, 6), Fraction::new(2, 3));

        // Negative denominator should be normalized
        assert_eq!(Fraction::new(1, -2), Fraction::new(-1, 2));

        // Both negative should normalize to positive
        assert_eq!(Fraction::new(-3, -4), Fraction::new(3, 4));
    }

    #[test]
    fn test_ordering() {
        assert!(Fraction::new(1, 3) < Fraction::new(1, 2));
        assert!(Fraction::new(2, 3) > Fraction::new(1, 2));
        assert!(Fraction::new(-1, 2) < Fraction::new(0, 1));
    }

    #[test]
    fn test_floor_ceil() {
        assert_eq!(Fraction::new(5, 3).floor(), Fraction::from_integer(1));
        assert_eq!(Fraction::new(5, 3).ceil(), Fraction::from_integer(2));
        assert_eq!(Fraction::new(6, 3).floor(), Fraction::from_integer(2));
        assert_eq!(Fraction::new(6, 3).ceil(), Fraction::from_integer(2));
        assert_eq!(Fraction::new(-1, 3).floor(), Fraction::from_integer(-1));
        assert_eq!(Fraction::new(-1, 3).ceil(), Fraction::from_integer(0));
    }

    #[test]
    fn test_display() {
        assert_eq!(format!("{}", Fraction::from_integer(5)), "5");
        assert_eq!(format!("{}", Fraction::new(1, 3)), "1/3");
        assert_eq!(format!("{}", Fraction::new(-2, 5)), "-2/5");
    }
}
