//! Exact rational number type for precise time representation.
//!
//! Uses rational numbers to avoid floating-point drift over time,
//! enabling precise subdivisions (triplets, quintuplets, etc.) and
//! exact cycle boundary computation.

use num::{BigInt, BigRational, One, ToPrimitive, Zero};
use std::cmp::Ordering;
use std::fmt;
use std::ops::{Add, Div, Mul, Neg, Sub};

/// Exact rational number for precise time representation.
#[derive(Clone, Debug, Eq, PartialEq, Hash)]
pub struct Fraction(BigRational);

impl Fraction {
    /// Create a new fraction from numerator and denominator.
    pub fn new(numer: i64, denom: i64) -> Self {
        Fraction(BigRational::new(BigInt::from(numer), BigInt::from(denom)))
    }

    /// Create a fraction from an integer.
    pub fn from_integer(n: i64) -> Self {
        Fraction(BigRational::from_integer(BigInt::from(n)))
    }

    /// Start of the cycle containing this time (floor to nearest integer).
    /// In Strudel terminology, this is "sam" (from Hindustani "sam" meaning "downbeat").
    pub fn sam(&self) -> Fraction {
        Fraction(BigRational::from_integer(self.0.floor().to_integer()))
    }

    /// Start of the next cycle (sam + 1).
    pub fn next_sam(&self) -> Fraction {
        self.sam() + Fraction::from_integer(1)
    }

    /// Position within the current cycle [0, 1).
    /// Returns the fractional part after subtracting sam.
    pub fn cycle_pos(&self) -> Fraction {
        self.clone() - self.sam()
    }

    /// TimeSpan representing the full cycle containing this time.
    pub fn whole_cycle(&self) -> super::TimeSpan {
        super::TimeSpan::new(self.sam(), self.next_sam())
    }

    /// Convert to f64 (lossy).
    pub fn to_f64(&self) -> f64 {
        self.0.to_f64().unwrap_or(0.0)
    }

    /// Floor to nearest integer.
    pub fn floor(&self) -> Fraction {
        Fraction(BigRational::from_integer(self.0.floor().to_integer()))
    }

    /// Ceiling to nearest integer.
    pub fn ceil(&self) -> Fraction {
        Fraction(BigRational::from_integer(self.0.ceil().to_integer()))
    }

    /// Maximum of two fractions (by reference).
    pub fn max_of(&self, other: &Fraction) -> Fraction {
        if self > other {
            self.clone()
        } else {
            other.clone()
        }
    }

    /// Minimum of two fractions (by reference).
    pub fn min_of(&self, other: &Fraction) -> Fraction {
        if self < other {
            self.clone()
        } else {
            other.clone()
        }
    }

    /// Check if this fraction is zero.
    pub fn is_zero(&self) -> bool {
        self.0.is_zero()
    }

    /// Check if this fraction is one.
    pub fn is_one(&self) -> bool {
        self.0.is_one()
    }

    /// Returns the numerator.
    pub fn numer(&self) -> i64 {
        self.0.numer().to_i64().unwrap_or(0)
    }

    /// Returns the denominator.
    pub fn denom(&self) -> i64 {
        self.0.denom().to_i64().unwrap_or(1)
    }

    /// Absolute value.
    pub fn abs(&self) -> Fraction {
        if self.0 < BigRational::zero() {
            -self.clone()
        } else {
            self.clone()
        }
    }
}

impl Default for Fraction {
    fn default() -> Self {
        Fraction::from_integer(0)
    }
}

impl From<i64> for Fraction {
    fn from(n: i64) -> Self {
        Fraction::from_integer(n)
    }
}

impl From<i32> for Fraction {
    fn from(n: i32) -> Self {
        Fraction::from_integer(n as i64)
    }
}

impl From<u32> for Fraction {
    fn from(n: u32) -> Self {
        Fraction::from_integer(n as i64)
    }
}

impl From<usize> for Fraction {
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

impl PartialOrd for Fraction {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Fraction {
    fn cmp(&self, other: &Self) -> Ordering {
        self.0.cmp(&other.0)
    }
}

impl Add for Fraction {
    type Output = Fraction;

    fn add(self, other: Fraction) -> Fraction {
        Fraction(self.0 + other.0)
    }
}

impl Add<&Fraction> for Fraction {
    type Output = Fraction;

    fn add(self, other: &Fraction) -> Fraction {
        Fraction(self.0 + &other.0)
    }
}

impl Add<Fraction> for &Fraction {
    type Output = Fraction;

    fn add(self, other: Fraction) -> Fraction {
        Fraction(&self.0 + other.0)
    }
}

impl Add<&Fraction> for &Fraction {
    type Output = Fraction;

    fn add(self, other: &Fraction) -> Fraction {
        Fraction(&self.0 + &other.0)
    }
}

impl Sub for Fraction {
    type Output = Fraction;

    fn sub(self, other: Fraction) -> Fraction {
        Fraction(self.0 - other.0)
    }
}

impl Sub<&Fraction> for Fraction {
    type Output = Fraction;

    fn sub(self, other: &Fraction) -> Fraction {
        Fraction(self.0 - &other.0)
    }
}

impl Sub<Fraction> for &Fraction {
    type Output = Fraction;

    fn sub(self, other: Fraction) -> Fraction {
        Fraction(&self.0 - other.0)
    }
}

impl Sub<&Fraction> for &Fraction {
    type Output = Fraction;

    fn sub(self, other: &Fraction) -> Fraction {
        Fraction(&self.0 - &other.0)
    }
}

impl Mul for Fraction {
    type Output = Fraction;

    fn mul(self, other: Fraction) -> Fraction {
        Fraction(self.0 * other.0)
    }
}

impl Mul<&Fraction> for Fraction {
    type Output = Fraction;

    fn mul(self, other: &Fraction) -> Fraction {
        Fraction(self.0 * &other.0)
    }
}

impl Mul<Fraction> for &Fraction {
    type Output = Fraction;

    fn mul(self, other: Fraction) -> Fraction {
        Fraction(&self.0 * other.0)
    }
}

impl Mul<&Fraction> for &Fraction {
    type Output = Fraction;

    fn mul(self, other: &Fraction) -> Fraction {
        Fraction(&self.0 * &other.0)
    }
}

impl Div for Fraction {
    type Output = Fraction;

    fn div(self, other: Fraction) -> Fraction {
        Fraction(self.0 / other.0)
    }
}

impl Div<&Fraction> for Fraction {
    type Output = Fraction;

    fn div(self, other: &Fraction) -> Fraction {
        Fraction(self.0 / &other.0)
    }
}

impl Div<Fraction> for &Fraction {
    type Output = Fraction;

    fn div(self, other: Fraction) -> Fraction {
        Fraction(&self.0 / other.0)
    }
}

impl Div<&Fraction> for &Fraction {
    type Output = Fraction;

    fn div(self, other: &Fraction) -> Fraction {
        Fraction(&self.0 / &other.0)
    }
}

impl Neg for Fraction {
    type Output = Fraction;

    fn neg(self) -> Fraction {
        Fraction(-self.0)
    }
}

impl Neg for &Fraction {
    type Output = Fraction;

    fn neg(self) -> Fraction {
        Fraction(-&self.0)
    }
}

impl fmt::Display for Fraction {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.0.is_integer() {
            write!(f, "{}", self.0.to_integer())
        } else {
            write!(f, "{}/{}", self.0.numer(), self.0.denom())
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
}
