use std::ops::{Deref, DerefMut};

use crate::machine::EmojiStyle;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
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

  pub fn to_string_lossy(&self, emoji_style: EmojiStyle) -> String {
    let mut s = String::new();
    let mut i = 0;
    while i < self.len() {
      let b = self[i];
      if b < 128 {
        s.push(b as char);
        i += 1;
      } else if i < self.len() - 1 {
        let b2 = self[i + 1];
        i += 2;
        let code = ((b as u16) << 8) + b2 as u16;
        if let Some(&c) = crate::gb2312::GB2312_TO_UNICODE.get(&code) {
          s.push(unsafe { char::from_u32_unchecked(c as u32) });
        } else if let Some(c) = emoji_style.code_to_char(code) {
          s.push(c);
        } else {
          s.push(char::REPLACEMENT_CHARACTER);
        }
      } else {
        s.push(char::REPLACEMENT_CHARACTER);
        i += 1;
      }
    }
    s
  }

  pub fn append(&mut self, other: &mut Self) {
    self.0.append(&mut other.0);
  }

  pub fn to_print_form(&self) -> ByteString {
    let mut buf = vec![];
    let mut v = 0;
    for &b in &self.0 {
      if v > 0 {
        buf.push(b);
        v -= 1;
      } else if b == 0x1f {
        v = 2;
      } else if b == 0 {
        break;
      } else {
        buf.push(b);
      }
    }
    Self(buf)
  }
}

impl From<Vec<u8>> for ByteString {
  fn from(x: Vec<u8>) -> Self {
    Self(x)
  }
}
