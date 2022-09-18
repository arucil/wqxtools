#![feature(
  exclusive_range_pattern,
  extend_one,
  const_mut_refs,
  never_type,
  io_error_more,
  const_maybe_uninit_assume_init
)]
#![allow(clippy::needless_late_init, clippy::useless_format, clippy::single_match)]

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
type HashMapEntry<'a, K, V> = std::collections::hash_map::Entry<'a, K, V>;

#[derive(Default)]
pub struct BuildSeaHasher;

impl hash::BuildHasher for BuildSeaHasher {
  type Hasher = seahash::SeaHasher;

  fn build_hasher(&self) -> Self::Hasher {
    seahash::SeaHasher::new()
  }
}
