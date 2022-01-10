use crate::{Array, ArrayMut};
use std::{
  os::raw::{c_char, c_ushort},
  string::FromUtf16Error,
};

#[repr(C)]
pub struct Utf16Str {
  pub data: *const c_ushort,
  pub len: usize,
}

#[repr(C)]
pub struct ByteSlice {
  pub data: *const c_char,
  pub len: usize,
}

#[repr(C)]
#[derive(Clone)]
pub struct Utf8String {
  pub data: *const c_char,
  pub len: usize,
}

#[repr(C)]
pub struct Utf8Str {
  pub data: *const c_char,
  pub len: usize,
}

impl Utf8String {
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

impl Utf8Str {
  pub(crate) unsafe fn new(str: &str) -> Self {
    let len = str.len();
    Self {
      data: str.as_ptr() as *const _,
      len,
    }
  }

  pub(crate) unsafe fn as_str<'a>(&self) -> &'a str {
    std::str::from_utf8_unchecked(std::slice::from_raw_parts(
      self.data as *const _,
      self.len,
    ))
  }
}

impl Utf16Str {
  pub(crate) unsafe fn to_string(&self) -> Result<String, FromUtf16Error> {
    String::from_utf16(std::slice::from_raw_parts(
      self.data as *const _,
      self.len,
    ))
  }
}

#[no_mangle]
pub extern "C" fn destroy_string(str: Utf8String) {
  if str.data.is_null() {
    return;
  }
  drop(unsafe {
    Box::from_raw(std::ptr::slice_from_raw_parts_mut(
      str.data as *mut u8,
      str.len,
    ))
  });
}

#[no_mangle]
pub extern "C" fn copy_byte_string(arr: Array<u8>) -> Array<u8> {
  unsafe { Array::new(arr.as_slice().to_vec()) }
}

#[no_mangle]
pub extern "C" fn destroy_byte_string(arr: Array<u8>) {
  if arr.data.is_null() {
    return;
  }
  drop(unsafe { arr.into_boxed_slice() });
}

#[no_mangle]
pub extern "C" fn destroy_byte_string_array_mut(arr: ArrayMut<Array<u8>>) {
  if arr.data.is_null() {
    return;
  }
  for s in unsafe { arr.into_boxed_slice() }.iter() {
    drop(unsafe { (*s).clone().into_boxed_slice() });
  }
}

#[no_mangle]
pub extern "C" fn destroy_str_array(arr: Array<Utf8Str>) {
  if arr.data.is_null() {
    return;
  }
  drop(unsafe { arr.into_boxed_slice() });
}
