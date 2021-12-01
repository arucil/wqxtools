#![feature(box_syntax, slice_ptr_get, io_error_more)]

pub mod array;
pub mod gvb;
pub mod string;
pub mod types;
pub mod config;

pub use self::array::*;
pub use self::gvb::*;
pub use self::string::*;
pub use self::types::*;
pub use self::config::*;

#[repr(C)]
pub struct Unit(pub i32);

impl Unit {
  fn new() -> Self {
    Self(0)
  }
}
