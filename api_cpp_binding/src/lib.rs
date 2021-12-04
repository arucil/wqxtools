#![feature(box_syntax, slice_ptr_get, io_error_more)]

pub mod array;
pub mod config;
pub mod gvb;
pub mod string;
pub mod types;

pub use self::array::*;
pub use self::config::*;
pub use self::gvb::*;
pub use self::string::*;
pub use self::types::*;

#[repr(C)]
pub struct Unit(pub i32);

impl Unit {
  fn new() -> Self {
    Self(0)
  }
}
