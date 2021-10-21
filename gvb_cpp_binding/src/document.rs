use crate::{CStr, CString, Either};
use gvb_interp as gvb;
use std::io;
use std::os::raw::c_ushort;

pub struct Document(gvb::Document);

#[no_mangle]
pub extern "C" fn load_document(
  path: *const c_ushort,
  len: usize,
) -> Either<CString, *mut Document> {
  let path = String::from_utf16(unsafe {
    std::slice::from_raw_parts(path as *const u16, len)
  })
  .unwrap();
  match gvb::Document::load(path) {
    Ok(doc) => Either::Right(Box::into_raw(box Document(doc))),
    Err(err) => {
      let msg = match err {
        gvb::DocumentError::Io(err) => match err.kind() {
          io::ErrorKind::PermissionDenied => format!("无权限"),
          io::ErrorKind::NotFound => format!("文件不存在"),
          io::ErrorKind::IsADirectory => format!("是文件夹"),
          _ => err.to_string(),
        },
        gvb::DocumentError::LoadBas(err) => {
          format!("文件偏移: {}, 错误信息: {}", err.location, err.message)
        }
        gvb::DocumentError::LoadTxt(err) => {
          format!("第 {} 行，错误信息: {}", err.location.0 + 1, err.message)
        }
        gvb::DocumentError::UnknownExt(_) => {
          format!("无法识别的后缀名")
        }
      };
      Either::Left(unsafe { CString::new(msg) })
    }
  }
}

#[no_mangle]
pub extern "C" fn destroy_document(doc: *mut Document) {
  drop(unsafe { Box::from_raw(doc) });
}

#[no_mangle]
pub extern "C" fn document_text(doc: *mut Document) -> CStr {
  let text = unsafe { (*doc).0.text() };
  CStr {
    data: text.as_bytes().as_ptr() as *const u8 as *const _,
    len: text.len(),
  }
}
