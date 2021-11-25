use crate::array::Array;
use crate::string::{Utf8String, Utf8Str, destroy_string};

#[repr(C)]
pub enum GvbSeverity {
  Warning,
  Error,
}

#[repr(C)]
pub struct GvbDiagnostic<M> {
  pub line: usize,
  pub start: usize,
  pub end: usize,
  pub message: M,
  pub severity: GvbSeverity,
}

#[no_mangle]
pub extern "C" fn gvb_destroy_string_diagnostic_array(
  arr: Array<GvbDiagnostic<Utf8String>>,
) {
  for diag in unsafe { arr.into_boxed_slice() }.iter() {
    destroy_string(diag.message.clone());
  }
}

#[no_mangle]
pub extern "C" fn gvb_destroy_str_diagnostic_array(
  arr: Array<GvbDiagnostic<Utf8Str>>,
) {
  drop(unsafe { arr.into_boxed_slice() });
}