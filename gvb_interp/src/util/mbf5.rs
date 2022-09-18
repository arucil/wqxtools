//! Microsoft Binary Format, Extended Precision (5 bytes)
//!
//! # Binary Format
//!
//! ```ignored
//!   byte 0  |      byte 1      |  byte 2  |  byte 3  |  byte 4
//!           | bit7   bit0~bit6 |
//! ----------|------------------|----------|----------|----------
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
//!
//! MBF5 doesn't have NaN and infinite number form.

use std::convert::TryFrom;
use std::fmt::{self, Display, Formatter, Write};
use std::ops::{Add, Div, Mul, Neg, Sub};
use std::str::FromStr;

/// Used for store floating point value of a variable.
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
pub struct Mbf5(f64);

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

impl From<[u8; 5]> for Mbf5 {
  fn from(x: [u8; 5]) -> Self {
    let sign = (x[1] >> 7) as u64;
    let exp = x[0] as i32;
    if exp == 0 {
      return Self(0.0);
    }
    let exp = (exp - EXPONENT_BIAS + F64_EXPONENT_BIAS) as u64;

    let mant = (((x[1] & 0x7f) as u64) << 24)
      | ((x[2] as u64) << 16)
      | ((x[3] as u64) << 8)
      | x[4] as u64;

    let bits = (sign << (F64_MANTISSA_BITS + F64_EXPONENT_BITS))
      | (exp << F64_MANTISSA_BITS)
      | (mant << MANTISSA_BITS_DIFF);

    Self(f64::from_bits(bits))
  }
}

impl From<Mbf5> for f64 {
  fn from(n: Mbf5) -> Self {
    n.0
  }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RealError {
  Nan,
  Infinite,
}

/// `value` will be normalized.
fn f64_to_array(value: &mut f64) -> Result<[u8; 5], RealError> {
  let x = value.to_bits();
  let sign = (x >> (F64_MANTISSA_BITS + F64_EXPONENT_BITS)) as u8;
  let mut exp = f64_exponent(x) + EXPONENT_BIAS;
  let mut mant = x & F64_MANTISSA_MASK;

  if exp == F64_EXPONENT_MAX - F64_EXPONENT_BIAS + EXPONENT_BIAS {
    if mant != 0 {
      return Err(RealError::Nan);
    } else {
      return Err(RealError::Infinite);
    }
  }

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
    return Err(RealError::Infinite);
  }

  if exp <= 0 {
    *value = 0.0;
    return Ok([0; 5]);
  }

  let exp = exp as _;
  let sign = sign << 7;
  let mant1 = (mant >> 24) as u8 & 0x7f | sign;
  let mant2 = (mant >> 16) as _;
  let mant3 = (mant >> 8) as _;
  let mant4 = mant as _;

  Ok([exp, mant1, mant2, mant3, mant4])
}

impl TryFrom<f64> for Mbf5 {
  type Error = RealError;

  fn try_from(mut value: f64) -> Result<Self, Self::Error> {
    f64_to_array(&mut value)?;
    Ok(Self(value))
  }
}

impl From<Mbf5> for [u8; 5] {
  fn from(mut value: Mbf5) -> [u8; 5] {
    f64_to_array(&mut value.0).unwrap()
  }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ParseRealError {
  Infinite,
  Malformed,
}

impl From<RealError> for ParseRealError {
  fn from(err: RealError) -> Self {
    match err {
      RealError::Infinite => Self::Infinite,
      _ => unreachable!(),
    }
  }
}

impl FromStr for Mbf5 {
  type Err = ParseRealError;

