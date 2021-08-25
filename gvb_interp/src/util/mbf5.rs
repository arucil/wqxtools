//! Microsoft Binary Format, Extended Precision (5 bytes)
//!
//! # Binary Format
//!
//! ```ignored
//!   byte 0  |      byte 1      |  byte 2  |  byte 3  |  byte 4
//!           | bit7   bit0~bit6 |
//!  exponent | sign   highest   |  higer   |  lower   |  lowest
//!           |        mantissa  | mantissa | mantissa | mantissa
//! ```
//!
//! The floating point number represented by the above form is:
//! ```ignored
//! (-1)^sign * 0.1M * 2^E
//! ```
//! where `M` represents the 31-bit mantissa, `E` represents the exponent.
//!
//! The exponent is in excess-128 form, e.g. 0x80 represents a exponent of 0,
//! 0x7a represents a exponent of -6, 0x84 represents a exponent of +4, etc.
//! 0x00 means the number is zero, and the mantissa doesn't matter.

use std::convert::TryFrom;
use std::fmt;
use std::fmt::Display;
use std::fmt::Write;
use std::ops::{Add, Div, Mul, Neg, Sub};

/// Used for store floating point value of a variable.
#[derive(Debug, Clone, Copy)]
pub struct Mbf5([u8; 5]);

/// Used for perform floating point calculations.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Mbf5Accum(f64);

const MANTISSA_BITS: usize = 31;
const MANTISSA_BITS_DIFF: usize = F64_MANTISSA_BITS - MANTISSA_BITS;

/// A bias of 0x81 instead of 0x80 because in IEEE754, the hidden bit of mantissa
/// is before the binary point (i.e. 1.M), while that of MBF is after the binary
/// point (i.e. 0.1M), and we want to make the mantissa of MBF conformant to the
/// rule of IEEE754, so here it is.
const EXPONENT_BIAS: i32 = 0x81;
const EXPONENT_BITS: usize = 31;

const F64_MANTISSA_BITS: usize = 52;
const F64_MANTISSA_MASK: u64 = (1 << F64_MANTISSA_BITS) - 1;
const F64_EXPONENT_BIAS: i32 = 1023;
const F64_EXPONENT_BITS: usize = 11;
const F64_EXPONENT_MASK: u64 = (1 << F64_EXPONENT_BITS) - 1;
const F64_EXPONENT_MAX: i32 = (1 << F64_EXPONENT_BITS) - 1;

impl From<&Mbf5> for Mbf5Accum {
  fn from(x: &Mbf5) -> Self {
    let sign = (x.0[1] >> 7) as u64;
    let exp = (x.0[0] as i32 - EXPONENT_BIAS + F64_EXPONENT_BIAS) as u64;
    let mant = (((x.0[1] & 0x7f) as u64) << 24)
      | ((x.0[2] as u64) << 16)
      | ((x.0[3] as u64) << 8)
      | x.0[4] as u64;

    let bits = (sign << (F64_MANTISSA_BITS + F64_EXPONENT_BITS))
      | (exp << F64_MANTISSA_BITS)
      | (mant << MANTISSA_BITS_DIFF);

    Self(f64::from_bits(bits))
  }
}

impl Into<f64> for Mbf5Accum {
  fn into(self) -> f64 {
    self.0
  }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FloatError {
  Nan,
  Infinite,
}

impl TryFrom<Mbf5Accum> for Mbf5 {
  /// Only FloatError::Infinite is possible
  type Error = FloatError;

  fn try_from(x: Mbf5Accum) -> Result<Self, FloatError> {
    let x = x.0.to_bits();
    let sign = (x >> (F64_MANTISSA_BITS + F64_EXPONENT_BITS)) as u8;
    let mut exp = f64_exponent(x) + EXPONENT_BIAS;
    let mut mant = x & F64_MANTISSA_MASK;

    // not infinite or NaN.
    assert!(exp != F64_EXPONENT_MAX - F64_EXPONENT_BIAS + EXPONENT_BIAS);

    // round mantissa
    const ROUND_BIT: u64 = 1 << (MANTISSA_BITS_DIFF - 1);
    const LOWEST_BIT: u64 = 1 << MANTISSA_BITS_DIFF;

    if mant & ROUND_BIT != 0 && mant & LOWEST_BIT != 0 {
      mant >>= MANTISSA_BITS_DIFF;
      mant += 1;
      // handle carry
      if mant & (1 << EXPONENT_BITS) != 0 {
        mant >>= 1;
        exp += 1;
      }
    } else {
      mant >>= MANTISSA_BITS_DIFF;
    }

    if exp > 0xff {
      return Err(FloatError::Infinite);
    }

    if exp <= 0 {
      return Ok(Self([0, sign << 7, 0, 0, 0]));
    }

    let exp = exp as u8;
    let sign = sign << 7;
    let mant1 = (mant >> 24) as u8 & 0x7f | sign;
    let mant2 = (mant >> 16) as u8;
    let mant3 = (mant >> 8) as u8;
    let mant4 = mant as u8;

    Ok(Self([exp, mant1, mant2, mant3, mant4]))
  }
}

impl TryFrom<f64> for Mbf5Accum {
  type Error = FloatError;

