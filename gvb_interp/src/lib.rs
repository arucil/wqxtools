#![allow(incomplete_features)]
#![feature(
  exclusive_range_pattern,
  let_chains,
  extend_one,
  const_panic,
  const_mut_refs
)]

pub mod ast;
pub mod diagnostic;
mod parser;
pub mod util;
pub mod document;
mod machine;
mod compiler;