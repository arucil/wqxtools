use std::convert::TryFrom;
use std::num::NonZeroUsize;

use crate::ast::Range;
use crate::util::mbf5::FloatError;
use crate::util::mbf5::Mbf5;
use crate::util::mbf5::Mbf5Accum;
use crate::HashMap;

pub(crate) use self::codegen::*;
pub(crate) use self::file::*;
pub(crate) use self::instruction::*;
pub(crate) use self::memory::*;
pub(crate) use self::r#type::*;

pub(crate) mod codegen;
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

pub struct VirtualMachine {
  data: Vec<Datum>,
  data_ptr: usize,
  pc: usize,
  code: Vec<Instr>,
  screen_mode: ScreenMode,
  print_mode: PrintMode,
  control_stack: Vec<ControlStackItem>,
  value_stack: Vec<(Range, TmpValue)>,
  interner: StringInterner,
  vars: HashMap<Symbol, Value>,
  arrays: HashMap<Symbol, Array>,
  user_funcs: HashMap<Symbol, UserFunc>,
  memory_man: MemoryManager,
  file_man: FileManager,
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
  AsmSuspend(),
}

#[derive(Debug, Clone)]
enum ControlStackItem {
  ForLoop {
    addr: Addr,
    var: Symbol,
    target: Mbf5Accum,
    step: Mbf5Accum,
  },
  WhileLoop {
    addr: Addr,
  },
  Sub {
    next_addr: Addr,
  },
}

#[derive(Debug, Clone)]
enum TmpValue {
  LValue(LValue),
  String(ByteString),
  Real(Mbf5Accum),
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

struct UserFunc {
  param: Symbol,
  body_addr: Addr,
}

#[derive(Debug, Clone)]
pub enum ExecResult {
  End,
  Continue,
  Sleep(usize),
  KeyboardInput {
    prompt: Option<String>,
    fields: Vec<KeyboardInputType>,
  },
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

impl VirtualMachine {
  pub fn new(
    g: CodeGen,
    memory_man: MemoryManager,
    file_man: FileManager,
  ) -> Self {
    Self {
      data: g.data,
      data_ptr: 0,
      pc: 0,
      code: g.code,
      screen_mode: ScreenMode::Text,
      print_mode: PrintMode::Normal,
      control_stack: vec![],
      value_stack: vec![],
      interner: g.interner,
      vars: HashMap::default(),
      arrays: HashMap::default(),
      user_funcs: HashMap::default(),
      memory_man,
      file_man,
      state: ExecState::Normal,
    }
  }

  pub fn exec(&mut self, input: Option<ExecInput>, steps: usize) -> ExecResult {
    match self.state {
      ExecState::Done => return ExecResult::End,
      ExecState::WaitForKeyboardInput => {
        todo!()
      }
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

  fn exec_instr(&mut self) -> Result<(), ExecResult> {
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
            .push((range, TmpValue::LValue(LValue::Var { name })));
        } else {
          let offset = self.calc_array_offset(name, dimensions)?;
          self
            .value_stack
            .push((range, TmpValue::LValue(LValue::Index { name, offset })));
        }
      }
      InstrKind::PushFnLValue { name, param } => {
        self
          .value_stack
          .push((range, TmpValue::LValue(LValue::Fn { name, param })));
      }
      InstrKind::SetRecordFields { .. } => todo!(),
      InstrKind::ForLoop { name, has_step } => {
        let step = if has_step {
          self.value_stack.pop().unwrap().1.unwrap_real()
        } else {
          Mbf5Accum::one()
        };
        let end = self.value_stack.pop().unwrap().1.unwrap_real();
        let start = self.value_stack.pop().unwrap().1.unwrap_real();

        let mut prev_loop = None;
        for (i, item) in self.control_stack.iter().enumerate().rev() {
          if let ControlStackItem::ForLoop { var: prev_var, .. } = item {
            if name == *prev_var {
              prev_loop = Some(i);
              break;
            }
          }
        }
        if let Some(i) = prev_loop {
          self.control_stack.truncate(i);
        }

        self.control_stack.push(ControlStackItem::ForLoop {
          addr: Addr(self.pc),
          var: name,
          target: end,
          step,
        });

        //self.store_lvalue(LValue::Var { name }, LValue::);
      }
      InstrKind::Assign => {
        let (rvalue_range, rvalue) = self.value_stack.pop().unwrap();
        let (lvalue_range, lvalue) = self.value_stack.pop().unwrap();
        let lvalue = lvalue.unwrap_lvalue();
        let rvalue = Value::from(rvalue);
        self.store_lvalue(lvalue, rvalue)?;
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
  ) -> Result<usize, ExecResult> {
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

  fn store_lvalue(
    &mut self,
    lvalue: LValue,
    (rvalue_range, rvalue): (Range, TmpValue),
  ) -> Result<(), ExecResult> {
    macro_rules! assign_real {
      ($var:ident => $body:expr) => {
        let value = rvalue.unwrap_real();
        match Mbf5::try_from(value) {
          Ok($var) => {
            $body
          }
          Err(FloatError::Infinite) => {
            self.state.error(
              rvalue_range,
              format!(
                "运算结果数值过大，超出了实数的表示范围。运算结果为：{}",
                f64::from(value),
              ),
            )?;
          }
          Err(_) => unreachable!(),
        }
      }
    }
    match lvalue {
      LValue::Var { name } => {
        match symbol_type(&self.interner, name) {
          Type::Integer => {
            let value = rvalue.unwrap_real();
            if let Err(FloatError::Infinite) = Mbf5::try_from(value) {
              self.state.error(
                rvalue_range,
                format!(
                  "运算结果数值过大，超出了实数的表示范围。运算结果为：{}",
                  f64::from(value),
                ),
              )?;
            }
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
            self.vars.insert(name, Value::Integer(int as u16));
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
          }
          (ArrayData::Real(arr), Value::Real(n)) => arr[offset] = n,
          (ArrayData::String(arr), Value::String(n)) => arr[offset] = n,
          _ => unreachable!(),
        }
        Ok(())
      }
      LValue::Fn { .. } => unreachable!(),
    }
  }
}

impl ExecState {
  fn error<S: ToString>(
    &mut self,
    range: Range,
    message: S,
  ) -> Result<!, ExecResult> {
    *self = Self::Done;
    Err(ExecResult::Error {
      range,
      message: message.to_string(),
    })
  }
}

impl TmpValue {
  fn unwrap_real(self) -> Mbf5Accum {
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
