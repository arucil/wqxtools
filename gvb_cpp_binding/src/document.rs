use crate::{Utf8Str, Utf8String, Utf16Str, ByteSlice, Either};
use gvb_interp as gvb;
use std::io;
use std::os::raw::c_ushort;

pub struct Document(gvb::Document);

#[no_mangle]
pub extern "C" fn load_document(
  path: Utf16Str,
) -> Either<Utf8String, *mut Document> {
  let path = unsafe { path.to_string() }.unwrap();
  match gvb::Document::load(path) {
    Ok(doc) => {
      Either::Right(Box::into_raw(box Document(doc)))
    },
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
        gvb::DocumentError::UnknownExt(Some(_)) => {
          format!("无法识别的后缀名")
        }
        gvb::DocumentError::UnknownExt(None) => {
          format!("文件缺少后缀名")
        }
      };
      Either::Left(unsafe { Utf8String::new(msg) })
    }
  }
}

#[no_mangle]
pub extern "C" fn destroy_document(doc: *mut Document) {
  drop(unsafe { Box::from_raw(doc) });
}

#[no_mangle]
pub extern "C" fn document_text(doc: *mut Document) -> Utf8Str {
  let text = unsafe { (*doc).0.text() };
  Utf8Str {
    data: text.as_bytes().as_ptr() as *const u8 as *const _,
    len: text.len(),
  }
}
