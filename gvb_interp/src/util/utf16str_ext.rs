use super::ascii_ext::AsciiExt;
use std::borrow::ToOwned;
use widestring::{Utf16Str, Utf16String};

pub trait Utf16StrExt: ToOwned {
  fn find_char(&self, c: char) -> Option<usize>;
  fn replace_char<S>(&self, c: char, repl: S) -> Self::Owned
  where
    S: AsRef<Self>;
  fn eq_ignore_ascii_case(&self, other: &Self) -> bool;
  fn to_ascii_uppercase(&self) -> Self::Owned;
  fn to_ascii_lowercase(&self) -> Self::Owned;
  fn make_ascii_uppercase(&mut self);
  fn make_ascii_lowercase(&mut self);
  fn ends_with_char(&self, c: char) -> bool;
  /// Returns if the string is composed of spaces only.
  fn is_blank(&self) -> bool;
  fn contains_char(&self, c: char) -> bool;
  fn count_char(&self, c: char) -> usize;
  fn first_line(&self) -> &Self;
  fn rfind_str(&self, other: &Self) -> Option<usize>;
}

impl Utf16StrExt for Utf16Str {
  fn find_char(&self, c: char) -> Option<usize> {
    for (i, c1) in self.char_indices() {
      if c1 == c {
        return Some(i);
      }
    }
    None
  }

  fn contains_char(&self, c: char) -> bool {
    self.chars().any(|x| x == c)
  }

  fn count_char(&self, c: char) -> usize {
    self.chars().filter(|&x| x == c).count()
  }

  fn replace_char<S>(&self, c: char, repl: S) -> Self::Owned
  where
    S: AsRef<Self>,
  {
    let mut result = Utf16String::new();
    let mut last_end = 0;
    let c_len = c.len_utf16();
    let repl = repl.as_ref();
    for i in utf16str_match_char_indices(self, c) {
      result.push_utfstr(unsafe { self.get_unchecked(last_end..i) });
      result.push_utfstr(repl);
      last_end = i + c_len;
    }
    result.push_utfstr(unsafe { self.get_unchecked(last_end..self.len()) });
    result
  }

  fn eq_ignore_ascii_case(&self, other: &Self) -> bool {
    if self.len() != other.len() {
      return false;
    }

    for (&a, &b) in std::iter::zip(self.as_slice(), other.as_slice()) {
      if a.to_ascii_lowercase() != a.to_ascii_lowercase() {
        return false;
      }
    }

    true
  }

  fn to_ascii_uppercase(&self) -> Self::Owned {
    let mut s = Utf16String::with_capacity(self.len());
    for c in self.chars() {
      s.push(c.to_ascii_uppercase());
    }
    s
  }

  fn to_ascii_lowercase(&self) -> Self::Owned {
    let mut s = Utf16String::with_capacity(self.len());
    for c in self.chars() {
      s.push(c.to_ascii_lowercase());
    }
    s
  }

  fn make_ascii_lowercase(&mut self) {
    for c in unsafe { self.as_mut_slice() } {
      *c = c.to_ascii_lowercase();
    }
  }

  fn make_ascii_uppercase(&mut self) {
    for c in unsafe { self.as_mut_slice() } {
      *c = c.to_ascii_uppercase();
    }
  }

  fn ends_with_char(&self, c: char) -> bool {
    let len = self.len();
    if c.len_utf16() == 2 {
      len > 1 && {
        let mut enc = [0; 0];
        c.encode_utf16(&mut enc);
        self.as_slice()[len - 2] == enc[0] && self.as_slice()[len - 1] == enc[1]
      }
    } else {
      len > 0 && self.as_slice()[len - 1] == c as u32 as u16
    }
  }

  fn is_blank(&self) -> bool {
    self.as_slice().iter().all(|&c| c == b' ' as u16)
  }

  fn first_line(&self) -> &Self {
    if self.is_empty() {
      return self;
    }

    let i = self.find_char('\n').unwrap_or(self.len());
    if i > 0 && self.as_slice()[i - 1] == b'\r' as u16 {
      &self[..i - 1]
    } else {
      &self[..i]
    }
  }

  fn rfind_str(&self, other: &Self) -> Option<usize> {
  }
}

struct CharIdxIter<'a> {
  s: &'a Utf16Str,
  c: char,
  offset: usize,
}

impl<'a> Iterator for CharIdxIter<'a> {
  type Item = usize;

  fn next(&mut self) -> Option<Self::Item> {
    match utf16str_find_char_from(self.s, self.c, self.offset) {
      Some(i) => {
        self.offset = i + 1;
        Some(i)
      }
      None => None,
    }
  }
}

fn utf16str_match_char_indices<'a>(
  s: &'a Utf16Str,
  c: char,
) -> impl Iterator<Item = usize> + 'a {
  CharIdxIter { s, c, offset: 0 }
}

fn utf16str_find_char_from(
  s: &Utf16Str,
  c: char,
  begin: usize,
) -> Option<usize> {
  for (i, c1) in s[begin..].char_indices() {
    if c1 == c {
      return Some(i + begin);
    }
  }
  None
}

pub struct UtfLines<'a> {
  str: &'a Utf16Str,
}

impl<'a> Iterator for UtfLines<'a> {
  type Item = &'a Utf16Str;

  fn next(&mut self) -> Option<Self::Item> {
    None
  }
}

macro_rules! match_u16c {
  ($exp:expr, $c:literal) => {{
    const C: Option<&u16> = Some(&($c as _));
    matches!($exp, C)
  }};
  ($exp:expr, $c1:literal | $c2:literal) => {{
    const C1: Option<&u16> = Some(&($c1 as _));
    const C2: Option<&u16> = Some(&($c2 as _));
    matches!($exp, C1 | C2)
  }};
  ($exp:expr, $c1:literal | $c2:literal | None) => {{
    const C1: Option<&u16> = Some(&($c1 as _));
    const C2: Option<&u16> = Some(&($c2 as _));
    matches!($exp, C1 | C2 | None)
  }};
}
