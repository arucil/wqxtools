use crate::{destroy_string, Diagnostic, Utf8String, Utf8Str};

#[repr(C)]
pub struct Array<T> {
  pub data: *const T,
  pub len: usize,
}

impl<T> Array<T> {
  pub(crate) unsafe fn new(v: Vec<T>) -> Self {
    let len = v.len();
    let data =
      Box::into_raw(Box::<[_]>::from(v.into_boxed_slice())).as_mut_ptr();
    Self { data, len }
  }

  pub(crate) unsafe fn as_slice<'a>(&self) -> &'a [T] {
    std::slice::from_raw_parts(self.data, self.len)
  }
}

#[no_mangle]
pub extern "C" fn destroy_string_diagnostic_array(
  arr: Array<Diagnostic<Utf8String>>,
) {
  for diag in unsafe {
    Box::from_raw(std::ptr::slice_from_raw_parts_mut(
      arr.data as *const _ as *mut Diagnostic<Utf8String>,
      arr.len,
    ))
  }
  .iter()
  {
    destroy_string(diag.message.clone());
  }
}

#[no_mangle]
pub extern "C" fn destroy_str_diagnostic_array(
  arr: Array<Diagnostic<Utf8Str>>,
) {
  drop(unsafe {
    Box::from_raw(std::ptr::slice_from_raw_parts_mut(
      arr.data as *const _ as *mut Diagnostic<Utf8Str>,
      arr.len,
    ))
  });
}
