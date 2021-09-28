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
pub struct StringError {
  char: char,
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
        bytes.push((c >> 8) as u8);
        bytes.push(c as u8);
      } else if let Some(c) = emoji_style.char_to_code(c) {
        bytes.push((c >> 8) as u8);
        bytes.push(c as u8);
      } else {
        return Err(StringError { char: c });
      }
    }
    Ok(Self(bytes))
  }
}
