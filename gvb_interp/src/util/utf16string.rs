use std::slice::SliceIndex;

macro_rules! utf16str {
  ($text:expr) => {{
    const _WIDESTRING_U16_MACRO_UTF8: &$crate::internals::core::primitive::str =
      $text;
    const _WIDESTRING_U16_MACRO_LEN: $crate::internals::core::primitive::usize =
      $crate::internals::length_as_utf16(_WIDESTRING_U16_MACRO_UTF8)
        + $extra_len;
    const _WIDESTRING_U16_MACRO_UTF16:
      [$crate::internals::core::primitive::u16; _WIDESTRING_U16_MACRO_LEN] = {
      let mut _widestring_buffer: [$crate::internals::core::primitive::u16;
        _WIDESTRING_U16_MACRO_LEN] = [0; _WIDESTRING_U16_MACRO_LEN];
      let mut _widestring_bytes = _WIDESTRING_U16_MACRO_UTF8.as_bytes();
      let mut _widestring_i = 0;
      while let $crate::internals::core::option::Option::Some((
        _widestring_ch,
        _widestring_rest,
      )) = $crate::internals::next_code_point(_widestring_bytes)
      {
        _widestring_bytes = _widestring_rest;
        if $extra_len > 0 && _widestring_ch == 0 {
          panic!("invalid NUL value found in string literal");
        }
        // https://doc.rust-lang.org/std/primitive.char.html#method.encode_utf16
        if _widestring_ch & 0xFFFF == _widestring_ch {
          _widestring_buffer[_widestring_i] =
            _widestring_ch as $crate::internals::core::primitive::u16;
          _widestring_i += 1;
        } else {
          let _widestring_code = _widestring_ch - 0x1_0000;
          _widestring_buffer[_widestring_i] = 0xD800
            | ((_widestring_code >> 10)
              as $crate::internals::core::primitive::u16);
          _widestring_buffer[_widestring_i + 1] = 0xDC00
            | ((_widestring_code as $crate::internals::core::primitive::u16)
              & 0x3FF);
          _widestring_i += 2;
        }
      }
      _widestring_buffer
    };
    #[allow(unused_unsafe)]
    unsafe {
      $crate::$str::$fn(&_WIDESTRING_U16_MACRO_UTF16)
    }
  }};
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Utf16String(Vec<u16>);

#[derive(Debug, PartialEq, Eq)]
pub struct Utf16Str([u16]);

impl Utf16Str {
  pub fn find_char<'a, P>(&'a self, c: char) -> Option<usize> {
    for (i, c1) in self.char_indices() {
      if c1 == c {
        return Some(i);
      }
    }
    None
  }

  pub fn len(&self) -> usize {
    self.0.len()
  }

  pub fn as_slice(&self) -> &[u16] {
    &self.0
  }

  pub fn char_indices(&self) -> Utf16CharIndices {
    Utf16CharIndices {
      index: 0,
      iter: self.chars(),
    }
  }

  pub fn chars(&self) -> Utf16Chars {
    Utf16Chars {
      code_units: &self.0,
    }
  }

  pub const unsafe fn from_slice_unchecked(s: &[u16]) -> &Self {
    &*(s as *const [u16] as *const Self)
  }

  pub unsafe fn get_unchecked<I>(&self, index: I) -> &Self
  where
    I: SliceIndex<[u16], Output = [u16]>,
  {
    Self::from_slice_unchecked(self.0.get_unchecked(index))
  }
}

impl Utf16String {
  pub fn new() -> Self {
    Self(vec![])
  }
}

pub struct Utf16Chars<'a> {
  code_units: &'a [u16],
}

impl<'a> Iterator for Utf16Chars<'a> {
  type Item = char;

  fn next(&mut self) -> Option<Self::Item> {
    if self.code_units.is_empty() {
      None
    } else if self.code_units[0].is_utf16_surrogate() {
      let c = unsafe {
        char::from_u32_unchecked(
          (self.code_units[0] & 0x3ff) as u32
            + (self.code_units[1] & 0x3ff) as u32,
        )
      };
      self.code_units = &self.code_units[2..];
      Some(c)
    } else {
      let c = unsafe { char::from_u32_unchecked(self.code_units[0] as u32) };
      self.code_units = &self.code_units[1..];
      Some(c)
    }
  }
}

pub struct Utf16CharIndices<'a> {
  index: usize,
  iter: Utf16Chars<'a>,
}

impl<'a> Iterator for Utf16CharIndices<'a> {
  type Item = (usize, char);

  fn next(&mut self) -> Option<Self::Item> {
    let pre_len = self.iter.code_units.len();
    let c = self.iter.next()?;
    let i = self.index;
    self.index += pre_len - self.iter.code_units.len();
    Some((i, c))
  }
}
