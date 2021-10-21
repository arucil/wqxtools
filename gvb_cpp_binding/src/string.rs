use std::os::raw::c_char;

#[repr(C)]
pub struct CString {
  pub data: *const c_char,
  pub len: usize,
}

#[repr(C)]
pub struct CStr {
  pub data: *const c_char,
  pub len: usize,
}

impl CString {
  pub(crate) unsafe fn new(str: String) -> Self {
    let len = str.len();
    let ptr =
      Box::into_raw(Box::<[u8]>::from(str.into_boxed_str())).as_mut_ptr();
    Self {
      data: ptr as *const c_char,
      len,
    }
  }
}

impl CStr {
  pub(crate) unsafe fn new(str: &str) -> Self {
    let len = str.len();
    Self {
      data: str.as_ptr() as *const _,
      len,
    }
  }
}

#[no_mangle]
pub extern "C" fn destroy_string(str: CString) {
  drop(unsafe {
    Box::from_raw(std::ptr::slice_from_raw_parts_mut(
      str.data as *mut u8,
      str.len,
    ))
  });
}
