use crate::{destroy_string, Array, Either, Utf16Str, Utf8Str, Utf8String};
use gvb_interp as gvb;
use std::io;

pub struct Document(gvb::Document);

#[repr(C)]
pub struct InsertText {
  pub pos: usize,
  pub str: Utf8Str,
}

#[repr(C)]
pub struct DeleteText {
  pub pos: usize,
  pub len: usize,
}

#[no_mangle]
pub extern "C" fn load_document(
  path: Utf16Str,
) -> Either<Utf8String, *mut Document> {
  let path = unsafe { path.to_string() }.unwrap();
  match gvb::Document::load(path) {
    Ok(doc) => Either::Right(Box::into_raw(box Document(doc))),
    Err(err) => {
      let msg = match err {
        gvb::LoadDocumentError::Io(err) => match err.kind() {
          io::ErrorKind::PermissionDenied => format!("无权限"),
          io::ErrorKind::NotFound => format!("文件不存在"),
          io::ErrorKind::IsADirectory => format!("是文件夹"),
          _ => err.to_string(),
        },
        gvb::LoadDocumentError::LoadBas(err) => {
          format!("文件偏移: {}, 错误信息: {}", err.location, err.message)
        }
        gvb::LoadDocumentError::LoadTxt(err) => {
          format!("第 {} 行，错误信息: {}", err.location.0 + 1, err.message)
        }
        gvb::LoadDocumentError::UnknownExt(Some(_)) => {
          format!("无法识别的后缀名")
        }
        gvb::LoadDocumentError::UnknownExt(None) => {
          format!("文件缺少后缀名")
        }
      };
      Either::Left(unsafe { Utf8String::new(msg) })
    }
  }
}

#[no_mangle]
pub extern "C" fn create_document() -> *mut Document {
  Box::into_raw(box Document(gvb::Document::new()))
}

pub type Modification = Either<InsertText, DeleteText>;

#[no_mangle]
pub extern "C" fn document_apply_edit(doc: *mut Document, edit: Modification) {
  let edit = match edit {
    Either::Left(insert) => gvb::Edit {
      pos: insert.pos,
      kind: gvb::EditKind::Insert(unsafe { insert.str.as_str() }),
    },
    Either::Right(delete) => gvb::Edit {
      pos: delete.pos,
      kind: gvb::EditKind::Delete(delete.len),
    },
  };
  unsafe {
    (*doc).0.apply_edit(edit);
  }
}

#[repr(C)]
pub enum Severity {
  Warning,
  Error,
}

#[repr(C)]
pub struct Diagnostic {
  pub line: usize,
  pub start: usize,
  pub end: usize,
  pub message: Utf8String,
  pub severity: Severity,
}

#[no_mangle]
pub extern "C" fn document_diagnostics(
  doc: *mut Document,
) -> Array<Diagnostic> {
  let line_diags = unsafe { (*doc).0.diagnostics() };
  let diags = line_diags
    .into_iter()
    .enumerate()
    .flat_map(|(line, line_diag)| {
      let line_start = line_diag.line_start;
      line_diag
        .diagnostics
        .into_iter()
        .map(move |diag| Diagnostic {
          line,
          start: line_start + diag.range.start,
          end: line_start + diag.range.end,
          message: unsafe { Utf8String::new(diag.message) },
          severity: match diag.severity {
            gvb::Severity::Warning => Severity::Warning,
            gvb::Severity::Error => Severity::Error,
          },
        })
    })
    .collect();
  unsafe { Array::new(diags) }
}

#[no_mangle]
pub extern "C" fn destroy_document(doc: *mut Document) {
  drop(unsafe { Box::from_raw(doc) });
}

#[no_mangle]
pub extern "C" fn document_text(doc: *mut Document) -> Utf8String {
  unsafe {
    let text = (*doc).0.text();
    Utf8String::new(text)
  }
}

#[no_mangle]
pub extern "C" fn destroy_diagnostic_array(arr: Array<Diagnostic>) {
  let diags = unsafe {
    Box::from_raw(std::ptr::slice_from_raw_parts_mut(
      arr.data as *const _ as *mut Diagnostic,
      arr.len,
    ))
  };
  for diag in diags.iter() {
    destroy_string(diag.message.clone());
  }
}
