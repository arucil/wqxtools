#![feature(box_syntax, slice_ptr_get, io_error_more)]

pub mod array;
pub mod string;
pub mod types;
pub mod gvb;

pub use self::array::*;
pub use self::string::*;
pub use self::types::*;
pub use self::gvb::*;

#[repr(C)]
pub struct Unit(pub i32);

impl Unit {
  fn new() -> Self {
    Self(0)
  }
}
