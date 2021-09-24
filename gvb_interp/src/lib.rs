#![allow(incomplete_features)]
#![feature(
  exclusive_range_pattern,
  let_chains,
  extend_one,
  const_panic,
  const_mut_refs
)]

pub mod ast;
mod compiler;
pub mod diagnostic;
pub mod document;
mod machine;
mod parser;
pub mod util;
