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
pub mod vm;

mod gb2312 {
  include!(concat!(env!("OUT_DIR"), "/gb2312.rs"));
}

use fasthash::{RandomState, metro::Hash64_1};

type HashMap<K, V> = std::collections::HashMap<K, V, RandomState<Hash64_1>>;