  fn try_from(value: f64) -> Result<Self, FloatError> {
    let x = value.to_bits();
    let exp = f64_exponent(x) + EXPONENT_BIAS;
    let mant = x & F64_MANTISSA_MASK;

    if exp == F64_EXPONENT_MAX - F64_EXPONENT_BIAS + EXPONENT_BIAS {
      if mant != 0 {
        return Err(FloatError::Nan);
      } else {
        return Err(FloatError::Infinite);
      }
    }

    if exp > 0xff {
      return Err(FloatError::Infinite);
    }

    if exp <= 0 {
      return Ok(Self(0.0));
    }

    Ok(Self(value))
  }
}

impl Display for Mbf5 {
  fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
    if self.is_zero() {
      return write!(fmt, "0");
    }

    // Why not simply implement Display for Mbf5Accum? Because after rounding
    // the mantissa of Mbf5Accum, the result may be infinity, so we construct
    // a Mbf5Accum from Mbf5, to make sure no rounding happens.
    let mut x = Mbf5Accum::from(self).0;

    if x < 0.0 {
      write!(fmt, "-")?;
      x = -x;
    }

    let mut result = String::new();
    if x < 0.01 || x >= 1_000_000_000.0 {
      write!(&mut result, "{:.8E}", x)?;
    } else {
      write!(&mut result, "{:}", x)?;
    }
    let end_of_frac = result.find('E').unwrap_or(result.len());
    let mut result = result.into_bytes();

    if end_of_frac < result.len() && result[end_of_frac + 1] >= b'0' {
      result.insert(end_of_frac + 1, b'+');
    }

    let mut i = end_of_frac - 1;
    while result[i] == b'0' || i > 9 {
      i -= 1;
    }

    if result[i] != b'.' {
      i += 1;
    }

    result.drain(i..end_of_frac);

    write!(fmt, "{}", std::str::from_utf8(&result).unwrap())
  }
}

impl Mbf5 {
  pub fn is_zero(&self) -> bool {
    self.0[0] == 0
  }

  pub fn as_array(&self) -> &[u8; 5] {
    &self.0
  }
}

pub type CalcResult = Result<Mbf5Accum, FloatError>;

impl Add for Mbf5Accum {
  type Output = CalcResult;

  fn add(self, rhs: Self) -> Self::Output {
    Self::try_from(self.0 + rhs.0)
  }
}

impl Sub for Mbf5Accum {
  type Output = CalcResult;

  fn sub(self, rhs: Self) -> Self::Output {
    Self::try_from(self.0 - rhs.0)
  }
}

impl Mul for Mbf5Accum {
  type Output = CalcResult;

  fn mul(self, rhs: Self) -> Self::Output {
    Self::try_from(self.0 * rhs.0)
  }
}

impl Div for Mbf5Accum {
  type Output = CalcResult;

  fn div(self, rhs: Self) -> Self::Output {
    Self::try_from(self.0 / rhs.0)
  }
}

impl Neg for Mbf5Accum {
  type Output = Self;

  fn neg(self) -> Self::Output {
    Self(-self.0)
  }
}

impl Mbf5Accum {
  pub fn is_positive(&self) -> bool {
    self.0 > 0.0
  }

  pub fn is_negative(&self) -> bool {
    self.0 < 0.0
  }

  pub fn is_zero(&self) -> bool {
    self.0 == 0.0
  }

  pub fn abs(&self) -> Self {
    Self(self.0.abs())
  }

  pub fn cos(&self) -> Self {
    Self::try_from(self.0.cos()).unwrap()
  }

  pub fn sin(&self) -> Self {
    Self::try_from(self.0.sin()).unwrap()
  }

