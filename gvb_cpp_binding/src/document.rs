use crate::{CString, new_cstring};
use gvb_interp as gvb;
use std::os::raw::c_ushort;

pub struct Document(gvb::Document);

#[repr(C)]
pub struct LoadedDocument {
  pub doc: *mut Document,
  pub text: CString,
}

#[no_mangle]
pub extern "C" fn load_document(
  path: *const c_ushort,
  len: usize,
) -> LoadedDocument {
  let path = String::from_utf16(unsafe {
    std::slice::from_raw_parts(path as *const u16, len)
  })
  .unwrap();
  let doc = gvb::Document::load(path).unwrap();
  LoadedDocument {
    doc: Box::into_raw(box Document(doc)),
    text: CString::new(String::new())
  }
}

#[no_mangle]
pub extern "C" fn destroy_document(doc: *mut Document) {
  drop(unsafe { Box::from_raw(doc) });
}
