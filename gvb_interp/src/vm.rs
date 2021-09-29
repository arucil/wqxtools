use std::convert::TryFrom;
use std::num::NonZeroUsize;
use std::time::Duration;

use crate::ast::Range;
use crate::util::mbf5::{Mbf5, RealError};
use crate::HashMap;

pub(crate) use self::codegen::*;
pub(crate) use self::device::*;
pub(crate) use self::file::*;
pub(crate) use self::instruction::*;
pub(crate) use self::memory::*;
pub(crate) use self::r#type::*;

pub(crate) mod codegen;
pub(crate) mod device;
pub(crate) mod file;
pub(crate) mod instruction;
pub(crate) mod memory;
pub(crate) mod r#type;

use string_interner::DefaultSymbol as Symbol;
use string_interner::StringInterner;

#[derive(Debug, Clone)]
pub(crate) struct Datum {
  pub range: Range,
  /// Does not include quotes.
  pub value: ByteString,
  pub is_quoted: bool,
}

pub struct VirtualMachine<'d, D> {
  data: Vec<Datum>,
  data_ptr: usize,
  pc: usize,
  code: Vec<Instr>,
  code_len: usize,
  control_stack: Vec<ControlRecord>,
  value_stack: Vec<(Range, TmpValue)>,
  interner: StringInterner,
  vars: HashMap<Symbol, Value>,
  arrays: HashMap<Symbol, Array>,
  user_funcs: HashMap<Symbol, UserFunc>,
  fn_call_stack: Vec<FnCallRecord>,
  device: &'d mut D,
  state: ExecState,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Type {
  Integer,
  Real,
  String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ExecState {
  Done,
  Normal,
  WaitForKeyboardInput,
  WaitForKey,
  AsmSuspend(),
}

#[derive(Debug, Clone)]
enum ControlRecord {
  ForLoop(ForLoopRecord),
  WhileLoop { addr: Addr },
  Sub { next_addr: Addr },
}

#[derive(Debug, Clone)]
struct ForLoopRecord {
  addr: Addr,
  var: Symbol,
  target: Mbf5,
  step: Mbf5,
}

#[derive(Debug, Clone)]
struct FnCallRecord {
  param: Symbol,
  next_addr: Addr,
}

#[derive(Debug, Clone)]
enum TmpValue {
  LValue(LValue),
  String(ByteString),
  Real(Mbf5),
}

#[derive(Debug, Clone)]
enum LValue {
  Index { name: Symbol, offset: usize },
  Var { name: Symbol },
  Fn { name: Symbol, param: Symbol },
}

/// persistent value
#[derive(Debug, Clone)]
pub enum Value {
  Integer(u16),
  Real(Mbf5),
  String(ByteString),
}

#[derive(Debug, Clone)]
struct Array {
  bounds: Vec<NonZeroUsize>,
  data: ArrayData,
}

#[derive(Debug, Clone)]
enum ArrayData {
  Integer(Vec<u16>),
  Real(Vec<Mbf5>),
  String(Vec<ByteString>),
}

#[derive(Debug, Clone)]
struct UserFunc {
  param: Symbol,
  body_addr: Addr,
}

type Result<T> = std::result::Result<T, ExecResult>;

#[derive(Debug, Clone)]
pub enum ExecResult {
  End,
  Continue,
  Sleep(Duration),
  KeyboardInput {
    prompt: Option<String>,
    fields: Vec<KeyboardInputType>,
  },
  InKey,
  Error {
    range: Range,
    message: String,
  },
}

#[derive(Debug, Clone)]
pub enum KeyboardInputType {
  String,
  Real,
  Func { name: String, param: String },
}

#[derive(Debug, Clone)]
pub enum ExecInput {
  KeyboardInput(Vec<KeyboardInput>),
  Key(u8),
}

#[derive(Debug, Clone)]
pub enum KeyboardInput {
  String(ByteString),
  Real(Mbf5),
  Func {
    name: String,
    param: String,
    body: (),
  },
}

impl<'d, D> VirtualMachine<'d, D>
where
  D: Device,
{
  pub fn new(g: CodeGen, device: &'d mut D) -> Self {
    Self {
      data: g.data,
      data_ptr: 0,
      pc: 0,
      code_len: g.code.len(),
      code: g.code,
      control_stack: vec![],
      value_stack: vec![],
      interner: g.interner,
      vars: HashMap::default(),
      arrays: HashMap::default(),
      user_funcs: HashMap::default(),
      fn_call_stack: vec![],
      device,
      state: ExecState::Normal,
    }
  }

  pub fn reset(&mut self) {
    self.data_ptr = 0;
    self.pc = 0;
    self.code.truncate(self.code_len);
    self.control_stack.clear();
    self.value_stack.clear();
    self.vars.clear();
    self.arrays.clear();
    self.user_funcs.clear();
    self.fn_call_stack.clear();
    self.device.clear();
    self.state = ExecState::Normal;
  }

  pub fn exec(&mut self, input: Option<ExecInput>, steps: usize) -> ExecResult {
    match self.state {
      ExecState::Done => return ExecResult::End,
      ExecState::WaitForKey => {
        match input {
          Some(ExecInput::Key(key)) => {
            self.value_stack.push((self.code[self.pc].range.clone(), Mbf5::from(key as u16).into()));
            self.pc += 1;
          }
          _ => unreachable!(),
        }
      }
      ExecState::WaitForKeyboardInput => {}
      ExecState::AsmSuspend() => {
        todo!()
      }
      ExecState::Normal => {
        // do nothing
      }
    }

    for _ in 0..steps {
      if let Err(result) = self.exec_instr() {
        return result;
      }
    }

    ExecResult::Continue
  }

  fn exec_instr(&mut self) -> Result<()> {
    let instr = &self.code[self.pc];
    let range = instr.range.clone();
    match instr.kind.clone() {
      InstrKind::DefFn { name, param, end } => {
        self.user_funcs.insert(
          name,
          UserFunc {
            param,
            body_addr: Addr(self.pc + 1),
          },
        );
        self.pc = end.0;
        return Ok(());
      }
      InstrKind::DimArray { name, dimensions } => {
        if self.arrays.contains_key(&name) {
          self.state.error(range, "重复定义数组")?;
        }
        let mut size = 1;
        let mut bounds = vec![];
        for _ in 0..dimensions.get() {
          let (range, value) = self.value_stack.pop().unwrap();
          let value = value.unwrap_real();
          let bound = f64::from(value.truncate()) as isize;
          if bound < 0 {
            self.state.error(
              range,
              format!("数组下标不能为负数。该下标的值为：{}", f64::from(value)),
            )?
          }
          size *= bound as usize;
          bounds
            .push(unsafe { NonZeroUsize::new_unchecked(bound as usize + 1) });
        }
        let data = ArrayData::new(symbol_type(&self.interner, name), size);
        self.arrays.insert(name, Array { bounds, data });
      }
      InstrKind::PushLValue { name, dimensions } => {
        if dimensions == 0 {
          self
            .value_stack
            .push((range, LValue::Var { name }.into()));
        } else {
          let offset = self.calc_array_offset(name, dimensions)?;
          self
            .value_stack
            .push((range, LValue::Index { name, offset }.into()));
        }
      }
      InstrKind::PushFnLValue { name, param } => {
        self
          .value_stack
          .push((range, LValue::Fn { name, param }.into()));
      }
      InstrKind::SetRecordFields { .. } => todo!(),
      InstrKind::ForLoop { name, has_step } => {
        let step = if has_step {
          self.value_stack.pop().unwrap().1.unwrap_real()
        } else {
          Mbf5::one()
        };
        let end = self.value_stack.pop().unwrap().1.unwrap_real();
        let start = self.value_stack.pop().unwrap();

        let mut prev_loop = None;
        for (i, item) in self.control_stack.iter().enumerate().rev() {
          if let ControlRecord::ForLoop(ForLoopRecord {
            var: prev_var, ..
          }) = item
          {
            if name == *prev_var {
              prev_loop = Some(i);
              break;
            }
          }
        }
        if let Some(i) = prev_loop {
          self.control_stack.truncate(i);
        }

        self
          .control_stack
          .push(ControlRecord::ForLoop(ForLoopRecord {
            addr: Addr(self.pc),
            var: name,
            target: end,
            step,
          }));

        self.store_lvalue(LValue::Var { name }, start)?;
      }
      InstrKind::NextFor { name } => {
        let mut found = None;
        if let Some(name) = name {
          while let Some(record) = self.control_stack.pop() {
            if let ControlRecord::ForLoop(record) = record {
              if record.var == name {
                found = Some(record);
                break;
              }
            }
          }
        } else {
          while let Some(record) = self.control_stack.pop() {
            if let ControlRecord::ForLoop(record) = record {
              found = Some(record);
              break;
            }
          }
        }

        if let Some(record) = found {
          let value = self.get_var_value(record.var).unwrap_real();
          let range = self.code[record.addr.0].range.clone();
          let new_value = match value + record.step {
            Ok(new_value) => new_value,
            Err(RealError::Infinite) => {
              self.state.error(
                range.clone(),
                format!("计数器数值过大，超出了实数的表示范围。"),
              )?;
            }
            Err(_) => unreachable!(),
          };

          self.store_lvalue(
            LValue::Var { name: record.var },
            (range, new_value.into()),
          )?;

          let end_loop = if record.step.is_positive() {
            new_value > record.target
          } else if record.step.is_negative() {
            new_value < record.target
          } else {
            new_value == record.target
          };

          if end_loop {
            self.pc += 1;
          } else {
            self.pc = record.addr.0 + 1;
            self.control_stack.push(ControlRecord::ForLoop(record));
          }
        } else {
          self.state.error(range, "NEXT 语句找不到匹配的 FOR 语句")?;
        }

        return Ok(());
      }
      InstrKind::GoSub(target) => {
        self.control_stack.push(ControlRecord::Sub {
          next_addr: Addr(self.pc + 1),
        });
        self.pc = target.0;
        return Ok(());
      }
      InstrKind::GoTo(target) => {
        self.pc = target.0;
        return Ok(());
      }
      InstrKind::JumpIfZero(target) => {
        let value = self.value_stack.pop().unwrap().1.unwrap_real();
        if value.is_zero() {
          self.pc = target.0;
        } else {
          self.pc += 1;
        }
        return Ok(());
      }
      InstrKind::CallFn(func) => {
        if let Some(func) = self.user_funcs.get(&func).cloned() {
          let arg = self.value_stack.pop().unwrap();
          let old_param = self.get_var_value(func.param);
          self.value_stack.push((arg.0.clone(), old_param));
          self.fn_call_stack.push(FnCallRecord {
            param: func.param,
            next_addr: Addr(self.pc + 1),
          });
          self.pc = func.body_addr.0;
        } else {
          self.state.error(range, "自定义函数不存在")?;
        }
        return Ok(());
      }
      InstrKind::ReturnFn => {
        let stack_len = self.value_stack.len();
        self.value_stack.swap(stack_len - 1, stack_len - 2);
        let old_param = self.value_stack.pop().unwrap();
        let record = self.fn_call_stack.pop().unwrap();
        self.store_lvalue(LValue::Var { name: record.param }, old_param)?;
        self.pc = record.next_addr.0;
        return Ok(());
      }
      InstrKind::Switch(branches) => {
        let value = self.calc_u8()? as usize;
        if value >= 1 && value <= branches.get() {
          match self.code[self.pc + value].kind.clone() {
            InstrKind::GoSub(target) => {
              let next_addr = Addr(self.pc + branches.get() + 1);
              self.control_stack.push(ControlRecord::Sub { next_addr });
              self.pc = target.0;
            }
            InstrKind::GoTo(target) => {
              self.pc = target.0;
            }
            _ => unreachable!(),
          }
        } else {
          self.pc += branches.get() + 1;
        }
        return Ok(());
      }
      InstrKind::RestoreDataPtr(ptr) => {
        self.data_ptr = ptr.0;
      }
      InstrKind::Return => {
        while let Some(record) = self.control_stack.pop() {
          if let ControlRecord::Sub { next_addr } = record {
            self.pc = next_addr.0;
            return Ok(());
          }
        }
        self.state.error(range, "之前没有执行过 GOSUB 语句，RETURN 语句无法执行")?;
      }
      InstrKind::Pop => {
        while let Some(record) = self.control_stack.pop() {
          if let ControlRecord::Sub { .. } = record {
            self.pc += 1;
            return Ok(());
          }
        }
        self.state.error(range, "之前没有执行过 GOSUB 语句，POP 语句无法执行")?;
      }
      InstrKind::PopValue => {
        self.value_stack.pop().unwrap();
      }
      InstrKind::PushNum(num) => {
        self.value_stack.push((range, num.into()));
      }
      InstrKind::PushVar(var) => {
        let value = self.get_var_value(var);
        self.value_stack.push((range, value));
      }
      InstrKind::PushStr(str) => {
        self.value_stack.push((range, str.into()));
      }
      InstrKind::PushInKey => {
        self.state.inkey()?;
      }
      InstrKind::PushIndex { name, dimensions } => {
        let offset = self.calc_array_offset(name, dimensions.get())?;
        let value = match &self.arrays[&name].data {
          ArrayData::Integer(arr) => Mbf5::from(arr[offset]).into(),
          ArrayData::Real(arr) => arr[offset].into(),
          ArrayData::String(arr) => arr[offset].clone().into(),
        };
        self.value_stack.push((range, value));
      }
      InstrKind::Assign => {
        let rvalue = self.value_stack.pop().unwrap();
        let (_, lvalue) = self.value_stack.pop().unwrap();
        let lvalue = lvalue.unwrap_lvalue();
        self.store_lvalue(lvalue, rvalue)?;
      }
      InstrKind::DrawLine { has_mode } => {
        let mode = self.calc_draw_mode(has_mode)?;
        let y2 = self.calc_u8()?;
        let x2 = self.calc_u8()?;
        let y1 = self.calc_u8()?;
        let x1 = self.calc_u8()?;
        self.device.draw_line(x1, y1, x2, y2, mode);
      }
      _ => todo!(),
    }
    self.pc += 1;
    Ok(())
  }

  fn calc_array_offset(
    &mut self,
    name: Symbol,
    dimensions: usize,
  ) -> Result<usize> {
    if !self.arrays.contains_key(&name) {
      let data = ArrayData::new(
        symbol_type(&self.interner, name),
        11usize.pow(dimensions as u32),
      );
      self.arrays.insert(
        name,
        Array {
          bounds: vec![unsafe { NonZeroUsize::new_unchecked(11) }; dimensions],
          data,
        },
      );
    }

    let array = &self.arrays[&name];
    let mut offset = 0;
    for i in (0..dimensions).rev() {
      let (range, value) = self.value_stack.pop().unwrap();
      let value = value.unwrap_real();
      let bound = f64::from(value.truncate()) as isize;
      if bound < 0 {
        self.state.error(
          range,
          format!(
            "数组下标不能为负数。该下标的值为：{}，取整后的值为：{}",
            f64::from(value),
            bound
          ),
        )?
      } else if bound as usize >= array.bounds[i].get() {
        self.state.error(
          range,
          format!(
            "数组下标超出上限。该下标的上限为：{}，该下标的值为：{}, 取整后的值为：{}",
            array.bounds[i].get(),
            f64::from(value),
            bound
          ),
        )?
      }

      offset = offset * array.bounds.get(i + 1).map_or(1, |n| n.get())
        + bound as usize;
    }
    Ok(offset)
  }

  fn calc_draw_mode(&mut self, has_mode: bool) -> Result<DrawMode> {
    if !has_mode {
      return Ok(DrawMode::Copy);
    }

    let value = self.calc_u8()? & 7;
    match value {
      0 => Ok(DrawMode::Erase),
      1 | 6 => Ok(DrawMode::Copy),
      2 => Ok(DrawMode::Not),
      _ => Ok(DrawMode::Copy),
    }
  }

  fn calc_u8(&mut self) -> Result<u32> {
    let (range, value) = self.value_stack.pop().unwrap();
    let value = value.unwrap_real();

    let value = f64::from(value);
    if value <= -1.0 || value >= 256.0 {
      self
        .state
        .error(range, format!("参数超出范围 0~255。运算结果为：{}", value,))?;
    }
    Ok(value as u32)
  }

  fn get_var_value(&mut self, name: Symbol) -> TmpValue {
    let ty = symbol_type(&self.interner, name);
    self
      .vars
      .entry(name)
      .or_insert_with(|| match ty {
        Type::Integer => Value::Integer(0),
        Type::Real => Value::Real(Mbf5::zero()),
        Type::String => Value::String(ByteString::new()),
      })
      .clone()
      .into()
  }

  fn store_lvalue(
    &mut self,
    lvalue: LValue,
    (rvalue_range, rvalue): (Range, TmpValue),
  ) -> Result<()> {
    macro_rules! assign_real {
      ($var:ident => $body:expr) => {
        let $var = rvalue.unwrap_real();
        $body;
      };
    }

    macro_rules! assign_int {
      ($var:ident => $body:expr) => {
        let value = rvalue.unwrap_real();
        let int = f64::from(value.truncate());
        if int <= -32769.0 || int >= 32768.0 {
          self.state.error(
            rvalue_range,
            format!(
              "运算结果数值过大，超出了整数的表示范围（-32768~32767），\
              无法赋值给整数变量。运算结果为：{}",
              f64::from(value),
            ),
          )?;
        }
        let $var = int as u16;
        $body;
      }
    }

    match lvalue {
      LValue::Var { name } => {
        match symbol_type(&self.interner, name) {
          Type::Integer => {
            assign_int!(num => self.vars.insert(name, Value::Integer(num)));
          }
          Type::Real => {
            assign_real!(num => self.vars.insert(name, Value::Real(num)));
          }
          Type::String => {
            self
              .vars
              .insert(name, Value::String(rvalue.unwrap_string()));
          }
        }
        Ok(())
      }
      LValue::Index { name, offset } => {
        match &mut self.arrays.get_mut(&name).unwrap().data {
          ArrayData::Integer(arr) => {
            assign_int!(num => arr[offset] = num);
          }
          ArrayData::Real(arr) => {
            assign_real!(num => arr[offset] = num);
          }
          ArrayData::String(arr) => {
            arr[offset] = rvalue.unwrap_string();
          }
        }
        Ok(())
      }
      LValue::Fn { .. } => unreachable!(),
    }
  }
}

impl ExecState {
  #[must_use]
  fn error<S: ToString>(&mut self, range: Range, message: S) -> Result<!> {
    *self = Self::Done;
    Err(ExecResult::Error {
      range,
      message: message.to_string(),
    })
  }

  #[must_use]
  fn inkey(&mut self) -> Result<!> {
    *self = Self::WaitForKey;
    Err(ExecResult::InKey)
  }
}

impl TmpValue {
  fn unwrap_real(self) -> Mbf5 {
    match self {
      Self::Real(num) => num,
      _ => unreachable!(),
    }
  }

  fn unwrap_string(self) -> ByteString {
    match self {
      Self::String(s) => s,
      _ => unreachable!(),
    }
  }

  fn unwrap_lvalue(self) -> LValue {
    match self {
      Self::LValue(lval) => lval,
      _ => unreachable!(),
    }
  }
}

impl From<Value> for TmpValue {
  fn from(v: Value) -> Self {
    match v {
      Value::Integer(n) => TmpValue::Real(n.into()),
      Value::Real(n) => TmpValue::Real(n.into()),
      Value::String(s) => TmpValue::String(s),
    }
  }
}

impl From<LValue> for TmpValue {
  fn from(v: LValue) -> Self {
    Self::LValue(v)
  }
}

impl From<Mbf5> for TmpValue {
  fn from(v: Mbf5) -> Self {
    Self::Real(v)
  }
}

impl From<ByteString> for TmpValue {
  fn from(v: ByteString) -> Self {
    Self::String(v)
  }
}

fn symbol_type(interner: &StringInterner, symbol: Symbol) -> Type {
  match interner.resolve(symbol).unwrap().as_bytes().last().unwrap() {
    b'%' => Type::Integer,
    b'$' => Type::String,
    _ => Type::Real,
  }
}

impl ArrayData {
  fn new(ty: Type, size: usize) -> Self {
    match ty {
      Type::Integer => ArrayData::Integer(vec![0; size]),
      Type::Real => ArrayData::Real(vec![Mbf5::zero(); size]),
      Type::String => ArrayData::String(vec![ByteString::new(); size]),
    }
  }
}