  pub fn tan(&self) -> CalcResult {
    Self::try_from(self.0.tan())
  }

  pub fn atan(&self) -> Self {
    Self::try_from(self.0.atan()).unwrap()
  }

  pub fn exp(&self) -> CalcResult {
    Self::try_from(self.0.exp())
  }

  pub fn truncate(&self) -> Self {
    Self::try_from(self.0.trunc()).unwrap()
  }

  pub fn ln(&self) -> CalcResult {
    Self::try_from(self.0.ln())
  }

  pub fn sqrt(&self) -> CalcResult {
    Self::try_from(self.0.sqrt())
  }
}

fn f64_exponent(x: u64) -> i32 {
  ((x >> F64_MANTISSA_BITS) & F64_EXPONENT_MASK) as i32 - F64_EXPONENT_BIAS
}

#[cfg(test)]
mod tests {
  use super::*;
  use std::convert::TryFrom;

  #[test]
  fn f64_to_mbf5_accum_valid() {
    assert_eq!(Ok(17.625), Mbf5Accum::try_from(17.625).map(|x| x.0));
  }

  #[test]
  fn f64_to_mbf5_accum_max() {
    assert_eq!(
      Ok(1.70141183e38),
      Mbf5Accum::try_from(1.70141183e38).map(|x| x.0)
    );
  }

  #[test]
  fn f64_to_mbf5_accum_negative() {
    assert_eq!(Ok(-34.6189), Mbf5Accum::try_from(-34.6189).map(|x| x.0));
  }

  #[test]
  fn f64_to_mbf5_accum_zero() {
    assert_eq!(Ok(0.0), Mbf5Accum::try_from(0.0).map(|x| x.0));
  }

  #[test]
  fn f64_to_mbf5_accum_exp_too_large() {
    assert_eq!(
      Err(FloatError::Infinite),
      Mbf5Accum::try_from(1.7e39).map(|x| x.0)
    );
  }

  #[test]
  fn f64_to_mbf5_accum_too_large() {
    assert_eq!(
      Err(FloatError::Infinite),
      Mbf5Accum::try_from(1.70141184e38).map(|x| x.0)
    );
  }

  #[test]
  fn f64_to_mbf5_accum_nan() {
    assert_eq!(
      Err(FloatError::Nan),
      Mbf5Accum::try_from(0.0 / 0.0).map(|x| x.0)
    );
  }

  #[test]
  fn fmt_mbf5_zero() {
    assert_eq!("0", &Mbf5([0, 0, 0, 0, 0]).to_string());
  }

  #[test]
  fn fmt_mbf5_one() {
    assert_eq!("1", &Mbf5([0x81, 0, 0, 0, 0]).to_string());
  }

  #[test]
  fn fmt_mbf5_neg_one() {
    assert_eq!("-1", &Mbf5([0x81, 0x80, 0, 0, 0]).to_string());
  }

  #[test]
  fn fmt_mbf5_max() {
    assert_eq!(
      "1.70141183E+38",
      &Mbf5([0xff, 0x7f, 0xff, 0xff, 0xff]).to_string()
    );
  }

  #[test]
  fn fmt_mbf5_neg_max() {
    assert_eq!(
      "-1.70141183E+38",
      &Mbf5([0xff, 0xff, 0xff, 0xff, 0xff]).to_string()
    );
  }

  #[test]
  fn fmt_mbf5_neg_0_5() {
    assert_eq!("-0.5", &Mbf5([0x80, 0x80, 0x00, 0x00, 0x00]).to_string());
  }

  #[test]
  fn fmt_mbf5_sqr_2() {
    assert_eq!(
      "1.41421356",
      &Mbf5([0x81, 0x35, 0x04, 0xf3, 0x34]).to_string()
    );
  }

  #[test]
  fn fmt_mbf5_1_000_000_000() {
    assert_eq!("1E+9", &Mbf5([0x9e, 0x6e, 0x6b, 0x28, 0x00]).to_string());
  }

  #[test]
  fn fmt_mbf5_neg_1_000_000_000() {
    assert_eq!("-1E+9", &Mbf5([0x9e, 0xee, 0x6b, 0x28, 0x00]).to_string());
  }

  #[test]
  fn fmt_mbf5_999_999_999() {
    assert_eq!(
      "999999999",
      &Mbf5([0x9e, 0x6e, 0x6b, 0x27, 0xfc]).to_string()
    );
  }

