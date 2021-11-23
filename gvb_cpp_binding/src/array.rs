use crate::{destroy_string, Diagnostic, Utf8Str, Utf8String};

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

  pub(crate) unsafe fn into_boxed_slice(self) -> Box<[T]> {
    Box::from_raw(std::ptr::slice_from_raw_parts_mut(
      self.data as *const _ as *mut _,
      self.len,
    ))
  }

  pub(crate) unsafe fn as_slice<'a>(&self) -> &'a [T] {
    std::slice::from_raw_parts(self.data, self.len)
  }
}

#[no_mangle]
pub extern "C" fn destroy_string_diagnostic_array(
  arr: Array<Diagnostic<Utf8String>>,
) {
  for diag in unsafe { arr.into_boxed_slice() }.iter() {
    destroy_string(diag.message.clone());
  }
}

#[no_mangle]
pub extern "C" fn destroy_str_diagnostic_array(
  arr: Array<Diagnostic<Utf8Str>>,
) {
  drop(unsafe { arr.into_boxed_slice() });
}
