use crate::{
  Array, Diagnostic, Either, Maybe, Severity, Unit, Utf8Str, Utf8String,
};
use gvb_interp as gvb;
use std::convert::TryInto;

pub struct VirtualMachine(
  pub(crate) gvb::VirtualMachine<'static, gvb::device::default::DefaultDevice>,
);

#[repr(C)]
pub enum ExecInput {
  None,
  /// The memory is managed by C++ code.
  KeyboardInput(Array<KeyboardInput>),
  Key(u8),
}

pub type InputFuncBody = gvb::InputFuncBody;

#[repr(C)]
pub enum KeyboardInput {
  /// The memory is managed by C++ code.
  String(Array<u8>),
  Integer(i16),
  Real(Real),
  Func(*mut InputFuncBody),
}

#[repr(C)]
pub struct Real(pub f64);

#[repr(C)]
pub enum KeyboardInputType {
  String,
  Integer,
  Real,
  Func { name: Utf8String, param: Utf8String },
}

#[repr(C)]
pub enum ExecResult {
  End,
  Continue,
  /// nanoseconds
  Sleep(u64),
  KeyboardInput {
    prompt: Maybe<Utf8String>,
    fields: Array<KeyboardInputType>,
  },
  InKey,
  Error {
    location: Location,
    message: Utf8String,
  },
}

#[repr(C)]
pub struct Location {
  pub line: usize,
  pub start_column: usize,
  pub end_column: usize,
}

#[no_mangle]
pub extern "C" fn destroy_vm(vm: *mut VirtualMachine) {
  drop(unsafe { Box::from_raw(vm) });
}

#[no_mangle]
pub extern "C" fn parse_real(input: Utf8Str) -> Maybe<Real> {
  use gvb::util::mbf5;
  match unsafe { input.as_str() }.parse::<mbf5::Mbf5>() {
    Ok(n) => Maybe::Just(Real(n.into())),
    Err(_) => Maybe::Nothing,
  }
}

#[repr(C)]
pub struct CompileFnBodyResult {
  /// may be null
  pub body: *mut InputFuncBody,
  pub diagnostics: Array<Diagnostic<Utf8String>>,
}

#[no_mangle]
pub extern "C" fn compile_fn_body(
  vm: *mut VirtualMachine,
  input: Utf8Str,
) -> CompileFnBodyResult {
  let (body, diags) = unsafe { (*vm).0.compile_fn(input.as_str()) };
  let body = if let Some(body) = body {
    Box::into_raw(box body)
  } else {
    std::ptr::null_mut()
  };
  let diags = diags
    .into_iter()
    .map(|diag| Diagnostic {
      line: 0,
      start: diag.range.start,
      end: diag.range.end,
      message: unsafe { Utf8String::new(diag.message) },
      severity: match diag.severity {
        gvb::Severity::Warning => Severity::Warning,
        gvb::Severity::Error => Severity::Error,
      },
    })
    .collect();
  let diagnostics = unsafe { Array::new(diags) };
  CompileFnBodyResult { body, diagnostics }
}

#[no_mangle]
pub extern "C" fn destroy_fn_body(body: *mut InputFuncBody) {
  drop(unsafe { Box::from_raw(body) })
}

#[no_mangle]
pub extern "C" fn vm_exec(
  vm: *mut VirtualMachine,
  input: ExecInput,
  steps: usize,
) -> ExecResult {
  let input = match input {
    ExecInput::None => gvb::ExecInput::None,
    ExecInput::Key(key) => gvb::ExecInput::Key(key),
    ExecInput::KeyboardInput(input) => {
      let input = unsafe { input.as_slice() }
        .iter()
        .map(|input| match input {
          KeyboardInput::String(s) => gvb::KeyboardInput::String(
            unsafe { s.as_slice() }.to_owned().into(),
          ),
          KeyboardInput::Integer(n) => gvb::KeyboardInput::Integer(*n),
          KeyboardInput::Real(Real(n)) => {
            gvb::KeyboardInput::Real((*n).try_into().unwrap())
          }
          KeyboardInput::Func(body) => gvb::KeyboardInput::Func {
            body: *unsafe { Box::from_raw(*body) },
          },
        })
        .collect();
      gvb::ExecInput::KeyboardInput(input)
    }
  };
  match unsafe { (*vm).0.exec(input, steps) } {
    gvb::ExecResult::End => ExecResult::End,
    gvb::ExecResult::Continue => ExecResult::Continue,
    gvb::ExecResult::Sleep(d) => ExecResult::Sleep(d.as_nanos() as u64),
    gvb::ExecResult::KeyboardInput { prompt, fields } => {
      ExecResult::KeyboardInput {
        prompt: match prompt {
          Some(prompt) => Maybe::Just(unsafe { Utf8String::new(prompt) }),
          None => Maybe::Nothing,
        },
        fields: unsafe {
          Array::new(
            fields
              .into_iter()
              .map(|field| match field {
                gvb::KeyboardInputType::String => KeyboardInputType::String,
                gvb::KeyboardInputType::Integer => KeyboardInputType::Integer,
                gvb::KeyboardInputType::Real => KeyboardInputType::Real,
                gvb::KeyboardInputType::Func { name, param } => {
                  KeyboardInputType::Func {
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
    gvb::ExecResult::InKey => ExecResult::InKey,
    gvb::ExecResult::Error { location, message } => ExecResult::Error {
      location: Location {
        line: location.line,
        start_column: location.range.start,
        end_column: location.range.end,
      },
      message: unsafe { Utf8String::new(message) },
    },
  }
}

#[no_mangle]
pub extern "C" fn vm_stop(
  dev: *mut VirtualMachine,
) -> Either<Utf8String, Unit> {
  match unsafe { (*dev).0.stop() } {
    Ok(()) => Either::Right(Unit::new()),
    Err(gvb::ExecResult::Error {
      location: _,
      message,
    }) => Either::Left(unsafe { Utf8String::new(message) }),
    Err(_) => unreachable!(),
  }
}

#[no_mangle]
pub extern "C" fn vm_reset(dev: *mut VirtualMachine) {
  unsafe {
    (*dev).0.start();
  }
}