  #[test]
  fn fmt_mbf5_neg_999_999_999() {
    assert_eq!(
      "-999999999",
      &Mbf5([0x9e, 0xee, 0x6b, 0x27, 0xfc]).to_string()
    );
  }

  #[test]
  fn fmt_mbf5_0_01() {
    assert_eq!("0.01", &Mbf5([0x7a, 0x23, 0xd7, 0x0a, 0x3e]).to_string());
  }

  #[test]
  fn fmt_mbf5_0_0003765() {
    assert_eq!(
      "3.765E-4",
      &Mbf5([0x75, 0x45, 0x64, 0xf9, 0x7e]).to_string()
    );
  }

  #[test]
  fn fmt_mbf5_11879546_4() {
    assert_eq!(
      "11879546",
      &Mbf5([0x98, 0x35, 0x44, 0x7a, 0x00]).to_string()
    );
  }

  #[test]
  fn fmt_mbf5_3_92767774_e_neg_8() {
    assert_eq!(
      "3.92767774E-8",
      &Mbf5([0x68, 0x28, 0xb1, 0x46, 0x00]).to_string()
    );
  }

  #[test]
  fn pos_is_pos() {
    let a = Mbf5Accum::try_from(41.73).unwrap();
    assert_eq!(true, a.is_positive());
  }

  #[test]
  fn zero_is_pos() {
    let a = Mbf5Accum::try_from(0.0).unwrap();
    assert_eq!(false, a.is_positive());
  }

  #[test]
  fn neg_is_pos() {
    let a = Mbf5Accum::try_from(-41.73).unwrap();
    assert_eq!(false, a.is_positive());
  }

  #[test]
  fn pos_is_neg() {
    let a = Mbf5Accum::try_from(41.73).unwrap();
    assert_eq!(false, a.is_negative());
  }

  #[test]
  fn zero_is_neg() {
    let a = Mbf5Accum::try_from(0.0).unwrap();
    assert_eq!(false, a.is_negative());
  }

  #[test]
  fn neg_is_neg() {
    let a = Mbf5Accum::try_from(-41.73).unwrap();
    assert_eq!(true, a.is_negative());
  }

  #[test]
  fn pos_is_zero() {
    let a = Mbf5Accum::try_from(41.73).unwrap();
    assert_eq!(false, a.is_zero());
  }

  #[test]
  fn zero_is_zero() {
    let a = Mbf5Accum::try_from(0.0).unwrap();
    assert_eq!(true, a.is_zero());
  }

  #[test]
  fn neg_is_zero() {
    let a = Mbf5Accum::try_from(-41.73).unwrap();
    assert_eq!(false, a.is_zero());
  }

  #[test]
  fn neg_pos() {
    let a = Mbf5Accum::try_from(41.73).unwrap();
    assert_eq!(-41.73, (-a).0);
  }

  #[test]
  fn neg_0() {
    let a = Mbf5Accum::try_from(0.0).unwrap();
    assert_eq!(0.0, (-a).0);
  }

  #[test]
  fn neg_neg() {
    let a = Mbf5Accum::try_from(-41.73).unwrap();
    assert_eq!(41.73, (-a).0);
  }

  #[test]
  fn add_normal() {
    let a = Mbf5Accum::try_from(41.73).unwrap();
    let b = Mbf5Accum::try_from(-7.1342).unwrap();
    assert_eq!(Ok(a.0 + b.0), (a + b).map(|x| x.0));
  }

  #[test]
  fn add_overflow() {
    let a = Mbf5Accum::try_from(1.70141183e+38).unwrap();
    let b = Mbf5Accum::try_from(0.00000001e+38).unwrap();
    assert_eq!(Err(FloatError::Infinite), (a + b).map(|x| x.0));
  }

  #[test]
  fn add_neg_overflow() {
    let a = Mbf5Accum::try_from(-1.70141183e+38).unwrap();
    let b = Mbf5Accum::try_from(-0.00000001e+38).unwrap();
    assert_eq!(Err(FloatError::Infinite), (a + b).map(|x| x.0));
  }

  #[test]
  fn sub_normal() {
    let a = Mbf5Accum::try_from(41.73).unwrap();
    let b = Mbf5Accum::try_from(-7.1342).unwrap();
    assert_eq!(Ok(a.0 - b.0), (a - b).map(|x| x.0));
  }

