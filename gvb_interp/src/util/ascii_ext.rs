
pub trait AsciiExt {
  fn to_ascii_uppercase(&self) -> Self;
  fn to_ascii_lowercase(&self) -> Self;
  fn is_ascii_lowercase(&self) -> bool;
  fn is_ascii_uppercase(&self) -> bool;
  fn is_ascii_digit(&self) -> bool;
  fn is_ascii_alphabetic(&self) -> bool;
  fn is_ascii_alphanumeric(&self) -> bool;
}

impl AsciiExt for u16 {
  fn to_ascii_lowercase(&self) -> u16 {
    *self | (self.is_ascii_uppercase() as u16 * 0b10_0000)
  }

  fn to_ascii_uppercase(&self) -> u16 {
    *self ^ (self.is_ascii_lowercase() as u16 * 0b10_0000)
  }

  fn is_ascii_uppercase(&self) -> bool {
    matches!(self, 65..=90)
  }

  fn is_ascii_lowercase(&self) -> bool {
    matches!(self, 97..=122)
  }

  fn is_ascii_alphanumeric(&self) -> bool {
    matches!(self, 65..=90 | 97..=122 | 48..=57)
  }

  fn is_ascii_digit(&self) -> bool {
    matches!(self, 48..=57)
  }

  fn is_ascii_alphabetic(&self) -> bool {
    matches!(self, 65..=90 | 97..=122)
  }
}