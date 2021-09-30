use nanorand::{Rng, WyRand};
use std::num::NonZeroUsize;
use std::time::Duration;

use crate::ast::{Range, SysFuncKind};
use crate::parser::read_number;
use crate::util::mbf5::{Mbf5, RealError};
use crate::HashMap;

pub(crate) use self::codegen::*;
pub(crate) use self::device::*;
pub(crate) use self::instruction::*;
pub(crate) use self::r#type::*;

pub(crate) mod codegen;
pub(crate) mod device;
pub(crate) mod instruction;
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
  rng: WyRand,
  current_rand: u32,
  state: ExecState,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Type {
  Integer,
  Real,
  String,
}

#[derive(Debug, Clone)]
enum ExecState {
  Done,
  Normal,
  WaitForKeyboardInput,
  WaitForKey,
  AsmSuspend,
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
    let mut vm = Self {
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
      rng: WyRand::new(),
      current_rand: 0,
      state: ExecState::Normal,
    };
    vm.current_rand = vm.rng.generate();
    vm
  }

  pub fn reset(&mut self, reset_pc: bool) {
    self.data_ptr = 0;
    if reset_pc {
      self.pc = 0;
    }
    self.code.truncate(self.code_len);
    self.control_stack.clear();
    self.value_stack.clear();
    self.vars.clear();
    self.arrays.clear();
    self.user_funcs.clear();
    self.fn_call_stack.clear();
    //self.device.clear();
    self.device.close_all_files();
    self.rng = WyRand::new();
    self.current_rand = self.rng.generate();
    self.state = ExecState::Normal;
  }

  pub fn exec(
    &mut self,
    input: Option<ExecInput>,
    mut steps: usize,
  ) -> ExecResult {
    match self.state {
      ExecState::Done => return ExecResult::End,
      ExecState::WaitForKey => match input {
        Some(ExecInput::Key(key)) => {
          self.value_stack.push((
            self.code[self.pc].range.clone(),
            Mbf5::from(key as u16).into(),
          ));
          self.pc += 1;
        }
        _ => unreachable!(),
      },
      ExecState::WaitForKeyboardInput => {}
      ExecState::AsmSuspend => {
        if !self.device.exec_asm(&mut steps, None) {
          return self.state.suspend_asm().unwrap_err();
        }
      }
      ExecState::Normal => {
        // do nothing
      }
    }

    while steps > 0 {
      if let Err(result) = self.exec_instr(&mut steps) {
        return result;
      }
    }

    ExecResult::Continue
  }

  fn exec_instr(&mut self, steps: &mut usize) -> Result<()> {
    *steps -= 1;
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
          self.value_stack.push((range, LValue::Var { name }.into()));
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
        let value = self.pop_u8(false)? as usize;
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
        self
          .state
          .error(range, "之前没有执行过 GOSUB 语句，RETURN 语句无法执行")?;
      }
      InstrKind::Pop => {
        while let Some(record) = self.control_stack.pop() {
          if let ControlRecord::Sub { .. } = record {
            self.pc += 1;
            return Ok(());
          }
        }
        self
          .state
          .error(range, "之前没有执行过 GOSUB 语句，POP 语句无法执行")?;
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
      InstrKind::Not => {
        let value = self.value_stack.pop().unwrap().1.unwrap_real();
        self
          .value_stack
          .push((range, Mbf5::from(value.is_zero()).into()));
      }
      InstrKind::Neg => {
        let value = self.value_stack.pop().unwrap().1.unwrap_real();
        self.value_stack.push((range, (-value).into()));
      }
      InstrKind::Eq => {
        let rhs = self.value_stack.pop().unwrap().1.unwrap_real();
        let lhs = self.value_stack.pop().unwrap().1.unwrap_real();
        self
          .value_stack
          .push((range, Mbf5::from(lhs == rhs).into()));
      }
      InstrKind::Ne => {
        let rhs = self.value_stack.pop().unwrap().1.unwrap_real();
        let lhs = self.value_stack.pop().unwrap().1.unwrap_real();
        self
          .value_stack
          .push((range, Mbf5::from(lhs != rhs).into()));
      }
      InstrKind::Gt => {
        let rhs = self.value_stack.pop().unwrap().1.unwrap_real();
        let lhs = self.value_stack.pop().unwrap().1.unwrap_real();
        self.value_stack.push((range, Mbf5::from(lhs > rhs).into()));
      }
      InstrKind::Lt => {
        let rhs = self.value_stack.pop().unwrap().1.unwrap_real();
        let lhs = self.value_stack.pop().unwrap().1.unwrap_real();
        self.value_stack.push((range, Mbf5::from(lhs < rhs).into()));
      }
      InstrKind::Ge => {
        let rhs = self.value_stack.pop().unwrap().1.unwrap_real();
        let lhs = self.value_stack.pop().unwrap().1.unwrap_real();
        self
          .value_stack
          .push((range, Mbf5::from(lhs >= rhs).into()));
      }
      InstrKind::Le => {
        let rhs = self.value_stack.pop().unwrap().1.unwrap_real();
        let lhs = self.value_stack.pop().unwrap().1.unwrap_real();
        self
          .value_stack
          .push((range, Mbf5::from(lhs <= rhs).into()));
      }
      InstrKind::Add => {
        let rhs = self.value_stack.pop().unwrap().1;
        let lhs = self.value_stack.pop().unwrap().1;
        match (lhs, rhs) {
          (TmpValue::Real(lhs), TmpValue::Real(rhs)) => match lhs + rhs {
            Ok(result) => self.value_stack.push((range, result.into())),
            Err(RealError::Infinite) => {
              self.state.error(
              range,
              format!(
                "运算结果数值过大，超出了实数的表示范围。加法运算的两个运算数分别为：{}，{}",
                lhs,
                rhs
              ))?;
            }
            Err(RealError::Nan) => unreachable!(),
          },
          (TmpValue::String(mut lhs), TmpValue::String(mut rhs)) => {
            lhs.append(&mut rhs);
            if lhs.len() > 255 {
              self.state.error(
                range,
                format!(
                  "运算结果字符串过长，长度超出 255。字符串长度为：{}",
                  lhs.len()
                ),
              )?;
            }
            self.value_stack.push((range, lhs.into()));
          }
          _ => unreachable!(),
        }
      }
      InstrKind::Sub => {
        let rhs = self.value_stack.pop().unwrap().1.unwrap_real();
        let lhs = self.value_stack.pop().unwrap().1.unwrap_real();
        match lhs - rhs {
          Ok(result) => self.value_stack.push((range, result.into())),
          Err(RealError::Infinite) => {
            self.state.error(
              range,
              format!(
                "运算结果数值过大，超出了实数的表示范围。减法运算的两个运算数分别为：{}，{}",
                lhs,
                rhs
              ))?;
          }
          Err(RealError::Nan) => unreachable!(),
        }
      }
      InstrKind::Mul => {
        let rhs = self.value_stack.pop().unwrap().1.unwrap_real();
        let lhs = self.value_stack.pop().unwrap().1.unwrap_real();
        match lhs * rhs {
          Ok(result) => self.value_stack.push((range, result.into())),
          Err(RealError::Infinite) => {
            self.state.error(
              range,
              format!(
                "运算结果数值过大，超出了实数的表示范围。乘法运算的两个运算数分别为：{}，{}",
                lhs,
                rhs
              ))?;
          }
          Err(RealError::Nan) => unreachable!(),
        }
      }
      InstrKind::Div => {
        let rhs = self.value_stack.pop().unwrap().1.unwrap_real();
        if rhs.is_zero() {
          self.state.error(range, "除以 0")?;
        }
        let lhs = self.value_stack.pop().unwrap().1.unwrap_real();
        match lhs / rhs {
          Ok(result) => self.value_stack.push((range, result.into())),
          Err(RealError::Infinite) => {
            self.state.error(
              range,
              format!(
                "运算结果数值过大，超出了实数的表示范围。除法运算的两个运算数分别为：{}，{}",
                lhs,
                rhs
              ))?;
          }
          Err(RealError::Nan) => unreachable!(),
        }
      }
      InstrKind::Pow => {
        let rhs = self.value_stack.pop().unwrap().1.unwrap_real();
        let lhs = self.value_stack.pop().unwrap().1.unwrap_real();
        match lhs.pow(rhs) {
          Ok(result) => self.value_stack.push((range, result.into())),
          Err(RealError::Infinite) => {
            self.state.error(
              range,
              format!(
                "运算结果数值过大，超出了实数的表示范围。底数为：{}，指数为：{}",
                lhs,
                rhs
              ))?;
          }
          Err(RealError::Nan) => {
            self.state.error(
              range,
              format!("超出乘方运算的定义域。底数为：{}，指数为：{}", lhs, rhs),
            )?;
          }
        }
      }
      InstrKind::And => {
        let rhs = self.value_stack.pop().unwrap().1.unwrap_real();
        let lhs = self.value_stack.pop().unwrap().1.unwrap_real();
        self
          .value_stack
          .push((range, Mbf5::from(!lhs.is_zero() && !rhs.is_zero()).into()));
      }
      InstrKind::Or => {
        let rhs = self.value_stack.pop().unwrap().1.unwrap_real();
        let lhs = self.value_stack.pop().unwrap().1.unwrap_real();
        self
          .value_stack
          .push((range, Mbf5::from(!lhs.is_zero() || !rhs.is_zero()).into()));
      }
      InstrKind::SysFuncCall { kind, arity } => {
        let value = self.exec_sys_func(range.clone(), kind, arity)?;
        self.value_stack.push((range, value));
      }
      InstrKind::PrintNewLine => {
        self.device.print_newline();
      }
      InstrKind::PrintSpc => {
        let value = self.pop_u8(false)?;
        self.device.print(&vec![b' '; value as usize]);
      }
      InstrKind::PrintTab => {
        let col = self.pop_range(1, 20)? as u8 - 1;
        let current_col = self.device.get_column();
        let spc_num = if current_col > col {
          20 - current_col + col
        } else {
          col - current_col
        };
        self.device.print(&vec![b' '; spc_num as usize]);
      }
      InstrKind::PrintValue => {
        let value = self.value_stack.pop().unwrap().1;
        match value {
          TmpValue::Real(num) => self.device.print(num.to_string().as_bytes()),
          TmpValue::String(s) => self.device.print(&s),
          _ => unreachable!(),
        }
      }
      InstrKind::SetRow => {
        let row = self.pop_range(1, 5)? as u8 - 1;
        self.device.set_row(row);
      }
      InstrKind::SetColumn => {
        let col = self.pop_range(1, 20)? as u8 - 1;
        self.device.set_row(col);
      }
      InstrKind::Beep => {
        todo!()
      }
      InstrKind::DrawBox { has_fill, has_mode } => {
        let mode = self.calc_draw_mode(has_mode)?;
        let fill = if has_fill {
          self.pop_u8(false)? & 1 != 0
        } else {
          false
        };
        let y2 = self.pop_u8(false)?;
        let x2 = self.pop_u8(false)?;
        let y1 = self.pop_u8(false)?;
        let x1 = self.pop_u8(false)?;
        self.device.draw_box(x1, y1, x2, y2, fill, mode);
      }
      InstrKind::Call => {
        let addr = self.pop_range(-65535, 65535)? as u16;
        if !self.device.exec_asm(steps, Some(addr)) {
          self.state.suspend_asm()?;
        }
      }
      InstrKind::DrawCircle { has_fill, has_mode } => {
        let mode = self.calc_draw_mode(has_mode)?;
        let fill = if has_fill {
          self.pop_u8(false)? & 1 != 0
        } else {
          false
        };
        let r = self.pop_u8(false)?;
        let y = self.pop_u8(false)?;
        let x = self.pop_u8(false)?;
        self.device.draw_circle(x, y, r, fill, mode);
      }
      InstrKind::Clear => {
        self.reset(false);
      }
      InstrKind::CloseFile => {
        let filenum = self.pop_filenum()?;
        if !self.device.close_file(filenum) {
          self.state.error(range, "未打开文件，不能关闭文件")?;
        }
      }
      InstrKind::Cls => {
        self.device.cls();
      }
      InstrKind::NoOp => {
        // do nothing
      }
      InstrKind::DrawPoint { has_mode } => {
        let mode = self.calc_draw_mode(has_mode)?;
        let y = self.pop_u8(false)?;
        let x = self.pop_u8(false)?;
        self.device.draw_point(x, y, mode);
      }
      InstrKind::DrawEllipse { has_fill, has_mode } => {
        let mode = self.calc_draw_mode(has_mode)?;
        let fill = if has_fill {
          self.pop_u8(false)? & 1 != 0
        } else {
          false
        };
        let ry = self.pop_u8(false)?;
        let rx = self.pop_u8(false)?;
        let y = self.pop_u8(false)?;
        let x = self.pop_u8(false)?;
        self.device.draw_ellipse(x, y, rx, ry, fill, mode);
      }
      InstrKind::End => {
        self.state.end()?;
      }
      InstrKind::Assign => {
        let rvalue = self.value_stack.pop().unwrap();
        let (_, lvalue) = self.value_stack.pop().unwrap();
        let lvalue = lvalue.unwrap_lvalue();
        self.store_lvalue(lvalue, rvalue)?;
      }
      InstrKind::DrawLine { has_mode } => {
        let mode = self.calc_draw_mode(has_mode)?;
        let y2 = self.pop_u8(false)?;
        let x2 = self.pop_u8(false)?;
        let y1 = self.pop_u8(false)?;
        let x1 = self.pop_u8(false)?;
        self.device.draw_line(x1, y1, x2, y2, mode);
      }
      InstrKind::AlignedAssign(align) => {
        let mut value = self.value_stack.pop().unwrap().1.unwrap_string();
        let lvalue = self.value_stack.pop().unwrap().1.unwrap_lvalue();

        macro_rules! aligned_set {
          ($str:ident => $body:expr) => {
            if value.len() > $str.len() {
              value.truncate($str.len());
              let $str = value;
              $body;
            } else {
              match align {
                Alignment::Left => {
                  $str.clone_from_slice(&value);
                }
                Alignment::Right => {
                  let padding = $str.len() - value.len();
                  $str[padding..].clone_from_slice(&value);
                  $str[..padding].fill(b' ');
                }
              }
              $body;
            }
          };
        }

        match lvalue {
          LValue::Var { name } => match self.get_var_value(name) {
            TmpValue::String(mut str) => {
              aligned_set!(str => self.vars.insert(name, str.into()));
            }
            _ => unreachable!(),
          },
          LValue::Index { name, offset } => {
            match &mut self.arrays.get_mut(&name).unwrap().data {
              ArrayData::String(arr) => {
                let mut str = arr[offset].clone();
                aligned_set!(str => arr[offset] = str);
              }
              _ => unreachable!(),
            }
          }
          _ => unreachable!(),
        }
      }
      InstrKind::SetTrace(_) => todo!(),
      InstrKind::SetScreenMode(mode) => {
        self.device.set_screen_mode(mode);
      }
      InstrKind::PlayNotes => {
        todo!()
      }
      InstrKind::Poke => {
        let value = self.pop_u8(false)?;
        let addr = self.pop_range(-65535, 65535)? as u16;
        self.device.set_byte(addr, value);
      }
      InstrKind::Swap => {
        let lvalue2 = self.value_stack.pop().unwrap().1.unwrap_lvalue();
        let lvalue1 = self.value_stack.pop().unwrap().1.unwrap_lvalue();
      }
      InstrKind::Restart => {
        self.reset(true);
      }
    }
    self.pc += 1;
    Ok(())
  }

  fn exec_sys_func(
    &mut self,
    range: Range,
    kind: SysFuncKind,
    arity: usize,
  ) -> Result<TmpValue> {
    match kind {
      SysFuncKind::Abs => {
        let value = self.value_stack.pop().unwrap().1.unwrap_real();
        Ok(value.abs().into())
      }
      SysFuncKind::Asc => {
        let value = self.value_stack.pop().unwrap().1.unwrap_string();
        if value.is_empty() {
          self.state.error(range, "ASC 函数的参数为空字符串")?;
        }
        Ok(Mbf5::from(value[0]).into())
      }
      SysFuncKind::Atn => {
        let value = self.value_stack.pop().unwrap().1.unwrap_real();
        Ok(value.atan().into())
      }
      SysFuncKind::Chr => {
        let value = self.pop_u8(false)?;
        Ok(ByteString::from(vec![value]).into())
      }
      SysFuncKind::Cos => {
        let value = self.value_stack.pop().unwrap().1.unwrap_real();
        Ok(value.cos().into())
      }
      SysFuncKind::Cvi => {
        let value = self.value_stack.pop().unwrap().1.unwrap_string();
        if value.len() != 2 {
          self.state.error(
            range,
            format!(
              "CVI$ 函数的参数字符串长度不等于 2。参数字符串长度为：{}",
              value.len()
            ),
          )?;
        }
        let lo = value[0] as u16;
        let hi = value[1] as u16;
        Ok(Mbf5::from(lo + (hi << 8)).into())
      }
      SysFuncKind::Cvs => {
        let value = self.value_stack.pop().unwrap().1.unwrap_string();
        if value.len() != 5 {
          self.state.error(
            range,
            format!(
              "CVS$ 函数的参数字符串长度不等于 5。参数字符串长度为：{}",
              value.len()
            ),
          )?;
        }
        Ok(
          Mbf5::from([value[0], value[1], value[2], value[3], value[4]]).into(),
        )
      }
      SysFuncKind::Eof => {
        let filenum = self.pop_filenum()?;
        if let Some(status) = self.device.file_status(filenum) {
          if status.mode != FileMode::Input {
            self.state.error(
              range,
              format!(
                "EOF 函数只能用于以 {} 模式打开的文件，但 {} 号文件是以 {} 模式打开的",
                FileMode::Input,
                filenum + 1,
                status.mode
              ))?;
          }
          Ok(Mbf5::from(status.pos >= status.len).into())
        } else {
          self.state.error(range, "未打开文件")?;
        }
      }
      SysFuncKind::Exp => {
        let value = self.value_stack.pop().unwrap().1.unwrap_real();
        match value.exp() {
          Ok(result) => Ok(result.into()),
          Err(RealError::Infinite) => self.state.error(
            range,
            format!(
              "运算结果数值过大，超出实数的表示范围。参数值是：{}",
              value
            ),
          )?,
          Err(RealError::Nan) => self.state.error(
            range,
            format!("超出 EXP 函数的定义域。参数值是：{}", value),
          )?,
        }
      }
      SysFuncKind::Int => {
        let value = self.value_stack.pop().unwrap().1.unwrap_real();
        Ok(value.truncate().into())
      }
      SysFuncKind::Left => {
        let len = self.pop_u8(true)? as usize;
        let value = self.value_stack.pop().unwrap().1.unwrap_string();
        let len = len.max(value.len());
        Ok(ByteString::from(value[..len].to_vec()).into())
      }
      SysFuncKind::Len => {
        let value = self.value_stack.pop().unwrap().1.unwrap_string();
        Ok(Mbf5::from(value.len() as u32).into())
      }
      SysFuncKind::Lof => {
        let filenum = self.pop_filenum()?;
        if let Some(status) = self.device.file_status(filenum) {
          if status.mode != FileMode::Random {
            self.state.error(
              range,
              format!(
                "LOF 函数只能用于以 {} 模式打开的文件，但 {} 号文件是以 {} 模式打开的",
                FileMode::Random,
                filenum + 1,
                status.mode
              ))?;
          }
          Ok(Mbf5::from(status.len).into())
        } else {
          self.state.error(range, "未打开文件")?;
        }
      }
      SysFuncKind::Log => {
        let value = self.value_stack.pop().unwrap().1.unwrap_real();
        match value.ln() {
          Ok(result) => Ok(result.into()),
          Err(RealError::Infinite) => self.state.error(
            range,
            format!(
              "运算结果数值过大，超出实数的表示范围。参数值是：{}",
              value
            ),
          )?,
          Err(RealError::Nan) => self.state.error(
            range,
            format!("超出 LOG 函数的定义域。参数值是：{}", value),
          )?,
        }
      }
      SysFuncKind::Mid => {
        let len = if arity == 3 {
          self.pop_u8(false)? as usize
        } else {
          255
        };
        let pos = (self.pop_u8(true)? - 1) as usize;
        let value = self.value_stack.pop().unwrap().1.unwrap_string();
        let start = pos.max(value.len());
        let end = (start + len).max(value.len());
        Ok(ByteString::from(value[start..end].to_vec()).into())
      }
      SysFuncKind::Mki => {
        let value = self.pop_range(-32768, 32767)? as i16;
        let lo = (value & 0xff) as u8;
        let hi = (value >> 8) as u8;
        Ok(ByteString::from(vec![lo, hi]).into())
      }
      SysFuncKind::Mks => {
        let value = self.value_stack.pop().unwrap().1.unwrap_real();
        Ok(ByteString::from(<[u8; 5]>::from(value).to_vec()).into())
      }
      SysFuncKind::Peek => {
        let addr = self.pop_range(-65535, 65535)? as u16;
        let value = self.device.get_byte(addr);
        Ok(Mbf5::from(value).into())
      }
      SysFuncKind::Pos => {
        self.value_stack.pop().unwrap();
        Ok(Mbf5::from(self.device.get_column()).into())
      }
      SysFuncKind::Right => {
        let len = self.pop_u8(true)? as usize;
        let value = self.value_stack.pop().unwrap().1.unwrap_string();
        let len = len.max(value.len());
        Ok(ByteString::from(value[value.len() - len..].to_vec()).into())
      }
      SysFuncKind::Rnd => {
        let value = self.value_stack.pop().unwrap().1.unwrap_real();
        if value.is_zero() {
          return Ok(u32_to_random_number(self.current_rand).into());
        }
        if value.is_negative() {
          self.rng.reseed(&<[u8; 5]>::from(value));
        }
        let value: u32 = self.rng.generate();
        self.current_rand = value;
        Ok(u32_to_random_number(value).into())
      }
      SysFuncKind::Sgn => {
        let value = self.value_stack.pop().unwrap().1.unwrap_real();
        if value.is_positive() {
          Ok(Mbf5::one().into())
        } else if value.is_negative() {
          Ok(Mbf5::neg_one().into())
        } else {
          Ok(Mbf5::zero().into())
        }
      }
      SysFuncKind::Sin => {
        let value = self.value_stack.pop().unwrap().1.unwrap_real();
        Ok(value.sin().into())
      }
      SysFuncKind::Sqr => {
        let value = self.value_stack.pop().unwrap().1.unwrap_real();
        match value.sqrt() {
          Ok(result) => Ok(result.into()),
          Err(RealError::Nan) => self.state.error(
            range,
            format!("超出 SQR 函数的定义域。参数值是：{}", value),
          )?,
          Err(RealError::Infinite) => unreachable!(),
        }
      }
      SysFuncKind::Str => {
        let value = self.value_stack.pop().unwrap().1.unwrap_real();
        Ok(ByteString::from(value.to_string().into_bytes()).into())
      }
      SysFuncKind::Tan => {
        let value = self.value_stack.pop().unwrap().1.unwrap_real();
        match value.tan() {
          Ok(result) => Ok(result.into()),
          Err(RealError::Infinite) => self.state.error(
            range,
            format!(
              "运算结果数值过大，超出实数的表示范围。参数值是：{}",
              value
            ),
          )?,
          Err(RealError::Nan) => self.state.error(
            range,
            format!("超出 TAN 函数的定义域。参数值是：{}", value),
          )?,
        }
      }
      SysFuncKind::Val => {
        let mut value = self.value_stack.pop().unwrap().1.unwrap_string();
        value.retain(|&b| b != b' ');
        let (len, _) = read_number(&*value, false);
        Ok(
          unsafe { std::str::from_utf8_unchecked(&value[..len]) }
            .parse::<Mbf5>()
            .unwrap_or(Mbf5::zero())
            .into(),
        )
      }
      SysFuncKind::Tab | SysFuncKind::Spc => unreachable!(),
    }
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

    let value = self.pop_u8(false)? & 7;
    match value {
      0 => Ok(DrawMode::Erase),
      1 | 6 => Ok(DrawMode::Copy),
      2 => Ok(DrawMode::Not),
      _ => Ok(DrawMode::Copy),
    }
  }

  fn pop_u8(&mut self, nonzero: bool) -> Result<u8> {
    Ok(self.pop_range(nonzero as i32, 255)? as u8)
  }

  fn pop_range(&mut self, min: i32, max: i32) -> Result<i32> {
    let (value_range, value) = self.value_stack.pop().unwrap();
    let value = value.unwrap_real();

    let value = f64::from(value);
    if value <= min as f64 - 1.0 || value >= max as f64 + 1.0 {
      self.state.error(
        value_range,
        format!("参数超出范围 {}~{}。运算结果为：{}", min, max, value),
      )?;
    }
    Ok(value as i32)
  }

  /// Returns [0, 2].
  fn pop_filenum(&mut self) -> Result<u8> {
    let (range, value) = self.value_stack.pop().unwrap();
    let value = f64::from(value.unwrap_real()) as i64;
    if value >= 1 && value <= 3 {
      Ok(value as u8 - 1)
    } else {
      self.state.error(range, "文件号超出范围 1~3")?
    }
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

  fn store_value(&mut self, lvalue: LValue, rvalue: Value) {
    match lvalue {
      LValue::Var { name } => {
        self.vars.insert(name, rvalue);
      }
      LValue::Index { name, offset } => {
        match (&mut self.arrays.get_mut(&name).unwrap().data, rvalue) {
          (ArrayData::Integer(arr), Value::Integer(num)) => {
            arr[offset] = num;
          }
          (ArrayData::Real(arr), Value::Real(num)) => {
            arr[offset] = num;
          }
          (ArrayData::String(arr), Value::String(str)) => {
            arr[offset] = str;
          }
          _ => unreachable!(),
        }
      }
      LValue::Fn { .. } => unreachable!(),
    }
  }

  fn store_tmp_value(
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

  #[must_use]
  fn suspend_asm(&mut self) -> Result<!> {
    *self = Self::AsmSuspend;
    Err(ExecResult::Continue)
  }

  #[must_use]
  fn end(&mut self) -> Result<!> {
    *self = Self::Done;
    Err(ExecResult::End)
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

impl TmpValue {
  fn into_value(self, state: &mut ExecState) -> Result<Value> {
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

impl From<ByteString> for Value {
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

fn u32_to_random_number(x: u32) -> Mbf5 {
  if x == 0 {
    return Mbf5::zero();
  }
  let n = x.leading_zeros();
  let exponent = (0x80 - n) as u8;
  let x = x << n;
  let mant1 = (x >> 24) as u8 & 0x7f;
  let mant2 = (x >> 16) as u8;
  let mant3 = (x >> 8) as u8;
  let mant4 = x as u8;
  Mbf5::from([exponent, mant1, mant2, mant3, mant4])
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
