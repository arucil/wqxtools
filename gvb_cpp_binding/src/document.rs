use crate::{
  Array, Device, Diagnostic, Either, Maybe, Severity, Unit, Utf16Str, Utf8Str,
  Utf8String, VirtualMachine,
};
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

pub type LoadDocumentResult = Either<Utf8String, *mut Document>;

#[no_mangle]
pub extern "C" fn load_document(path: Utf16Str) -> LoadDocumentResult {
  let path = unsafe { path.to_string() }.unwrap();
  match gvb::Document::load(path) {
    Ok(doc) => Either::Right(Box::into_raw(box Document(doc))),
    Err(err) => {
      let msg = match err {
        gvb::LoadDocumentError::Io(err) => io_error_to_string(err),
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

#[repr(C)]
pub struct SaveError {
  message: Utf8String,
  bas_specific: bool,
}

pub type SaveDocumentResult = Either<SaveError, Unit>;

#[no_mangle]
pub extern "C" fn save_document(
  doc: *mut Document,
  path: Utf16Str,
) -> SaveDocumentResult {
  let path = unsafe { path.to_string() }.unwrap();
  match unsafe { (*doc).0.save(path) } {
    Ok(()) => Either::Right(Unit::new()),
    Err(err) => {
      let (msg, bas_specific) = match err {
        gvb::SaveDocumentError::Io(err) => (io_error_to_string(err), false),
        gvb::SaveDocumentError::InvalidExt(Some(_)) => {
          (format!("无法识别的后缀名"), false)
        }
        gvb::SaveDocumentError::InvalidExt(None) => {
          (format!("文件缺少后缀名"), false)
        }
        gvb::SaveDocumentError::Save(err) => (
          format!("第 {} 行：{}", err.line, err.message),
          err.bas_specific,
        ),
      };
      Either::Left(SaveError {
        message: unsafe { Utf8String::new(msg) },
        bas_specific,
      })
    }
  }
}

#[no_mangle]
pub extern "C" fn document_device(doc: *mut Document) -> *mut Device {
  Box::into_raw(box Device(unsafe { (*doc).0.create_device() }))
}

#[no_mangle]
pub extern "C" fn document_vm(
  doc: *mut Document,
  device: *mut Device,
) -> Maybe<*mut VirtualMachine> {
  match unsafe { (*doc).0.create_vm(&mut (*device).0) } {
    Ok(vm) => Maybe::Just(Box::into_raw(box VirtualMachine(vm))),
    Err(()) => Maybe::Nothing,
  }
}

fn io_error_to_string(err: io::Error) -> String {
  match err.kind() {
    io::ErrorKind::PermissionDenied => format!("无权限"),
    io::ErrorKind::NotFound => format!("文件不存在"),
    io::ErrorKind::IsADirectory => format!("是文件夹"),
    _ => err.to_string(),
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

#[no_mangle]
pub extern "C" fn document_diagnostics(
  doc: *mut Document,
) -> Array<Diagnostic<Utf8Str>> {
  let line_diags = unsafe { (*doc).0.diagnostics() };
  let diags = line_diags
    .into_iter()
    .enumerate()
    .flat_map(|(line, line_diag)| {
      let line_start = line_diag.line_start;
      line_diag.diagnostics.iter().map(move |diag| Diagnostic {
        line,
        start: line_start + diag.range.start,
        end: line_start + diag.range.end,
        message: unsafe { Utf8Str::new(&diag.message) },
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
pub extern "C" fn document_text(doc: *mut Document) -> Utf8Str {
  unsafe {
    let text = (*doc).0.text();
    Utf8Str::new(text)
  }
}