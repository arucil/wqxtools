use smallvec::{Array, SmallVec};
use std::fmt::{self, Alignment, Debug, Formatter};
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

#[derive(Debug, Clone)]
pub struct Program {
  pub lines: Vec<ProgramLine>,
}

pub struct NonEmptyVec<T: Array>(pub SmallVec<T>);

#[derive(Clone, PartialEq, Eq)]
pub struct Range {
  pub start: usize,
  pub end: usize,
}

impl<T> Clone for NonEmptyVec<T>
where
  T: Array,
  T::Item: Clone,
{
  fn clone(&self) -> Self {
    Self(self.0.clone())
  }
}

impl<T> Debug for NonEmptyVec<T>
where
  T: Array,
  T::Item: Debug,
{
  fn fmt(&self, f: &mut Formatter) -> fmt::Result {
    self.0.fmt(f)
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
    let s = format!("{}..{}", self.start, self.end);
    if let Some(w) = f.width() {
      if w > s.len() {
        match f.align() {
          Some(Alignment::Right) | None => {
            return write!(
              f,
              "{}{}",
              f.fill().to_string().repeat(w - s.len()),
              s
            )
          }
          Some(Alignment::Left) => {
            return write!(
              f,
              "{}{}",
              s,
              f.fill().to_string().repeat(w - s.len()),
            )
          }
          Some(Alignment::Center) => {
            return write!(
              f,
              "{}{}{}",
              f.fill().to_string().repeat((w - s.len()) / 2),
              s,
              f.fill().to_string().repeat((w - s.len() + 1) / 2),
            )
          }
        }
      }
    }
    write!(f, "{}", s)
  }
}
