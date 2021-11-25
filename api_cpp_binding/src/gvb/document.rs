use crate::{
  Array, Either, GvbDevice, GvbDiagnostic, GvbSeverity, GvbVirtualMachine,
  Maybe, Unit, Utf16Str, Utf8Str, Utf8String,
};
use gvb_interp as gvb;
use std::io;

pub struct GvbDocument(gvb::Document);

#[repr(C)]
pub struct GvbInsertText {
  pub pos: usize,
  pub str: Utf8Str,
}

#[repr(C)]
pub struct GvbDeleteText {
  pub pos: usize,
  pub len: usize,
}

pub type GvbLoadDocumentResult = Either<Utf8String, *mut GvbDocument>;

#[no_mangle]
pub extern "C" fn gvb_load_document(path: Utf16Str) -> GvbLoadDocumentResult {
  let path = unsafe { path.to_string() }.unwrap();
  match gvb::Document::load(path) {
    Ok(doc) => Either::Right(Box::into_raw(box GvbDocument(doc))),
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
pub struct GvbSaveError {
  message: Utf8String,
  bas_specific: bool,
}

pub type GvbSaveDocumentResult = Either<GvbSaveError, Unit>;

#[no_mangle]
pub extern "C" fn gvb_save_document(
  doc: *mut GvbDocument,
  path: Utf16Str,
) -> GvbSaveDocumentResult {
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
      Either::Left(GvbSaveError {
        message: unsafe { Utf8String::new(msg) },
        bas_specific,
      })
    }
  }
}

#[no_mangle]
pub extern "C" fn gvb_document_device(
  doc: *mut GvbDocument,
  data_dir: Utf16Str,
) -> *mut GvbDevice {
  let data_dir = unsafe { data_dir.to_string() }.unwrap();
  Box::into_raw(box GvbDevice(unsafe { (*doc).0.create_device(data_dir) }))
}

#[no_mangle]
pub extern "C" fn gvb_document_vm(
  doc: *mut GvbDocument,
  device: *mut GvbDevice,
) -> Maybe<*mut GvbVirtualMachine> {
  match unsafe { (*doc).0.create_vm(&mut (*device).0) } {
    Ok(vm) => Maybe::Just(Box::into_raw(box GvbVirtualMachine(vm))),
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
pub extern "C" fn gvb_create_document() -> *mut GvbDocument {
  Box::into_raw(box GvbDocument(gvb::Document::new()))
}

pub type GvbModification = Either<GvbInsertText, GvbDeleteText>;

#[no_mangle]
pub extern "C" fn gvb_document_apply_edit(
  doc: *mut GvbDocument,
  edit: GvbModification,
) {
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
pub extern "C" fn gvb_document_diagnostics(
  doc: *mut GvbDocument,
) -> Array<GvbDiagnostic<Utf8Str>> {
  let line_diags = unsafe { (*doc).0.diagnostics() };
  let diags = line_diags
    .into_iter()
    .enumerate()
    .flat_map(|(line, line_diag)| {
      let line_start = line_diag.line_start;
      line_diag.diagnostics.iter().map(move |diag| GvbDiagnostic {
        line,
        start: line_start + diag.range.start,
        end: line_start + diag.range.end,
        message: unsafe { Utf8Str::new(&diag.message) },
        severity: match diag.severity {
          gvb::Severity::Warning => GvbSeverity::Warning,
          gvb::Severity::Error => GvbSeverity::Error,
        },
      })
    })
    .collect();
  unsafe { Array::new(diags) }
}

#[no_mangle]
pub extern "C" fn gvb_destroy_document(doc: *mut GvbDocument) {
  drop(unsafe { Box::from_raw(doc) });
}

#[no_mangle]
pub extern "C" fn gvb_document_text(doc: *mut GvbDocument) -> Utf8Str {
  unsafe {
    let text = (*doc).0.text();
    Utf8Str::new(text)
  }
}
