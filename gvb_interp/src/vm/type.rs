use std::ops::{Deref, DerefMut};

use crate::machine::EmojiStyle;

#[derive(Debug, Clone)]
pub struct ByteString(Vec<u8>);

impl Deref for ByteString {
  type Target = Vec<u8>;

  fn deref(&self) -> &Self::Target {
    &self.0
  }
}

impl DerefMut for ByteString {
  fn deref_mut(&mut self) -> &mut Self::Target {
    &mut self.0
  }
}

#[derive(Debug, Clone)]
pub enum StringError {
  InvalidChar(char),
  TooLong,
}

impl ByteString {
  pub fn new() -> Self {
    Self(vec![])
  }

  pub fn from_str<S: AsRef<str>>(
    str: S,
    emoji_style: EmojiStyle,
  ) -> Result<Self, StringError> {
    let str = str.as_ref();
    let mut bytes = vec![];
    for c in str.chars() {
      let b = c as u32;
      if b < 256 {
        bytes.push(b as u8);
      } else if let Some(&c) = crate::gb2312::UNICODE_TO_GB2312.get(&(b as u16))
      {
        bytes.push(0x1f);
        bytes.push((c >> 8) as u8);
        bytes.push(c as u8);
      } else if let Some(c) = emoji_style.char_to_code(c) {
        bytes.push(0x1f);
        bytes.push((c >> 8) as u8);
        bytes.push(c as u8);
      } else {
        return Err(StringError::InvalidChar(c));
      }
    }
    if bytes.len() > 255 {
      return Err(StringError::TooLong);
    }
    Ok(Self(bytes))
  }

  pub fn append(&mut self, other: &mut Self) {
    self.0.append(&mut other.0);
  }
}

impl From<Vec<u8>> for ByteString {
  fn from(x: Vec<u8>) -> Self {
    Self(x)
  }
}