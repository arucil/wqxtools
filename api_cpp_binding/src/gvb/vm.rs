use crate::{
  destroy_byte_string, destroy_string, Array, ArrayMut, Either, GvbDevice,
  GvbDiagnostic, GvbSeverity, Maybe, Unit, Utf16Str, Utf8Str, Utf8String,
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
  let input = String::from_utf16_lossy(unsafe {
    std::slice::from_raw_parts(input.data, input.len)
  });
  let (body, diags) = unsafe { (*vm).0.compile_fn(input.as_str()) };
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
      for field in unsafe { fields.into_boxed_slice() }.iter() {
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
      gvb_destroy_input_array(input);
    }
  }
}

/// memory of `GvbKeyboardInput`s in `data` is consumed.
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

/// memory of `GvbKeyboardInput`s in `data` is consumed.
#[no_mangle]
pub extern "C" fn gvb_destroy_input_array(input: Array<GvbKeyboardInput>) {
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
  InvalidChar(usize, u32),
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
  let (s, problems) = unsafe { (*vm).0.byte_string_from_str(&s).into() };
  if let Some(p) = problems
    .into_iter()
    .find(|p| matches!(p, gvb::vm::r#type::StringProblem::InvalidChar(..)))
  {
    if let gvb::vm::r#type::StringProblem::InvalidChar(i, c) = p {
      Either::Left(GvbStringError::InvalidChar(i, c as _))
    } else {
      unreachable!()
    }
  } else {
    Either::Right(unsafe { Array::new(s.into()) })
  }
}

/// memory of `s` is not consumed.
#[no_mangle]
pub extern "C" fn gvb_byte_string_to_utf8_lossy(
  vm: *const GvbVirtualMachine,
  s: Array<u8>,
) -> Utf8String {
  let s = unsafe { s.as_slice() }.into();
  unsafe { Utf8String::new((*vm).0.string_from_byte_string_lossy(s)) }
}

#[repr(C)]
#[derive(Clone)]
pub enum GvbBinding {
  Var {
    name: Utf8String,
    value: GvbValue,
  },
  Array {
    name: Utf8String,
    dimensions: Array<u16>,
  },
}

#[repr(C)]
#[derive(Clone)]
pub enum GvbValue {
  Integer(i16),
  Real(GvbReal),
  String(Array<u8>),
}

impl From<gvb::Value> for GvbValue {
  fn from(v: gvb::Value) -> Self {
    match v {
      gvb::Value::Integer(n) => Self::Integer(n),
      gvb::Value::Real(n) => Self::Real(GvbReal(n.into())),
      gvb::Value::String(n) => Self::String(unsafe { Array::new(n.into()) }),
    }
  }
}

impl From<GvbValue> for gvb::Value {
  fn from(v: GvbValue) -> Self {
    match v {
      GvbValue::Integer(n) => Self::Integer(n),
      GvbValue::Real(GvbReal(n)) => Self::Real(n.try_into().unwrap()),
      GvbValue::String(n) => {
        Self::String((&unsafe { n.into_boxed_slice() }[..]).into())
      }
    }
  }
}

#[no_mangle]
pub extern "C" fn gvb_destroy_value(value: GvbValue) {
  match value {
    GvbValue::Integer(_) => {}
    GvbValue::Real(_) => {}
    GvbValue::String(s) => destroy_byte_string(s),
  }
}

#[no_mangle]
pub extern "C" fn gvb_vm_bindings(
  vm: *const GvbVirtualMachine,
) -> ArrayMut<GvbBinding> {
  let bindings = unsafe { (*vm).0.bindings() }
    .into_iter()
    .map(|(name, b)| match b {
      gvb::Binding::Var { value } => GvbBinding::Var {
        name: unsafe { Utf8String::new(name) },
        value: value.into(),
      },
      gvb::Binding::Array { dimensions } => GvbBinding::Array {
        name: unsafe { Utf8String::new(name) },
        dimensions: unsafe { Array::new(dimensions) },
      },
    })
    .collect();
  unsafe { ArrayMut::new(bindings) }
}

#[no_mangle]
pub extern "C" fn gvb_destroy_bindings(bindings: *mut ArrayMut<GvbBinding>) {
  if unsafe { (*bindings).data.is_null() } {
    return;
  }
  for binding in unsafe { (*bindings).clone().into_boxed_slice() }.iter() {
    match binding {
      GvbBinding::Var { name, value } => {
        destroy_string(name.clone());
        gvb_destroy_value(value.clone());
      }
      GvbBinding::Array { name, dimensions } => {
        destroy_string(name.clone());
        drop(unsafe { dimensions.clone().into_boxed_slice() });
      }
    }
  }
  unsafe {
    (*bindings).data = std::ptr::null_mut();
    (*bindings).len = 0;
  }
}

#[repr(C)]
pub enum GvbBindingType {
  Integer,
  Real,
  String,
}

/// memory of `value` is consumed.
#[no_mangle]
pub extern "C" fn gvb_vm_modify_var(
  vm: *mut GvbVirtualMachine,
  name: Utf8Str,
  value: GvbValue,
) {
  let name = unsafe {
    std::str::from_utf8_unchecked(std::slice::from_raw_parts(
      name.data as *const _,
      name.len,
    ))
  };
  unsafe {
    (*vm).0.modify_var(name, value.into());
  }
}

#[repr(C)]
pub enum GvbDimensionValues {
  Integer(ArrayMut<i16>),
  Real(ArrayMut<GvbReal>),
  String(ArrayMut<Array<u8>>),
}

#[no_mangle]
pub extern "C" fn gvb_vm_arr_dim_values(
  vm: *const GvbVirtualMachine,
  name: Utf8Str,
  subs: Array<u16>,
  dim: usize,
) -> GvbDimensionValues {
  let name = unsafe {
    std::str::from_utf8_unchecked(std::slice::from_raw_parts(
      name.data as *const _,
      name.len,
    ))
  };
  let subs = unsafe { std::slice::from_raw_parts(subs.data, subs.len) };
  match unsafe { (*vm).0.arr_dimension_values(name, subs, dim) } {
    gvb::DimensionValues::Integer(vec) => {
      GvbDimensionValues::Integer(unsafe { ArrayMut::new(vec) })
    }
    gvb::DimensionValues::Real(vec) => GvbDimensionValues::Real(unsafe {
      ArrayMut::new(vec.into_iter().map(|n| GvbReal(n.into())).collect())
    }),
    gvb::DimensionValues::String(vec) => GvbDimensionValues::String(unsafe {
      ArrayMut::new(vec.into_iter().map(|n| Array::new(n.into())).collect())
    }),
  }
}

#[no_mangle]
pub extern "C" fn gvb_destroy_real_array_mut(arr: ArrayMut<GvbReal>) {
  if arr.data.is_null() {
    return;
  }
  drop(unsafe { arr.into_boxed_slice() });
}

/// memory of `subs` is managed by C++ code.
///
/// memory of `value` is consumed.
#[no_mangle]
pub extern "C" fn gvb_vm_modify_arr(
  vm: *mut GvbVirtualMachine,
  name: Utf8Str,
  subs: Array<u16>,
  value: GvbValue,
) {
  let name = unsafe {
    std::str::from_utf8_unchecked(std::slice::from_raw_parts(
      name.data as *const _,
      name.len,
    ))
  };
  let subs = unsafe { std::slice::from_raw_parts(subs.data, subs.len) };
  unsafe {
    (*vm).0.modify_arr(name, subs, value.into());
  }
}
