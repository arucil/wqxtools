#![feature(
  exclusive_range_pattern,
  extend_one,
  const_mut_refs,
  never_type,
  io_error_more
)]

mod ast;
mod compiler;
pub mod diagnostic;
pub mod document;
mod machine;
mod parser;
pub mod util;
pub mod vm;

pub use self::document::*;
pub use self::vm::*;
pub use self::diagnostic::*;

mod gb2312 {
  include!(concat!(env!("OUT_DIR"), "/gb2312.rs"));
}

use fasthash::{metro::Hash64_1, RandomState};

type HashMap<K, V> = std::collections::HashMap<K, V, RandomState<Hash64_1>>;
