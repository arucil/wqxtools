#![feature(
  exclusive_range_pattern,
  extend_one,
  const_mut_refs,
  never_type,
  io_error_more,
  int_abs_diff,
  const_maybe_uninit_assume_init,
  maybe_uninit_extra
)]

mod ast;
mod compiler;
pub mod device;
pub mod diagnostic;
pub mod document;
pub mod machine;
mod parser;
pub mod util;
pub mod vm;

pub use self::diagnostic::*;
pub use self::document::*;
pub use self::vm::*;

mod gb2312 {
  include!(concat!(env!("OUT_DIR"), "/gb2312.rs"));
}

use std::hash;

type HashMap<K, V> = std::collections::HashMap<K, V, BuildSeaHasher>;

#[derive(Default)]
pub struct BuildSeaHasher;

impl hash::BuildHasher for BuildSeaHasher {
  type Hasher = seahash::SeaHasher;

  fn build_hasher(&self) -> Self::Hasher {
    seahash::SeaHasher::new()
  }
}
