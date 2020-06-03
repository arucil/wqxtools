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
//! ```
//! (-1)^sign * 0.1M * 2^E
//! ```
//! where `M` represents the 31-bit mantissa, `E` represents the exponent.
//!
//! The exponent is in excess-128 form, e.g. 0x80 means a exponent of 0, 0x7a means
//! a exponent of -6, 0x84 means a exponent of +4, etc.

/// Used for store floating point value of a variable.
pub struct Mbf5([u8; 5]);

/// Used for perform floating point calculations.
pub struct Mbf5Accum(f64);

impl From<Mbf5> for Mbf5Accum {
  fn from(x: Mbf5) -> Self {
  }
}