#![feature(box_syntax, slice_ptr_get, io_error_more)]

pub mod array;
pub mod document;
pub mod types;
pub mod string;
pub mod device;
pub mod vm;
pub mod diagnostic;

pub use self::array::*;
pub use self::document::*;
pub use self::types::*;
pub use self::string::*;
pub use self::device::*;
pub use self::vm::*;
pub use self::diagnostic::*;

#[repr(C)]
pub struct Unit(pub i32);

impl Unit {
  fn new() -> Self {
    Self(0)
  }
}