  fn from_str(s: &str) -> Result<Self, Self::Err> {
    let s = s.as_bytes();
    let mut i = 0;
    let mut buf = String::new();

    if s.is_empty() {
      return Err(ParseRealError::Malformed);
    }

    macro_rules! push_digits {
      () => {
        if matches!(s.get(i), Some(c) if c.is_ascii_digit()) {
          while let Some(&c) = s.get(i) {
            if c.is_ascii_digit() {
              buf.push(c as char);
              i += 1;
            } else {
              break;
            }
          }
        } else {
          buf.push('0');
        }
      }
    }

    match s.first() {
      Some(b'-') => {
        buf.push('-');
        i += 1;
      }
      Some(b'+') => {
        i += 1;
      }
      _ => {}
    }

    push_digits!();

    if let Some(b'.') = s.get(i) {
      buf.push('.');
      i += 1;
      push_digits!();
    }

    if let Some(b'e' | b'E') = s.get(i) {
      buf.push('e');
      i += 1;
      if let Some(c @ (b'+' | b'-')) = s.get(i) {
        buf.push(*c as char);
        i += 1;
      }
      push_digits!();
    }

    if s.get(i).is_some() {
      return Err(ParseRealError::Malformed);
    }

    let mut num = buf.parse::<f64>().unwrap();
    f64_to_array(&mut num)?;
    Ok(Self(num))
  }
}

impl From<u8> for Mbf5 {
  fn from(n: u8) -> Self {
    Self(n as f64)
  }
}

impl From<i16> for Mbf5 {
  fn from(n: i16) -> Self {
    Self(n as f64)
  }
}

impl From<u32> for Mbf5 {
  fn from(n: u32) -> Self {
    Self(n as f64)
  }
}

impl From<u64> for Mbf5 {
  fn from(n: u64) -> Self {
    Self(n as f64)
  }
}

impl From<bool> for Mbf5 {
  fn from(n: bool) -> Self {
    Self(n as u32 as f64)
  }
}

impl Display for Mbf5 {
  fn fmt(&self, f: &mut Formatter) -> fmt::Result {
    if self.is_zero() {
      return f.write_char('0');
    }

    // Why not simply implement Display for Mbf5Accum? Because after rounding
    // the mantissa of Mbf5Accum, the result may be infinity, so we construct
    // a Mbf5Accum from Mbf5, to make sure no rounding happens.
    let mut x = self.0;

    if x < 0.0 {
      f.write_char('-')?;
      x = -x;
    }

    let mut base10_exponent = 0i32;
    let exponent = f64_exponent(self.0.to_bits());
    if exponent <= 0x80 {
      base10_exponent = -9;
      x *= 1e9;
    }

    if x > 999_999_999.0 {
      while x >= 999_999_999.0 {
        base10_exponent += 1;
        x /= 10.0;
      }
    }

    if x != 999_999_999.0 {
      while x <= 99_999_999.9 {
        base10_exponent -= 1;
        x *= 10.0;
      }
      x += 0.5;
    }

    base10_exponent += 10;
    let mut point_index = if !(0..=10).contains(&base10_exponent) {
      base10_exponent -= 2;
      1
    } else {
      let x = base10_exponent - 1;
      base10_exponent = 0;
      x
    };

    if point_index == 0 {
      f.write_char('.')?;
    } else if point_index == -1 {
      f.write_char('.')?;
      f.write_char('0')?;
    }

    let mut int = x as u32;
    let mut num_digits = 0;
    let mut digits = [0u8; 11];
    for divisor in [
      100_000_000u32,
      10_000_000,
      1_000_000,
      100_000,
      10_000,
      1000,
      100,
      10,
      1,
    ] {
      digits[num_digits] = b'0' + (int / divisor) as u8;
      num_digits += 1;
      int %= divisor;

      point_index -= 1;
      if point_index == 0 {
        digits[num_digits] = b'.';
        num_digits += 1;
      }
    }

    if digits[num_digits - 1] == b'0' {
      while digits[num_digits - 1] == b'0' {
        num_digits -= 1;
      }
    }

    if digits[num_digits - 1] == b'.' {
      num_digits -= 1;
    }

    f.write_str(unsafe {
      std::str::from_utf8_unchecked(&digits[..num_digits])
    })?;

    if base10_exponent == 0 {
      return Ok(());
    } else {
      f.write_char('E')?;
    }

    if base10_exponent < 0 {
      base10_exponent = -base10_exponent;
      f.write_char('-')?;
    } else {
      f.write_char('+')?;
    }

    f.write_char((b'0' + (base10_exponent / 10) as u8) as char)?;
    f.write_char((b'0' + (base10_exponent % 10) as u8) as char)
  }
}

pub type CalcResult = Result<Mbf5, RealError>;

impl Add for Mbf5 {
  type Output = CalcResult;

  fn add(self, rhs: Self) -> Self::Output {
    Self::try_from(self.0 + rhs.0)
  }
}

impl Sub for Mbf5 {
  type Output = CalcResult;

