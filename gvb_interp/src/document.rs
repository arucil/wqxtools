use std::path::Path;
use std::io;
use std::fs;

mod binary;
mod emoji;
mod gb2312 {
  include!(concat!(env!("OUT_DIR"), "/gb2312.rs"));
}

pub struct Document {
  base_addr: u16,
}

impl Document {
  /// Load a `.BAS` or `.txt` file.
  pub fn load<P>(path: impl AsRef<Path>) -> io::Result<Self> {
    todo!()
  }
}