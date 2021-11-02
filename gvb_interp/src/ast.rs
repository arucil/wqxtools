use crate::parser::ParseResult;
use smallvec::{Array, SmallVec};
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
  pub start: usize,
  pub end: usize,
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
  pub fn new(start: usize, end: usize) -> Self {
    assert!(start <= end);
    Self { start, end }
  }

  pub fn is_empty(&self) -> bool {
    self.start == self.end
  }

  pub fn len(&self) -> usize {
    self.end - self.start
  }
}

impl Debug for Range {
  fn fmt(&self, f: &mut Formatter) -> fmt::Result {
    f.pad(&format!("{}..{}", self.start, self.end))
  }
}
