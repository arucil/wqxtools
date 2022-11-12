use crate::parser::ParseResult;
use smallvec::{Array, SmallVec};
use std::cmp::Ordering;
use std::fmt::{self, Debug, Formatter};
use std::num::NonZeroUsize;
use std::ops::{Deref, DerefMut};

pub mod expr;
pub mod label;
pub mod line;
pub mod node;
pub mod stmt;
pub mod token;

pub use self::expr::*;
pub use self::label::*;
pub use self::line::*;
pub use self::node::*;
pub use self::stmt::*;
pub use self::token::*;

#[derive(Clone)]
pub struct Program {
  pub lines: Vec<ParseResult<ProgramLine>>,
}

pub struct NonEmptyVec<T: Array>(pub SmallVec<T>);

#[derive(Clone, PartialEq, Eq)]
pub struct Range {
  pub start: CharIndex,
  pub end: CharIndex,
}

#[derive(Clone, Eq, Ord)]
pub struct CharIndex {
  pub utf8: u32,
  pub utf16: u32,
}

#[derive(Clone, PartialEq, Eq)]
pub struct CharOffset {
  pub utf8: i32,
  pub utf16: i32,
}

impl<T: Array> PartialEq for NonEmptyVec<T>
where
  T::Item: PartialEq,
{
  fn eq(&self, other: &Self) -> bool {
    self.0.eq(&other.0)
  }
}

impl<T: Array> Eq for NonEmptyVec<T> where T::Item: Eq {}

impl<T: Array> NonEmptyVec<T> {
  pub fn len(&self) -> NonZeroUsize {
    unsafe { NonZeroUsize::new_unchecked(self.0.len()) }
  }
}

impl<T: Array> Clone for NonEmptyVec<T>
where
  T::Item: Clone,
{
  fn clone(&self) -> Self {
    Self(self.0.clone())
  }
}

impl<T: Array> Debug for NonEmptyVec<T>
where
  T::Item: Debug,
{
  fn fmt(&self, f: &mut Formatter) -> fmt::Result {
    self.0.fmt(f)
  }
}

#[cfg(test)]
impl Program {
  pub fn to_string(&self, text: &str) -> String {
    let mut buf = String::new();
    let mut offset = 0;
    for line in &self.lines {
      buf += &line.to_string(&text[offset..offset + line.content.source_len]);
      buf += "==================================\n";
      offset += line.content.source_len;
    }
    buf
  }
}

impl<T: Array> NonEmptyVec<T> {
  pub fn new() -> Self {
    Self(SmallVec::new())
  }
}

impl<T: Array> Deref for NonEmptyVec<T> {
  type Target = SmallVec<T>;
  fn deref(&self) -> &Self::Target {
    &self.0
  }
}

impl<T: Array> DerefMut for NonEmptyVec<T> {
  fn deref_mut(&mut self) -> &mut Self::Target {
    &mut self.0
  }
}

impl Range {
  pub fn new(start: CharIndex, end: CharIndex) -> Self {
    assert!(start <= end);
    Self { start, end }
  }

  pub fn new_ascii(start: u32, end: u32) -> Self {
    assert!(start <= end);
    Self {
      start: CharIndex::new_ascii(start),
      end: CharIndex::new_ascii(end),
    }
  }

  pub fn empty(start: CharIndex) -> Self {
    Self {
      start: start.clone(),
      end: start,
    }
  }

  pub fn start() -> Self {
    Self {
      start: CharIndex::start(),
      end: CharIndex::start(),
    }
  }

  pub fn empty_ascii(start: u32) -> Self {
    Self {
      start: CharIndex::new_ascii(start),
      end: CharIndex::new_ascii(start),
    }
  }

  pub fn is_empty(&self) -> bool {
    self.start == self.end
  }

  pub fn utf8_len(&self) -> u32 {
    self.end.utf8 - self.start.utf8
  }

  pub fn utf16_len(&self) -> u32 {
    self.end.utf16 - self.start.utf16
  }

  pub fn offset(&self, offset: CharOffset) -> Self {
    Self {
      start: self.start.offset(offset),
      end: self.end.offset(offset),
    }
  }

  pub fn utf8_range(&self) -> std::ops::Range<usize> {
    (self.start.utf8 as usize)..(self.end.utf8 as usize)
  }
}

impl Debug for Range {
  fn fmt(&self, f: &mut Formatter) -> fmt::Result {
    f.pad(&format!("{:?}..{:?}", self.start, self.end))
  }
}

impl CharIndex {
  pub fn new_ascii(i: u32) -> Self {
    Self { utf8: i, utf16: i }
  }

  pub fn start() -> Self {
    Self { utf8: 0, utf16: 0 }
  }

  pub fn offset(&self, offset: CharOffset) -> Self {
    Self {
      utf8: self.utf8 + offset.utf8 as u32,
      utf16: self.utf16 + offset.utf16 as u32,
    }
  }

  pub fn end_of<S>(s: S) -> Self
  where
    S: AsRef<str>,
  {
    let s = s.as_ref();
    Self {
      utf8: s.len() as u32,
      utf16: s.encode_utf16().count() as u32,
    }
  }
}

impl Debug for CharIndex {
  fn fmt(&self, f: &mut Formatter) -> fmt::Result {
    f.pad(&format!("({}|{})", self.utf8, self.utf16))
  }
}

impl PartialEq for CharIndex {
  fn eq(&self, other: &Self) -> bool {
    self.utf8 == other.utf8
  }
}

impl PartialOrd for CharIndex {
  fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
    self.utf8.partial_cmp(&other.utf8)
  }
}

impl From<char> for CharOffset {
  fn from(value: char) -> Self {
    Self {
      utf8: value.len_utf8() as i32,
      utf16: value.len_utf16() as i32,
    }
  }
}

impl CharOffset {
  pub fn new_ascii(n: i32) -> Self {
    Self { utf8: n, utf16: n }
  }
}
