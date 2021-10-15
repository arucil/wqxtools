use std::os::raw::c_char;

#[repr(C)]
pub struct CString {
  pub data: *const c_char,
  pub len: usize,
}

pub(crate) fn new_cstring(str: String) -> CString {
  let len = str.len();
  let ptr = Box::into_raw(Box::<[u8]>::from(str.into_boxed_str())).as_mut_ptr();
  CString {
    data: ptr as *const c_char,
    len,
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
