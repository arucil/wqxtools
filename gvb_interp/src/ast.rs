use smallvec::{Array, SmallVec};
use std::fmt::{self, Debug, Formatter};

pub mod expr;
pub mod line;
pub mod stmt;
pub mod token;
pub mod node;
pub mod label;

pub use self::line::*;
pub use self::expr::*;
pub use self::stmt::*;
pub use self::token::*;
pub use self::node::*;
pub use self::label::*;

#[derive(Debug, Clone)]
pub struct Program {
  pub lines: Vec<ProgramLine>,
}

pub struct NonEmptyVec<T: Array>(pub SmallVec<T>);

#[derive(Debug, Clone, PartialEq, Eq)]
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

impl Range {
  pub fn new(start: usize, end: usize) -> Self {
    assert!(start <= end);
    Self { start, end }
  }

  pub fn is_empty(&self) -> bool {
    self.start == self.end
  }
}