use crate::{
  destroy_string, Array, Either, GvbDevice, GvbDiagnostic, GvbSeverity,
  GvbVirtualMachine, Maybe, Unit, Utf16Str, Utf8Str, Utf8String,
};
use gvb_interp as gvb;
use std::io;

pub struct GvbDocument(gvb::Document);

#[repr(C)]
pub struct GvbInsertText<S> {
  pub pos: usize,
  pub str: S,
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
  match gvb::Document::load_file(path) {
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

#[no_mangle]
pub extern "C" fn gvb_create_document() -> *mut GvbDocument {
  Box::into_raw(box GvbDocument(gvb::Document::new()))
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
          format!("第 {} 行：{}", err.line + 1, err.message),
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

pub type GvbEdit = Either<GvbInsertText<Utf8Str>, GvbDeleteText>;

#[no_mangle]
pub extern "C" fn gvb_document_apply_edit(
  doc: *mut GvbDocument,
  edit: GvbEdit,
) {
  let edit = match edit {
    Either::Left(insert) => gvb::Edit {
      pos: insert.pos,
      kind: gvb::EditKind::Insert(unsafe { insert.str.as_str().into() }),
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

#[no_mangle]
pub extern "C" fn gvb_document_machine_name(doc: *mut GvbDocument) -> Utf8Str {
  unsafe { Utf8Str::new((*doc).0.machine_name()) }
}

#[repr(C)]
pub struct GvbReplaceChar {
  pub start: usize,
  pub end: usize,
  pub ch: char,
}

impl From<gvb::ReplaceChar> for GvbReplaceChar {
  fn from(r: gvb::ReplaceChar) -> Self {
    Self {
      start: r.range.start,
      end: r.range.end,
      ch: r.ch,
    }
  }
}

pub type GvbDocSyncMachResult = Either<Utf8String, Array<GvbReplaceChar>>;

#[no_mangle]
pub extern "C" fn gvb_document_sync_machine_name(
  doc: *mut GvbDocument,
) -> GvbDocSyncMachResult {
  match unsafe { (*doc).0.sync_machine_name() } {
    Ok(edits) => Either::Right(unsafe {
      Array::new(edits.into_iter().map(From::from).collect())
    }),
    Err(err) => Either::Left(mach_prop_error_to_string(err)),
  }
}

fn mach_prop_error_to_string(err: gvb::MachinePropError) -> Utf8String {
  match err {
    gvb::MachinePropError::NotFound(name) => unsafe {
      Utf8String::new(format!("不存在机型 {} 的配置信息", name))
    },
    gvb::MachinePropError::Save(err) => unsafe {
      Utf8String::new(format!(
        "转换源码时发生错误：第 {} 行：{}",
        err.line + 1,
        err.message
      ))
    },
    gvb::MachinePropError::Load(err) => unsafe {
      Utf8String::new(format!(
        "转换源码时发生错误：第 {} 行，错误信息: {}",
        err.location.0 + 1,
        err.message
      ))
    },
  }
}

#[repr(C)]
pub struct GvbReplaceText {
  pub start: usize,
  pub end: usize,
  pub str: Utf8String,
}

pub type GvbDocMachEditResult = Either<Utf8String, GvbReplaceText>;

#[no_mangle]
pub extern "C" fn gvb_document_machine_name_edit(
  doc: *mut GvbDocument,
  name: Utf8Str,
) -> GvbDocMachEditResult {
  let name = unsafe { name.as_str() };
  match unsafe { (*doc).0.compute_machine_name_edit(name) } {
    Ok(edit) => Either::Right(GvbReplaceText {
      start: edit.range.start,
      end: edit.range.end,
      str: unsafe { Utf8String::new(edit.str) },
    }),
    Err(err) => Either::Left(mach_prop_error_to_string(err)),
  }
}

#[no_mangle]
pub extern "C" fn gvb_destroy_replace_text(rep: GvbReplaceText) {
  destroy_string(rep.str);
}

#[no_mangle]
pub extern "C" fn gvb_destroy_replace_char_array(reps: Array<GvbReplaceChar>) {
  drop(unsafe { reps.into_boxed_slice() });
}

#[repr(C)]
pub enum GvbLabelTarget {
  CurLine,
  PrevLine,
  NextLine,
}

impl From<GvbLabelTarget> for gvb::LabelTarget {
  fn from(t: GvbLabelTarget) -> Self {
    match t {
      GvbLabelTarget::CurLine => Self::CurLine,
      GvbLabelTarget::PrevLine => Self::PrevLine,
      GvbLabelTarget::NextLine => Self::NextLine,
    }
  }
}

#[repr(C)]
pub struct GvbAddLabelResult {
  pub edit: GvbReplaceText,
  pub goto: Maybe<usize>,
}

pub type GvbDocLabelEditResult = Either<Utf8String, GvbAddLabelResult>;

#[no_mangle]
pub extern "C" fn gvb_document_add_label_edit(
  doc: *mut GvbDocument,
  target: GvbLabelTarget,
  position: usize,
) -> GvbDocLabelEditResult {
  match unsafe { (*doc).0.compute_add_label_edit(target.into(), position) } {
    Ok(result) => Either::Right(GvbAddLabelResult {
      edit: GvbReplaceText {
        start: result.edit.range.start,
        end: result.edit.range.end,
        str: unsafe { Utf8String::new(result.edit.str) },
      },
      goto: result.goto.into(),
    }),
    Err(gvb::AddLabelError::AlreadyHasLabel) => Either::Left(unsafe {
      Utf8String::new(format!("当前行已经有行号"))
    }),
    Err(gvb::AddLabelError::CannotInferLabel) => {
      Either::Left(unsafe { Utf8String::new(format!("无法推测行号")) })
    }
  }
}

#[repr(C)]
pub enum GvbDocRelabelError {
  LabelOverflow(u32),
  LabelNotFound {
    start: usize,
    end: usize,
    label: u16,
  },
}

pub type GvbDocRelabelResult =
  Either<GvbDocRelabelError, Array<GvbReplaceText>>;

#[no_mangle]
pub extern "C" fn gvb_document_relabel_edits(
  doc: *mut GvbDocument,
  start: u16,
  inc: u16,
) -> GvbDocRelabelResult {
  match unsafe { (*doc).0.compute_relabel_edits(start, inc) } {
    Ok(edits) => Either::Right(unsafe {
      Array::new(
        edits
          .into_iter()
          .map(|edit| GvbReplaceText {
            start: edit.range.start,
            end: edit.range.end,
            str: Utf8String::new(edit.str),
          })
          .collect(),
      )
    }),
    Err(gvb::RelabelError::LabelOverflow(label)) => {
      Either::Left(GvbDocRelabelError::LabelOverflow(label))
    }
    Err(gvb::RelabelError::LabelNotFound { range, label }) => {
      Either::Left(GvbDocRelabelError::LabelNotFound {
        start: range.start,
        end: range.end,
        label,
      })
    }
  }
}

#[no_mangle]
pub extern "C" fn gvb_destroy_replace_text_array(edits: Array<GvbReplaceText>) {
  for edit in unsafe { edits.into_boxed_slice() }.iter() {
    destroy_string(edit.str.clone());
  }
}