  fn sub(self, rhs: Self) -> Self::Output {
    Self::try_from(self.0 - rhs.0)
  }
}

impl Mul for Mbf5 {
  type Output = CalcResult;

  fn mul(self, rhs: Self) -> Self::Output {
    Self::try_from(self.0 * rhs.0)
  }
}

impl Div for Mbf5 {
  type Output = CalcResult;

  fn div(self, rhs: Self) -> Self::Output {
    Self::try_from(self.0 / rhs.0)
  }
}

impl Neg for Mbf5 {
  type Output = Self;

  fn neg(self) -> Self::Output {
    Self(-self.0)
  }
}

impl PartialEq<f64> for Mbf5 {
  fn eq(&self, other: &f64) -> bool {
    self.0 == *other
  }
}

impl Mbf5 {
  pub const ZERO: Self = Self(0.0);

  pub const ONE: Self = Self(1.0);

  pub const NEG_ONE: Self = Self(-1.0);

  pub fn is_positive(&self) -> bool {
    self.0 > 0.0
  }

  pub fn is_negative(&self) -> bool {
    self.0 < 0.0
  }

  pub fn is_zero(&self) -> bool {
    self.0 == 0.0
  }

  pub fn is_one(&self) -> bool {
    self.0 == 1.0
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

  pub fn pow(&self, exp: Mbf5) -> CalcResult {
    Self::try_from(self.0.powf(exp.0))
  }
}

fn f64_exponent(x: u64) -> i32 {
  ((x >> F64_MANTISSA_BITS) & F64_EXPONENT_MASK) as i32 - F64_EXPONENT_BIAS
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn f64_to_mbf5_accum_valid() {
    assert_eq!(Ok(17.625), Mbf5::try_from(17.625).map(|x| x.0));
  }

  #[test]
  fn f64_to_mbf5_accum_max() {
    assert_eq!(
      Ok(1.70141183e38),
      Mbf5::try_from(1.70141183e38).map(|x| x.0)
    );
  }

  #[test]
  fn f64_to_mbf5_accum_negative() {
    assert_eq!(Ok(-34.6189), Mbf5::try_from(-34.6189).map(|x| x.0));
  }

  #[test]
  fn f64_to_mbf5_accum_zero() {
    assert_eq!(Ok(0.0), Mbf5::try_from(0.0).map(|x| x.0));
  }

  #[test]
  fn f64_to_mbf5_accum_exp_too_large() {
    assert_eq!(
      Err(RealError::Infinite),
      Mbf5::try_from(1.7e39).map(|x| x.0)
    );
  }

  #[test]
  fn f64_to_mbf5_accum_too_large() {
    assert_eq!(
      Err(RealError::Infinite),
      Mbf5::try_from(1.70141184e38).map(|x| x.0)
    );
  }

  #[test]
  fn f64_to_mbf5_accum_nan() {
    assert_eq!(Err(RealError::Nan), Mbf5::try_from(f64::NAN).map(|x| x.0));
  }

  #[test]
  fn fmt_mbf5_zero() {
    assert_eq!("0", &Mbf5::from([0u8; 5]).to_string());
  }

  #[test]
  fn fmt_mbf5_one() {
    assert_eq!("1", &Mbf5::from([0x81, 0, 0, 0, 0]).to_string());
  }

  #[test]
  fn fmt_mbf5_neg_one() {
    assert_eq!("-1", &Mbf5::from([0x81, 0x80, 0, 0, 0]).to_string());
  }

  #[test]
  fn fmt_mbf5_12() {
    assert_eq!("12", &Mbf5::from([0x84, 0x40, 0, 0, 0]).to_string());
  }

  #[test]
  fn fmt_mbf5_max() {
    assert_eq!(
      "1.70141183E+38",
      &Mbf5::from([0xff, 0x7f, 0xff, 0xff, 0xff]).to_string()
    );
  }

  #[test]
  fn fmt_mbf5_mantissa_max() {
    assert_eq!(
      "0.9999999997671694",
      &Mbf5::from([0x80, 0x7f, 0xff, 0xff, 0xff]).0.to_string()
    );
  }

  #[test]
  fn fmt_mbf5_neg_max() {
    assert_eq!(
      "-1.70141183E+38",
      &Mbf5::from([0xff, 0xff, 0xff, 0xff, 0xff]).to_string()
    );
  }

  #[test]
  fn fmt_mbf5_neg_0_5() {
    assert_eq!(
      "-.5",
      &Mbf5::from([0x80, 0x80, 0x00, 0x00, 0x00]).to_string()
    );
  }

  #[test]
  fn fmt_mbf5_sqr_2() {
    assert_eq!(
      "1.41421356",
      &Mbf5::from([0x81, 0x35, 0x04, 0xf3, 0x34]).to_string()
    );
  }

  #[test]
  fn fmt_mbf5_1_000_000_000() {
    assert_eq!(
      "1E+09",
      &Mbf5::from([0x9e, 0x6e, 0x6b, 0x28, 0x00]).to_string()
    );
  }

  #[test]
  fn fmt_mbf5_neg_1_000_000_000() {
    assert_eq!(
      "-1E+09",
      &Mbf5::from([0x9e, 0xee, 0x6b, 0x28, 0x00]).to_string()
    );
  }

  #[test]
  fn fmt_mbf5_999_999_999() {
    assert_eq!(
      "999999999",
      &Mbf5::from([0x9e, 0x6e, 0x6b, 0x27, 0xfc]).to_string()
    );
  }

  #[test]
  fn fmt_mbf5_neg_999_999_999() {
    assert_eq!(
      "-999999999",
      &Mbf5::from([0x9e, 0xee, 0x6b, 0x27, 0xfc]).to_string()
    );
  }

  #[test]
  fn fmt_mbf5_0_01() {
    assert_eq!(
      ".01",
      &Mbf5::from([0x7a, 0x23, 0xd7, 0x0a, 0x3e]).to_string()
    );
  }

  #[test]
  fn fmt_mbf5_0_0003765() {
    assert_eq!(
      "3.765E-04",
      &Mbf5::from([0x75, 0x45, 0x64, 0xf9, 0x7e]).to_string()
    );
  }

  #[test]
  fn fmt_mbf5_11879546_4() {
    assert_eq!(
      "11879546",
      &Mbf5::from([0x98, 0x35, 0x44, 0x7a, 0x00]).to_string()
    );
  }

  #[test]
  fn fmt_mbf5_3_92767774_e_neg_8() {
    assert_eq!(
      "3.92767774E-08",
      &Mbf5::from([0x68, 0x28, 0xb1, 0x46, 0x00]).to_string()
    );
  }

  #[test]
  fn pos_is_pos() {
    let a = Mbf5::try_from(41.73).unwrap();
    assert!(a.is_positive());
  }

  #[test]
  fn zero_is_pos() {
    let a = Mbf5::try_from(0.0).unwrap();
    assert!(!a.is_positive());
  }

  #[test]
  fn neg_is_pos() {
    let a = Mbf5::try_from(-41.73).unwrap();
    assert!(!a.is_positive());
  }

  #[test]
  fn pos_is_neg() {
    let a = Mbf5::try_from(41.73).unwrap();
    assert!(!a.is_negative());
  }

  #[test]
  fn zero_is_neg() {
    let a = Mbf5::try_from(0.0).unwrap();
    assert!(!a.is_negative());
  }

  #[test]
  fn neg_is_neg() {
    let a = Mbf5::try_from(-41.73).unwrap();
    assert!(a.is_negative());
  }

  #[test]
  fn pos_is_zero() {
    let a = Mbf5::try_from(41.73).unwrap();
    assert!(!a.is_zero());
  }

  #[test]
  fn zero_is_zero() {
    let a = Mbf5::try_from(0.0).unwrap();
    assert!(a.is_zero());
  }

  #[test]
  fn neg_is_zero() {
    let a = Mbf5::try_from(-41.73).unwrap();
    assert!(!a.is_zero());
  }

  #[test]
  fn neg_pos() {
    let a = Mbf5::try_from(41.73).unwrap();
    assert_eq!(-41.73, (-a).0);
  }

  #[test]
  fn neg_0() {
    let a = Mbf5::try_from(0.0).unwrap();
    assert_eq!(0.0, (-a).0);
  }

  #[test]
  fn neg_neg() {
    let a = Mbf5::try_from(-41.73).unwrap();
    assert_eq!(41.73, (-a).0);
  }

  #[test]
  fn add_normal() {
    let a = Mbf5::try_from(41.73).unwrap();
    let b = Mbf5::try_from(-7.1342).unwrap();
    assert_eq!(Ok(a.0 + b.0), (a + b).map(|x| x.0));
  }

  #[test]
  fn add_overflow() {
    let a = Mbf5::try_from(1.70141183e+38).unwrap();
    let b = Mbf5::try_from(0.00000001e+38).unwrap();
    assert_eq!(Err(RealError::Infinite), (a + b).map(|x| x.0));
  }

  #[test]
  fn add_neg_overflow() {
    let a = Mbf5::try_from(-1.70141183e+38).unwrap();
    let b = Mbf5::try_from(-0.00000001e+38).unwrap();
    assert_eq!(Err(RealError::Infinite), (a + b).map(|x| x.0));
  }

  #[test]
  fn sub_normal() {
    let a = Mbf5::try_from(41.73).unwrap();
    let b = Mbf5::try_from(-7.1342).unwrap();
    assert_eq!(Ok(a.0 - b.0), (a - b).map(|x| x.0));
  }

  #[test]
  fn sub_overflow() {
    let a = Mbf5::try_from(1.70141183e+38).unwrap();
    let b = Mbf5::try_from(-0.00000001e+38).unwrap();
    assert_eq!(Err(RealError::Infinite), (a - b).map(|x| x.0));
  }

  #[test]
  fn sub_neg_overflow() {
    let a = Mbf5::try_from(-1.70141183e+38).unwrap();
    let b = Mbf5::try_from(0.00000001e+38).unwrap();
    assert_eq!(Err(RealError::Infinite), (a - b).map(|x| x.0));
  }

  #[test]
  fn mul_normal() {
    let a = Mbf5::try_from(41.73).unwrap();
    let b = Mbf5::try_from(-7.1342).unwrap();
    assert_eq!(Ok(a.0 * b.0), (a * b).map(|x| x.0));
  }

  #[test]
  fn mul_overflow() {
    let a = Mbf5::try_from(1e34).unwrap();
    let b = Mbf5::try_from(2e4).unwrap();
    assert_eq!(Err(RealError::Infinite), (a * b).map(|x| x.0));
  }

  #[test]
  fn mul_neg_overflow() {
    let a = Mbf5::try_from(1e34).unwrap();
    let b = Mbf5::try_from(-2e4).unwrap();
    assert_eq!(Err(RealError::Infinite), (a * b).map(|x| x.0));
  }

  #[test]
  fn div_normal() {
    let a = Mbf5::try_from(41.73).unwrap();
    let b = Mbf5::try_from(-7.1342).unwrap();
    assert_eq!(Ok(a.0 / b.0), (a / b).map(|x| x.0));
  }

  #[test]
  fn div_by_0() {
    let a = Mbf5::try_from(41.73).unwrap();
    let b = Mbf5::try_from(0.0).unwrap();
    assert_eq!(Err(RealError::Infinite), (a / b).map(|x| x.0));
  }

  #[test]
  fn div_nan() {
    let a = Mbf5::try_from(0.0).unwrap();
    let b = Mbf5::try_from(0.0).unwrap();
    assert_eq!(Err(RealError::Nan), (a / b).map(|x| x.0));
  }

  #[test]
  fn div_overflow() {
    let a = Mbf5::try_from(1.70141184e+37).unwrap();
    let b = Mbf5::try_from(0.1).unwrap();
    assert_eq!(Err(RealError::Infinite), (a / b).map(|x| x.0));
  }

  #[test]
  fn abs_pos() {
    let a = Mbf5::try_from(1.74).unwrap();
    assert_eq!(1.74, a.abs().0);
  }

  #[test]
  fn abs_zero() {
    let a = Mbf5::try_from(0.0).unwrap();
    assert_eq!(0.0, a.abs().0);
  }

  #[test]
  fn abs_neg() {
    let a = Mbf5::try_from(-54.8).unwrap();
    assert_eq!(54.8, a.abs().0);
  }

  #[test]
  fn sin_normal() {
    let a = Mbf5::try_from(617849.13).unwrap();
    assert_eq!(a.0.sin(), a.sin().0);
  }

  #[test]
  fn cos_normal() {
    let a = Mbf5::try_from(617849.13).unwrap();
    assert_eq!(a.0.cos(), a.cos().0);
  }

  #[test]
  fn tan_normal() {
    let a = Mbf5::try_from(1.74).unwrap();
    assert_eq!(Ok(1.74f64.tan()), a.tan().map(|x| x.0));
  }

  #[test]
  fn tan_large() {
    let a = Mbf5::try_from(std::f64::consts::FRAC_PI_2).unwrap();
    assert_eq!(Ok(16331239353195370.0), a.tan().map(|x| x.0));
  }

  #[test]
  fn ln_normal() {
    let a = Mbf5::try_from(135.16).unwrap();
    assert_eq!(Ok(a.0.ln()), a.ln().map(|x| x.0));
  }

  #[test]
  fn ln_zero() {
    let a = Mbf5::try_from(0.0).unwrap();
    assert_eq!(Err(RealError::Infinite), a.ln().map(|x| x.0));
  }

  #[test]
  fn ln_neg() {
    let a = Mbf5::try_from(-14.1).unwrap();
    assert_eq!(Err(RealError::Nan), a.ln().map(|x| x.0));
  }

  #[test]
  fn parse_int() {
    assert_eq!(
      "123".parse::<Mbf5>().map(|num| num.to_string()),
      Ok("123".to_owned())
    );
  }

  #[test]
  fn parse_fraction_1() {
    assert_eq!(
      "+123.456".parse::<Mbf5>().map(|num| num.to_string()),
      Ok("123.456".to_owned())
    );
  }

  #[test]
  fn parse_fraction_2() {
    assert_eq!(
      ".0078125".parse::<Mbf5>().map(|num| num.to_string()),
      Ok("7.8125E-03".to_owned())
    );
  }

  #[test]
  fn parse_fraction_3() {
    assert_eq!(
      "-.0625".parse::<Mbf5>().map(|num| num.to_string()),
      Ok("-.0625".to_owned())
    );
  }

  #[test]
  fn parse_fraction_4() {
    assert_eq!(
      "5.7203".parse::<Mbf5>().map(|num| num.to_string()),
      Ok("5.7203".to_owned())
    );
  }

  #[test]
  fn parse_exponent() {
    assert_eq!(
      "-123e23".parse::<Mbf5>().map(|num| num.to_string()),
      Ok("-1.23E+25".to_owned())
    );
  }

  #[test]
  fn parse_positive_exponent() {
    assert_eq!(
      "123E+23".parse::<Mbf5>().map(|num| num.to_string()),
      Ok("1.23E+25".to_owned())
    );
  }

  #[test]
  fn parse_negative_exponent() {
    assert_eq!(
      "123e-23".parse::<Mbf5>().map(|num| num.to_string()),
      Ok("1.23E-21".to_owned())
    );
  }

  #[test]
  fn parse_empty() {
    assert_eq!(
      "".parse::<Mbf5>().map(|num| num.to_string()),
      Err(ParseRealError::Malformed),
    );
  }

  #[test]
  fn parse_redundant_chars() {
    assert_eq!(
      "123abc".parse::<Mbf5>().map(|num| num.to_string()),
      Err(ParseRealError::Malformed),
    );
  }

  #[test]
  fn parse_missing_integral_part() {
    assert_eq!(
      ".23".parse::<Mbf5>().map(|num| num.to_string()),
      Ok(".23".to_owned())
    );
  }

  #[test]
  fn parse_missing_fractional_part() {
    assert_eq!(
      "12.".parse::<Mbf5>().map(|num| num.to_string()),
      Ok("12".to_owned())
    );
  }

  #[test]
  fn parse_missing_exponent_1() {
    assert_eq!(
      "12.e".parse::<Mbf5>().map(|num| num.to_string()),
      Ok("12".to_owned())
    );
  }

  #[test]
  fn parse_missing_exponent_2() {
    assert_eq!(
      "12.e-".parse::<Mbf5>().map(|num| num.to_string()),
      Ok("12".to_owned())
    );
  }

  #[test]
  fn parse_missing_exponent_3() {
    assert_eq!(
      "+12.e+".parse::<Mbf5>().map(|num| num.to_string()),
      Ok("12".to_owned())
    );
  }

  #[test]
  fn parse_missing_all_parts() {
    assert_eq!(
      "-.e+".parse::<Mbf5>().map(|num| num.to_string()),
      Ok("0".to_owned())
    );
  }

  #[test]
  fn parse_infinite() {
    assert_eq!(
      "1e300".parse::<Mbf5>().map(|num| num.to_string()),
      Err(ParseRealError::Infinite)
    );
  }
}
