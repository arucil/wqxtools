#![feature(box_syntax, slice_ptr_get, io_error_more)]

pub mod array;
pub mod document;
pub mod either;
pub mod string;
pub mod device;

pub use self::array::*;
pub use self::document::*;
pub use self::either::*;
pub use self::string::*;
pub use self::device::*;

#[repr(C)]
pub struct Unit(pub i32);

impl Unit {
  fn new() -> Self {
    Self(0)
  }
}