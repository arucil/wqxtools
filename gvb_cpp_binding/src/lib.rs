#![feature(box_syntax, slice_ptr_get, io_error_more)]

pub mod document;
pub mod string;
pub mod either;

pub use self::document::*;
pub use self::string::*;
pub use self::either::*;