  #[test]
  fn sub_overflow() {
    let a = Mbf5Accum::try_from(1.70141183e+38).unwrap();
    let b = Mbf5Accum::try_from(-0.00000001e+38).unwrap();
    assert_eq!(Err(FloatError::Infinite), (a - b).map(|x| x.0));
  }

  #[test]
  fn sub_neg_overflow() {
    let a = Mbf5Accum::try_from(-1.70141183e+38).unwrap();
    let b = Mbf5Accum::try_from(0.00000001e+38).unwrap();
    assert_eq!(Err(FloatError::Infinite), (a - b).map(|x| x.0));
  }

  #[test]
  fn mul_normal() {
    let a = Mbf5Accum::try_from(41.73).unwrap();
    let b = Mbf5Accum::try_from(-7.1342).unwrap();
    assert_eq!(Ok(a.0 * b.0), (a * b).map(|x| x.0));
  }

  #[test]
  fn mul_overflow() {
    let a = Mbf5Accum::try_from(1e34).unwrap();
    let b = Mbf5Accum::try_from(2e4).unwrap();
    assert_eq!(Err(FloatError::Infinite), (a * b).map(|x| x.0));
  }

  #[test]
  fn mul_neg_overflow() {
    let a = Mbf5Accum::try_from(1e34).unwrap();
    let b = Mbf5Accum::try_from(-2e4).unwrap();
    assert_eq!(Err(FloatError::Infinite), (a * b).map(|x| x.0));
  }

  #[test]
  fn div_normal() {
    let a = Mbf5Accum::try_from(41.73).unwrap();
    let b = Mbf5Accum::try_from(-7.1342).unwrap();
    assert_eq!(Ok(a.0 / b.0), (a / b).map(|x| x.0));
  }

  #[test]
  fn div_by_0() {
    let a = Mbf5Accum::try_from(41.73).unwrap();
    let b = Mbf5Accum::try_from(0.0).unwrap();
    assert_eq!(Err(FloatError::Infinite), (a / b).map(|x| x.0));
  }

  #[test]
  fn div_nan() {
    let a = Mbf5Accum::try_from(0.0).unwrap();
    let b = Mbf5Accum::try_from(0.0).unwrap();
    assert_eq!(Err(FloatError::Nan), (a / b).map(|x| x.0));
  }

  #[test]
  fn div_overflow() {
    let a = Mbf5Accum::try_from(1.70141184e+37).unwrap();
    let b = Mbf5Accum::try_from(0.1).unwrap();
    assert_eq!(Err(FloatError::Infinite), (a / b).map(|x| x.0));
  }

  #[test]
  fn abs_pos() {
    let a = Mbf5Accum::try_from(1.74).unwrap();
    assert_eq!(1.74, a.abs().0);
  }

  #[test]
  fn abs_zero() {
    let a = Mbf5Accum::try_from(0.0).unwrap();
    assert_eq!(0.0, a.abs().0);
  }

  #[test]
  fn abs_neg() {
    let a = Mbf5Accum::try_from(-54.8).unwrap();
    assert_eq!(54.8, a.abs().0);
  }

  #[test]
  fn sin_normal() {
    let a = Mbf5Accum::try_from(617849.13).unwrap();
    assert_eq!(a.0.sin(), a.sin().0);
  }

  #[test]
  fn cos_normal() {
    let a = Mbf5Accum::try_from(617849.13).unwrap();
    assert_eq!(a.0.cos(), a.cos().0);
  }

  #[test]
  fn tan_normal() {
    let a = Mbf5Accum::try_from(1.74).unwrap();
    assert_eq!(Ok(1.74f64.tan()), a.tan().map(|x| x.0));
  }

  #[test]
  fn tan_large() {
    let a = Mbf5Accum::try_from(std::f64::consts::FRAC_PI_2).unwrap();
    assert_eq!(Ok(16331239353195370.0), a.tan().map(|x| x.0));
  }

  #[test]
  fn ln_normal() {
    let a = Mbf5Accum::try_from(135.16).unwrap();
    assert_eq!(Ok(a.0.ln()), a.ln().map(|x| x.0));
  }

  #[test]
  fn ln_zero() {
    let a = Mbf5Accum::try_from(0.0).unwrap();
    assert_eq!(Err(FloatError::Infinite), a.ln().map(|x| x.0));
  }

  #[test]
  fn ln_neg() {
    let a = Mbf5Accum::try_from(-14.1).unwrap();
    assert_eq!(Err(FloatError::Nan), a.ln().map(|x| x.0));
  }
}
