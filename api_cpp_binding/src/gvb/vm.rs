use crate::{
  destroy_byte_string, destroy_string, Array, Either, GvbDevice, GvbDiagnostic,
  GvbSeverity, Maybe, Unit, Utf16Str, Utf8String,
};
use gvb_interp as gvb;
use std::convert::TryInto;

pub struct GvbVirtualMachine(
  pub(crate) gvb::VirtualMachine<'static, gvb::device::default::DefaultDevice>,
);

#[repr(C)]
pub enum GvbExecInput {
  None,
  KeyboardInput(Array<GvbKeyboardInput>),
  Key(u8),
}

pub type GvbInputFuncBody = gvb::InputFuncBody;

#[repr(C)]
#[derive(Clone)]
pub enum GvbKeyboardInput {
  Integer(i16),
  Real(GvbReal),
  String(Array<u8>),
  Func(*mut GvbInputFuncBody),
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct GvbReal(pub f64);

#[repr(C)]
pub enum GvbKeyboardInputType {
  String,
  Integer,
  Real,
  Func { name: Utf8String, param: Utf8String },
}

#[repr(C)]
pub enum GvbExecResult {
  End,
  Continue,
  /// nanoseconds
  Sleep(u64),
  KeyboardInput {
    prompt: Maybe<Utf8String>,
    fields: Array<GvbKeyboardInputType>,
  },
  InKey,
  Error {
    location: GvbLocation,
    message: Utf8String,
  },
}

#[repr(C)]
pub struct GvbLocation {
  pub line: usize,
  pub start_column: usize,
  pub end_column: usize,
}

#[no_mangle]
pub extern "C" fn gvb_destroy_vm(vm: *mut GvbVirtualMachine) {
  drop(unsafe { Box::from_raw(vm) });
}

#[repr(C)]
pub struct GvbCompileFnBodyResult {
  /// may be null
  pub body: *mut GvbInputFuncBody,
  pub diagnostics: Array<GvbDiagnostic<Utf8String>>,
}

#[no_mangle]
pub extern "C" fn gvb_compile_fn_body(
  vm: *const GvbVirtualMachine,
  input: Utf16Str,
) -> GvbCompileFnBodyResult {
  let (body, diags) = unsafe {
    (*vm).0.compile_fn(
      String::from_utf16_lossy(std::slice::from_raw_parts(
        input.data, input.len,
      ))
      .as_str(),
    )
  };
  let body = if let Some(body) = body {
    Box::into_raw(box body)
  } else {
    std::ptr::null_mut()
  };
  let diags = diags
    .into_iter()
    .map(|diag| GvbDiagnostic {
      line: 0,
      start: diag.range.start,
      end: diag.range.end,
      message: unsafe { Utf8String::new(diag.message) },
      severity: match diag.severity {
        gvb::Severity::Warning => GvbSeverity::Warning,
        gvb::Severity::Error => GvbSeverity::Error,
      },
    })
    .collect();
  let diagnostics = unsafe { Array::new(diags) };
  GvbCompileFnBodyResult { body, diagnostics }
}

#[no_mangle]
pub extern "C" fn gvb_destroy_fn_body(body: *mut GvbInputFuncBody) {
  if body.is_null() {
    return;
  }
  drop(unsafe { Box::from_raw(body) })
}

#[no_mangle]
pub extern "C" fn gvb_vm_exec(
  vm: *mut GvbVirtualMachine,
  input: GvbExecInput,
  steps: usize,
) -> GvbExecResult {
  let input = match input {
    GvbExecInput::None => gvb::ExecInput::None,
    GvbExecInput::Key(key) => gvb::ExecInput::Key(key),
    GvbExecInput::KeyboardInput(input) => {
      let input = unsafe { input.as_slice() }
        .iter()
        .map(|input| match input {
          GvbKeyboardInput::String(s) => gvb::KeyboardInput::String(
            unsafe { s.as_slice() }.to_owned().into(),
          ),
          GvbKeyboardInput::Integer(n) => gvb::KeyboardInput::Integer(*n),
          GvbKeyboardInput::Real(GvbReal(n)) => {
            gvb::KeyboardInput::Real((*n).try_into().unwrap())
          }
          GvbKeyboardInput::Func(body) => gvb::KeyboardInput::Func {
            body: *unsafe { Box::from_raw(*body) },
          },
        })
        .collect();
      gvb::ExecInput::KeyboardInput(input)
    }
  };
  match unsafe { (*vm).0.exec(input, steps) } {
    gvb::ExecResult::End => GvbExecResult::End,
    gvb::ExecResult::Continue => GvbExecResult::Continue,
    gvb::ExecResult::Sleep(d) => GvbExecResult::Sleep(d.as_nanos() as u64),
    gvb::ExecResult::KeyboardInput { prompt, fields } => {
      GvbExecResult::KeyboardInput {
        prompt: match prompt {
          Some(prompt) => Maybe::Just(unsafe { Utf8String::new(prompt) }),
          None => Maybe::Nothing,
        },
        fields: unsafe {
          Array::new(
            fields
              .into_iter()
              .map(|field| match field {
                gvb::KeyboardInputType::String => GvbKeyboardInputType::String,
                gvb::KeyboardInputType::Integer => {
                  GvbKeyboardInputType::Integer
                }
                gvb::KeyboardInputType::Real => GvbKeyboardInputType::Real,
                gvb::KeyboardInputType::Func { name, param } => {
                  GvbKeyboardInputType::Func {
                    name: Utf8String::new(name),
                    param: Utf8String::new(param),
                  }
                }
              })
              .collect(),
          )
        },
      }
    }
    gvb::ExecResult::InKey => GvbExecResult::InKey,
    gvb::ExecResult::Error { location, message } => GvbExecResult::Error {
      location: GvbLocation {
        line: location.line,
        start_column: location.range.start,
        end_column: location.range.end,
      },
      message: unsafe { Utf8String::new(message) },
    },
  }
}

pub type GvbStopVmResult = Either<Utf8String, Unit>;

#[no_mangle]
pub extern "C" fn gvb_vm_stop(vm: *mut GvbVirtualMachine) -> GvbStopVmResult {
  match unsafe { (*vm).0.stop() } {
    Ok(()) => Either::Right(Unit::new()),
    Err(gvb::ExecResult::Error {
      location: _,
      message,
    }) => Either::Left(unsafe { Utf8String::new(message) }),
    Err(_) => unreachable!(),
  }
}

#[no_mangle]
pub extern "C" fn gvb_vm_reset(vm: *mut GvbVirtualMachine) {
  unsafe {
    (*vm).0.start();
  }
}

#[no_mangle]
pub extern "C" fn gvb_reset_exec_result(result: *mut GvbExecResult) {
  match std::mem::replace(unsafe { &mut *result }, GvbExecResult::Continue) {
    GvbExecResult::End => {}
    GvbExecResult::Continue => {}
    GvbExecResult::Sleep(_) => {}
    GvbExecResult::KeyboardInput { prompt, fields } => {
      if let Maybe::Just(s) = prompt {
        destroy_string(s);
      }
      for field in unsafe { fields.as_slice() } {
        match field {
          GvbKeyboardInputType::Integer => {}
          GvbKeyboardInputType::Real => {}
          GvbKeyboardInputType::String => {}
          GvbKeyboardInputType::Func { name, param } => {
            destroy_string(name.clone());
            destroy_string(param.clone());
          }
        }
      }
      drop(unsafe { fields.into_boxed_slice() });
    }
    GvbExecResult::InKey => {}
    GvbExecResult::Error {
      location: _,
      message,
    } => {
      destroy_string(message);
    }
  }
}

#[no_mangle]
pub extern "C" fn gvb_reset_exec_input(input: *mut GvbExecInput) {
  match std::mem::replace(unsafe { &mut *input }, GvbExecInput::None) {
    GvbExecInput::None => {}
    GvbExecInput::Key(_) => {}
    GvbExecInput::KeyboardInput(input) => {
      for input in unsafe { input.into_boxed_slice() }.iter() {
        match input {
          GvbKeyboardInput::Integer(_) => {}
          GvbKeyboardInput::Real(_) => {}
          GvbKeyboardInput::String(s) => {
            destroy_byte_string((*s).clone());
          }
          GvbKeyboardInput::Func(_func) => {
            // NOTE no need to free `_func`, since it was consumed by VM.
          }
        }
      }
    }
  }
}

#[no_mangle]
pub extern "C" fn gvb_new_input_array(
  data: *const GvbKeyboardInput,
  len: usize,
) -> Array<GvbKeyboardInput> {
  let mut v = vec![GvbKeyboardInput::Integer(0); len];
  for i in 0..len {
    unsafe {
      v[i] = (*data.add(i)).clone();
    }
  }
  unsafe { Array::new(v) }
}

/// Returns if a key was pressed.
#[no_mangle]
pub extern "C" fn gvb_assign_device_key(
  device: *mut GvbDevice,
  input: *mut GvbExecInput,
) -> bool {
  if let Some(key) = unsafe { (*device).0.key() } {
    *unsafe { &mut *input } = GvbExecInput::Key(key);
    true
  } else {
    false
  }
}

#[repr(C)]
pub enum GvbStringError {
  InvalidUtf16,
  InvalidChar(u32),
}

type GvbStringResult = Either<GvbStringError, Array<u8>>;

#[no_mangle]
pub extern "C" fn gvb_utf16_to_byte_string(
  vm: *const GvbVirtualMachine,
  s: Utf16Str,
) -> GvbStringResult {
  let s = unsafe { std::slice::from_raw_parts(s.data as *const _, s.len) };
  let s = match String::from_utf16(s) {
    Ok(s) => s,
    Err(_) => return Either::Left(GvbStringError::InvalidUtf16),
  };
  match unsafe { (*vm).0.byte_string_from_str(&s).into() } {
    Ok(s) => Either::Right(unsafe { Array::new(s.into()) }),
    Err(gvb::vm::r#type::StringError::InvalidChar(c)) => {
      Either::Left(GvbStringError::InvalidChar(c as _))
    }
  }
}
