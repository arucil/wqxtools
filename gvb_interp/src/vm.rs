use bstr::{ByteSlice, ByteVec};
use nanorand::{Rng, WyRand};
use std::fmt::{self, Display, Formatter};
use std::io;
use std::num::NonZeroUsize;
use std::time::Duration;

use crate::ast::{self, SysFuncKind};
use crate::machine::EmojiStyle;
use crate::parser::read_number;
use crate::util::mbf5::{Mbf5, ParseRealError, RealError};
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
  /// Does not include quotes.
  pub value: ByteString,
  pub is_quoted: bool,
}

const NUM_FILES: usize = 3;

pub struct VirtualMachine<'d, D: Device> {
  emoji_style: EmojiStyle,
  data: Vec<Datum>,
  data_ptr: usize,
  pc: usize,
  code: Vec<Instr>,
  code_len: usize,
  control_stack: Vec<ControlRecord>,
  value_stack: Vec<(Location, TmpValue)>,
  interner: StringInterner,
  store: Store,
  fn_call_stack: Vec<FnCallRecord>,
  device: &'d mut D,
  files: [Option<OpenFile<D::File>>; NUM_FILES],
  rng: WyRand,
  current_rand: u32,
  state: ExecState,
}

#[derive(Default)]
struct Store {
  vars: HashMap<Symbol, Value>,
  arrays: HashMap<Symbol, Array>,
  user_funcs: HashMap<Symbol, UserFunc>,
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
  WaitForKeyboardInput { lvalues: Vec<(Location, LValue)> },
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
  Integer(i16),
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
  Integer(Vec<i16>),
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
    location: Location,
    message: String,
  },
}

#[derive(Debug, Clone)]
pub enum KeyboardInputType {
  String,
  Integer,
  Real,
  Func { name: String, param: String },
}

pub enum ExecInput {
  KeyboardInput(Vec<KeyboardInput>),
  Key(u8),
}

pub enum KeyboardInput {
  String(ByteString),
  Integer(i16),
  Real(Mbf5),
  Func { body: InputFuncBody },
}

pub struct InputFuncBody {
  interner: StringInterner,
  code: Vec<Instr>,
}

#[derive(Debug, Clone)]
struct OpenFile<F> {
  pub file: F,
  pub mode: FileMode,
}

#[derive(Debug, Clone)]
enum FileMode {
  Input,
  Output,
  Append,
  Random {
    record_len: u8,
    fields: Vec<RecordField>,
  },
}

#[derive(Debug, Clone)]
struct RecordField {
  len: u8,
  lvalue: LValue,
}

impl<'d, D> VirtualMachine<'d, D>
where
  D: Device,
{
  pub fn new(g: CodeGen, device: &'d mut D) -> Self {
    let mut vm = Self {
      emoji_style: g.emoji_style,
      data: g.data,
      data_ptr: 0,
      pc: 0,
      code_len: g.code.len(),
      code: g.code,
      control_stack: vec![],
      value_stack: vec![],
      interner: g.interner,
      store: Store::default(),
      fn_call_stack: vec![],
      device,
      files: [None, None, None],
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
    self.store.clear();
    self.fn_call_stack.clear();
    //self.device.clear();
    for file in &mut self.files {
      file.take();
    }
    self.rng = WyRand::new();
    self.current_rand = self.rng.generate();
    self.state = ExecState::Normal;
  }

  pub fn exec(
    &mut self,
    input: Option<ExecInput>,
    mut steps: usize,
  ) -> ExecResult {
    match std::mem::replace(&mut self.state, ExecState::Normal) {
      ExecState::Done => return ExecResult::End,
      ExecState::WaitForKey => self.assign_key(input.unwrap()),
      ExecState::WaitForKeyboardInput { lvalues } => {
        self.assign_input(input.unwrap(), lvalues)
      }
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
    let loc = instr.loc.clone();

    macro_rules! bin_op {
      ($lhs:ident, $rhs:ident, $body:expr) => {{
        let $rhs = self.value_stack.pop().unwrap().1;
        match $rhs {
          TmpValue::Real($rhs) => {
            let $lhs = self.value_stack.pop().unwrap().1.unwrap_real();
            self.value_stack.push((loc, Mbf5::from($body).into()));
          }
          TmpValue::String($rhs) => {
            let $lhs = self.value_stack.pop().unwrap().1.unwrap_string();
            self.value_stack.push((loc, Mbf5::from($body).into()));
          }
          _ => unreachable!(),
        }
      }};
    }

    match instr.kind.clone() {
      InstrKind::DefFn { name, param, end } => {
        self.store.user_funcs.insert(
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
        if self.store.arrays.contains_key(&name) {
          self.state.error(loc, "重复定义数组")?;
        }
        let mut size = 1;
        let mut bounds = vec![];
        for _ in 0..dimensions.get() {
          let (loc, value) = self.value_stack.pop().unwrap();
          let value = value.unwrap_real();
          let bound = f64::from(value.truncate()) as isize;
          if bound < 0 {
            self.state.error(
              loc,
              format!("数组下标不能为负数。该下标的值为：{}", f64::from(value)),
            )?
          }
          size *= bound as usize;
          bounds
            .push(unsafe { NonZeroUsize::new_unchecked(bound as usize + 1) });
        }
        let data = ArrayData::new(symbol_type(&self.interner, name), size);
        self.store.arrays.insert(name, Array { bounds, data });
      }
      InstrKind::PushLValue { name, dimensions } => {
        if dimensions == 0 {
          self.value_stack.push((loc, LValue::Var { name }.into()));
        } else {
          let offset = self.calc_array_offset(name, dimensions)?;
          self
            .value_stack
            .push((loc, LValue::Index { name, offset }.into()));
        }
      }
      InstrKind::PushFnLValue { name, param } => {
        self
          .value_stack
          .push((loc, LValue::Fn { name, param }.into()));
      }
      InstrKind::SetRecordFields { fields } => {
        self.exec_field(loc, fields.get())?
      }
      InstrKind::ForLoop { name, has_step } => {
        self.exec_for(loc, name, has_step)?
      }
      InstrKind::NextFor { name } => {
        return self.exec_next(loc, name);
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
        if let Some(func) = self.store.user_funcs.get(&func).cloned() {
          let arg = self.value_stack.pop().unwrap();
          let old_param = self.load_tmp_value(func.param);
          self.value_stack.push((arg.0.clone(), old_param));
          self.fn_call_stack.push(FnCallRecord {
            param: func.param,
            next_addr: Addr(self.pc + 1),
          });
          self.pc = func.body_addr.0;
        } else {
          self.state.error(loc, "自定义函数不存在")?;
        }
        return Ok(());
      }
      InstrKind::ReturnFn => {
        let stack_len = self.value_stack.len();
        self.value_stack.swap(stack_len - 1, stack_len - 2);
        let old_param = self.value_stack.pop().unwrap();
        let record = self.fn_call_stack.pop().unwrap();
        self.store_tmp_value(LValue::Var { name: record.param }, old_param)?;
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
          .error(loc, "之前没有执行过 GOSUB 语句，RETURN 语句无法执行")?;
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
          .error(loc, "之前没有执行过 GOSUB 语句，POP 语句无法执行")?;
      }
      InstrKind::PopValue => {
        self.value_stack.pop().unwrap();
      }
      InstrKind::PushNum(num) => {
        self.value_stack.push((loc, num.into()));
      }
      InstrKind::PushVar(var) => {
        let value = self.load_tmp_value(var);
        self.value_stack.push((loc, value));
      }
      InstrKind::PushStr(str) => {
        self.value_stack.push((loc, str.into()));
      }
      InstrKind::PushInKey => {
        self.state.inkey()?;
      }
      InstrKind::PushIndex { name, dimensions } => {
        let offset = self.calc_array_offset(name, dimensions.get())?;
        let value = match &self.store.arrays[&name].data {
          ArrayData::Integer(arr) => Mbf5::from(arr[offset]).into(),
          ArrayData::Real(arr) => arr[offset].into(),
          ArrayData::String(arr) => arr[offset].clone().into(),
        };
        self.value_stack.push((loc, value));
      }
      InstrKind::Not => {
        let value = self.value_stack.pop().unwrap().1.unwrap_real();
        self
          .value_stack
          .push((loc, Mbf5::from(value.is_zero()).into()));
      }
      InstrKind::Neg => {
        let value = self.value_stack.pop().unwrap().1.unwrap_real();
        self.value_stack.push((loc, (-value).into()));
      }
      InstrKind::Eq => bin_op!(lhs, rhs, lhs == rhs),
      InstrKind::Ne => bin_op!(lhs, rhs, lhs != rhs),
      InstrKind::Gt => bin_op!(lhs, rhs, lhs > rhs),
      InstrKind::Lt => bin_op!(lhs, rhs, lhs < rhs),
      InstrKind::Ge => bin_op!(lhs, rhs, lhs >= rhs),
      InstrKind::Le => bin_op!(lhs, rhs, lhs <= rhs),
      InstrKind::Add => {
        let rhs = self.value_stack.pop().unwrap().1;
        let lhs = self.value_stack.pop().unwrap().1;
        match (lhs, rhs) {
          (TmpValue::Real(lhs), TmpValue::Real(rhs)) => match lhs + rhs {
            Ok(result) => self.value_stack.push((loc, result.into())),
            Err(RealError::Infinite) => {
              self.state.error(
              loc,
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
                loc,
                format!(
                  "运算结果字符串过长，长度超出 255。字符串长度为：{}",
                  lhs.len()
                ),
              )?;
            }
            self.value_stack.push((loc, lhs.into()));
          }
          _ => unreachable!(),
        }
      }
      InstrKind::Sub => {
        let rhs = self.value_stack.pop().unwrap().1.unwrap_real();
        let lhs = self.value_stack.pop().unwrap().1.unwrap_real();
        match lhs - rhs {
          Ok(result) => self.value_stack.push((loc, result.into())),
          Err(RealError::Infinite) => {
            self.state.error(
              loc,
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
          Ok(result) => self.value_stack.push((loc, result.into())),
          Err(RealError::Infinite) => {
            self.state.error(
              loc,
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
          self.state.error(loc, "除以 0")?;
        }
        let lhs = self.value_stack.pop().unwrap().1.unwrap_real();
        match lhs / rhs {
          Ok(result) => self.value_stack.push((loc, result.into())),
          Err(RealError::Infinite) => {
            self.state.error(
              loc,
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
          Ok(result) => self.value_stack.push((loc, result.into())),
          Err(RealError::Infinite) => {
            self.state.error(
              loc,
              format!(
                "运算结果数值过大，超出了实数的表示范围。底数为：{}，指数为：{}",
                lhs,
                rhs
              ))?;
          }
          Err(RealError::Nan) => {
            self.state.error(
              loc,
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
          .push((loc, Mbf5::from(!lhs.is_zero() && !rhs.is_zero()).into()));
      }
      InstrKind::Or => {
        let rhs = self.value_stack.pop().unwrap().1.unwrap_real();
        let lhs = self.value_stack.pop().unwrap().1.unwrap_real();
        self
          .value_stack
          .push((loc, Mbf5::from(!lhs.is_zero() || !rhs.is_zero()).into()));
      }
      InstrKind::SysFuncCall { kind, arity } => {
        let value = self.exec_sys_func(loc.clone(), kind, arity)?;
        self.value_stack.push((loc, value));
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
          TmpValue::String(s) => self.device.print(&s.to_print_form()),
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
      InstrKind::Write { to_file } => {
        self.exec_write(loc, to_file, false)?;
      }
      InstrKind::WriteEnd { to_file } => {
        self.exec_write(loc, to_file, true)?;
      }
      InstrKind::KeyboardInput {
        prompt,
        fields: num_fields,
      } => {
        let mut lvalues = vec![];
        let mut fields = vec![];
        for _ in 0..num_fields.get() {
          let (lval_loc, lvalue) = self.value_stack.pop().unwrap();
          let lvalue = lvalue.unwrap_lvalue();
          match lvalue {
            LValue::Fn { name, param } => {
              fields.push(KeyboardInputType::Func {
                name: self.interner.resolve(name).unwrap().to_owned(),
                param: self.interner.resolve(param).unwrap().to_owned(),
              })
            }
            _ => match lvalue.get_type(&self.interner) {
              Type::Integer => fields.push(KeyboardInputType::Integer),
              Type::Real => fields.push(KeyboardInputType::Real),
              Type::String => fields.push(KeyboardInputType::String),
            },
          }
          lvalues.push((lval_loc, lvalue));
        }

        fields.reverse();
        self.state.input(lvalues, prompt, fields)?;
      }
      InstrKind::FileInput { fields: num_fields } => {
        let filenum = self.get_filenum(true)?;
        let file = if let Some(file) = &mut self.files[filenum as usize] {
          if let FileMode::Input = file.mode {
            &mut file.file
          } else {
            self.state.error(
              loc,
              format!(
                "INPUT 语句只能用于以 INPUT 模式打开的文件，但 {} 号文件是以 {} 模式打开的",
                filenum + 1,
                file.mode
              ))?;
          }
        } else {
          self.state.error(loc, "未打开文件")?;
        };

        let mut fields = vec![];
        for _ in 0..num_fields.get() {
          let (lval_loc, lvalue) = self.value_stack.pop().unwrap();
          let lvalue = lvalue.unwrap_lvalue();
          fields.push((lval_loc, lvalue));
        }
        for (lval_loc, lvalue) in fields.into_iter().rev() {
          exec_file_input(
            &mut self.state,
            &mut self.store,
            &self.interner,
            self.emoji_style,
            lval_loc,
            lvalue,
            file,
          )?;
        }
      }
      InstrKind::ReadData => self.exec_read(loc)?,
      InstrKind::OpenFile { mode, has_len } => {
        self.exec_open(loc, mode, has_len)?
      }
      InstrKind::Beep => {
        self.device.beep();
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
        let filenum = self.get_filenum(true)?;
        if self.files[filenum as usize].take().is_none() {
          self.state.error(loc, "未打开文件，不能关闭文件")?;
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
      InstrKind::ReadRecord => {
        self.exec_get_put(loc.clone(), |this, filenum| {
          let file = this.files[filenum].as_mut().unwrap();
          let (record_len, fields) = match &file.mode {
            FileMode::Random { record_len, fields } => (*record_len, fields),
            _ => unreachable!(),
          };
          let file = &mut file.file;

          let mut buf = vec![0; record_len as usize];
          let read_len =
            this
              .state
              .io(loc.clone(), "读取文件", file.read(&mut buf))?;
          if read_len == 0 {
            this.state.error(loc, "不能在文件末尾读取记录")?;
          }
          if read_len < record_len as usize {
            this.state.error(loc, "文件大小不是记录长度的整数倍")?;
          }

          let mut offset = 0;
          for field in fields {
            this.store.store_value(
              field.lvalue.clone(),
              Value::String(
                buf[offset..offset + field.len as usize].to_owned().into(),
              ),
            );
            offset += field.len as usize;
          }

          Ok(())
        })?;
      }
      InstrKind::WriteRecord => {
        self.exec_get_put(loc.clone(), |this, filenum| {
          let file = this.files[filenum].as_mut().unwrap();
          let (record_len, fields) = match &file.mode {
            FileMode::Random { record_len, fields } => (*record_len, fields),
            _ => unreachable!(),
          };
          let file = &mut file.file;

          let mut buf = vec![0u8; record_len as usize];
          let mut offset = 0;
          for field in fields {
            let str = this
              .store
              .load_value(&this.interner, field.lvalue.clone())
              .unwrap_string();
            if str.len() == field.len as usize {
              buf[offset..offset + field.len as usize].clone_from_slice(&str);
            }
            offset += field.len as usize;
          }

          this.state.io(loc.clone(), "写入文件", file.write(&buf))?;

          Ok(())
        })?;
      }
      InstrKind::Assign => {
        let rvalue = self.value_stack.pop().unwrap();
        let (_, lvalue) = self.value_stack.pop().unwrap();
        let lvalue = lvalue.unwrap_lvalue();
        self.store_tmp_value(lvalue, rvalue)?;
      }
      InstrKind::DrawLine { has_mode } => {
        let mode = self.calc_draw_mode(has_mode)?;
        let y2 = self.pop_u8(false)?;
        let x2 = self.pop_u8(false)?;
        let y1 = self.pop_u8(false)?;
        let x1 = self.pop_u8(false)?;
        self.device.draw_line(x1, y1, x2, y2, mode);
      }
      InstrKind::AlignedAssign(align) => self.exec_set(loc, align)?,
      InstrKind::SetTrace(_) => todo!(),
      InstrKind::SetScreenMode(mode) => {
        self.device.set_screen_mode(mode);
      }
      InstrKind::PlayNotes => {
        let value = self.value_stack.pop().unwrap().1.unwrap_string();
        self.device.play_notes(&value);
      }
      InstrKind::Poke => {
        let value = self.pop_u8(false)?;
        let addr = self.pop_range(-65535, 65535)? as u16;
        self.device.set_byte(addr, value);
      }
      InstrKind::Swap => {
        let lvalue2 = self.value_stack.pop().unwrap().1.unwrap_lvalue();
        let lvalue1 = self.value_stack.pop().unwrap().1.unwrap_lvalue();
        let value1 = self.store.load_value(&self.interner, lvalue1.clone());
        let value2 = self.store.load_value(&self.interner, lvalue2.clone());
        self.store.store_value(lvalue2, value1);
        self.store.store_value(lvalue1, value2);
      }
      InstrKind::Restart => {
        self.device.set_screen_mode(ScreenMode::Text);
        self.device.cls();
        self.reset(true);
      }
      InstrKind::SetPrintMode(mode) => {
        self.device.set_print_mode(mode);
      }
      InstrKind::Wend => {
        let mut found = None;
        while let Some(record) = self.control_stack.pop() {
          if let ControlRecord::WhileLoop { addr } = record {
            found = Some(addr);
            break;
          }
        }

        if let Some(addr) = found {
          self.pc = addr.0;
        } else {
          self.state.error(loc, "WEND 语句找不到匹配的 WHILE 语句")?;
        }

        return Ok(());
      }
      InstrKind::WhileLoop { start, end } => {
        let value = self.value_stack.pop().unwrap().1.unwrap_real();
        if value.is_zero() {
          self.pc = end.0;
        } else {
          self
            .control_stack
            .push(ControlRecord::WhileLoop { addr: start });
          self.pc += 1;
        }

        return Ok(());
      }
      InstrKind::Sleep => {
        let value = self.value_stack.pop().unwrap().1.unwrap_real();
        if value.is_positive() {
          let ns = (self.device.sleep_unit().as_nanos() as f64
            * f64::from(value)) as u64;
          self.state.sleep(Duration::from_nanos(ns))?;
        }
      }
    }
    self.pc += 1;
    Ok(())
  }

  fn exec_sys_func(
    &mut self,
    loc: Location,
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
          self.state.error(loc, "ASC 函数的参数为空字符串")?;
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
            loc,
            format!(
              "CVI$ 函数的参数字符串长度不等于 2。参数字符串长度为：{}",
              value.len()
            ),
          )?;
        }
        let lo = value[0] as u16;
        let hi = value[1] as u16;
        Ok(Mbf5::from((lo + (hi << 8)) as i16).into())
      }
      SysFuncKind::Cvs => {
        let value = self.value_stack.pop().unwrap().1.unwrap_string();
        if value.len() != 5 {
          self.state.error(
            loc,
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
        let filenum = self.get_filenum(true)?;
        if let Some(file) = &self.files[filenum as usize] {
          if !matches!(file.mode, FileMode::Input) {
            self.state.error(
              loc,
              format!(
                "EOF 函数只能用于以 INPUT 模式打开的文件，但 {} 号文件是以 {} 模式打开的",
                filenum + 1,
                file.mode
              ))?;
          }
          let len =
            self
              .state
              .io(loc.clone(), "获取文件大小", file.file.len())?;
          let pos = self.state.io(loc, "获取文件指针", file.file.pos())?;
          Ok(Mbf5::from(pos >= len).into())
        } else {
          self.state.error(loc, "未打开文件")?;
        }
      }
      SysFuncKind::Exp => {
        let value = self.value_stack.pop().unwrap().1.unwrap_real();
        match value.exp() {
          Ok(result) => Ok(result.into()),
          Err(RealError::Infinite) => self.state.error(
            loc,
            format!(
              "运算结果数值过大，超出实数的表示范围。参数值是：{}",
              value
            ),
          )?,
          Err(RealError::Nan) => self.state.error(
            loc,
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
        let filenum = self.get_filenum(true)?;
        if let Some(file) = &self.files[filenum as usize] {
          if !matches!(file.mode, FileMode::Random { .. }) {
            self.state.error(
              loc,
              format!(
                "LOF 函数只能用于以 RANDOM 模式打开的文件，但 {} 号文件是以 {} 模式打开的",
                filenum + 1,
                file.mode
              ))?;
          }
          let len = self.state.io(loc, "获取文件大小", file.file.len())?;
          Ok(Mbf5::from(len).into())
        } else {
          self.state.error(loc, "未打开文件")?;
        }
      }
      SysFuncKind::Log => {
        let value = self.value_stack.pop().unwrap().1.unwrap_real();
        match value.ln() {
          Ok(result) => Ok(result.into()),
          Err(RealError::Infinite) => self.state.error(
            loc,
            format!(
              "运算结果数值过大，超出实数的表示范围。参数值是：{}",
              value
            ),
          )?,
          Err(RealError::Nan) => self.state.error(
            loc,
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
            loc,
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
            loc,
            format!(
              "运算结果数值过大，超出实数的表示范围。参数值是：{}",
              value
            ),
          )?,
          Err(RealError::Nan) => self.state.error(
            loc,
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

  fn exec_write(
    &mut self,
    loc: Location,
    to_file: bool,
    end: bool,
  ) -> Result<()> {
    let value = self.value_stack.pop().unwrap().1;
    let file = if to_file {
      let filenum = self.get_filenum(end)?;
      if let Some(file) = &mut self.files[filenum as usize] {
        if let FileMode::Output | FileMode::Append = file.mode {
          Some(&mut file.file)
        } else {
          self.state.error(
            loc,
            format!(
              "LOF 函数只能用于以 OUTPUT 或 APPEND 模式打开的文件，\
              但 {} 号文件是以 {} 模式打开的",
              filenum + 1,
              file.mode
            ),
          )?;
        }
      } else {
        self.state.error(loc, "未打开文件，不能执行 WRITE 操作")?;
      }
    } else {
      None
    };

    macro_rules! write_file {
      ($file:ident, $w:expr) => {
        self.state.io(loc.clone(), "写入文件", $file.write($w))?;
      };
    }

    match value {
      TmpValue::Real(num) => {
        if let Some(file) = file {
          write_file!(file, num.to_string().as_bytes());
          if end {
            write_file!(file, &[0xffu8]);
          } else {
            write_file!(file, b",");
          }
        } else {
          self.device.print(num.to_string().as_bytes());
          if !end {
            self.device.print(b",");
          }
        }
      }
      TmpValue::String(s) => {
        if let Some(file) = file {
          write_file!(file, b"\"");
          write_file!(file, &s);
          write_file!(file, b"\"");
          if end {
            write_file!(file, &[0xff]);
          } else {
            write_file!(file, b",");
          }
        } else {
          self.device.print(b"\"");
          self.device.print(&s.to_print_form());
          self.device.print(b"\"");
          if !end {
            self.device.print(b",");
          }
        }
      }
      _ => unreachable!(),
    }

    Ok(())
  }

  fn exec_open(
    &mut self,
    loc: Location,
    mode: ast::FileMode,
    has_len: bool,
  ) -> Result<()> {
    let len = if has_len {
      let mut len = self.pop_u8(false)?;
      if len == 0 || len > 128 {
        len = 32;
      }
      len
    } else {
      32
    };

    let filenum = self.get_filenum(true)?;
    let (name_loc, filename) = self.value_stack.pop().unwrap();
    let mut filename = filename.unwrap_string();

    if self.files[filenum as usize].is_some() {
      self
        .state
        .error(loc, format!("重复打开 {} 号文件", filenum + 1))?;
    }

    if let Some(i) = filename.find_byteset(b"/\\") {
      self.state.error(
        name_loc,
        format!("文件名中不能包含\"{}\"字符", filename[i] as char),
      )?;
    }

    if !filename.to_ascii_uppercase().ends_with(b".DAT") {
      filename.push_str(b".DAT");
    }

    let (mode, read, write, truncate) = match mode {
      ast::FileMode::Input => (FileMode::Input, true, false, false),
      ast::FileMode::Output => (FileMode::Output, false, true, true),
      ast::FileMode::Append => (FileMode::Append, false, true, false),
      ast::FileMode::Random => (
        FileMode::Random {
          record_len: len,
          fields: vec![],
        },
        true,
        true,
        false,
      ),
      _ => unreachable!(),
    };

    let file = self.state.io(
      loc,
      "打开文件",
      self.device.open_file(&filename, read, write, truncate),
    )?;

    self.files[filenum as usize] = Some(OpenFile { file, mode });

    Ok(())
  }

  fn exec_read(&mut self, loc: Location) -> Result<()> {
    if self.data_ptr >= self.data.len() {
      self.state.error(
        loc,
        if self.data.is_empty() {
          "没有 DATA 可供读取"
        } else {
          "DATA 已经读取结束，没有更多 DATA 可供读取"
        },
      )?;
    }

    let datum = &self.data[self.data_ptr];
    self.data_ptr += 1;

    let lvalue = self.value_stack.pop().unwrap().1.unwrap_lvalue();
    match lvalue.get_type(&self.interner) {
      Type::String => {
        let str = datum.value.clone();
        self.store.store_value(lvalue, Value::String(str));
      }
      ty @ (Type::Integer | Type::Real) => {
        if datum.is_quoted {
          self.state.error(
            loc,
            format!(
              "读取到的数据：\"{}\"，是用引号括起来的字符串，无法转换为数值",
              datum.value.to_string_lossy(self.emoji_style)
            ),
          )?
        }

        let mut str = datum.value.clone();
        str.retain(|&b| b != b' ');
        if str.is_empty() {
          let value = if ty == Type::Integer {
            Value::Integer(0)
          } else {
            Value::Real(Mbf5::zero())
          };
          self.store.store_value(lvalue, value);
        } else {
          match unsafe { std::str::from_utf8_unchecked(&str) }.parse::<Mbf5>() {
            Ok(num) => {
              if ty == Type::Integer {
                let int = f64::from(num.truncate());
                if int <= -32769.0 || int >= 32768.0 {
                  self.state.error(
                        loc,
                        format!(
                          "读取到的数据：{}，超出了整数的表示范围（-32768~32767），\
                          无法赋值给整数变量",
                          f64::from(num),
                        ),
                      )?;
                } else {
                  self.store.store_value(lvalue, Value::Integer(int as i16));
                }
              } else {
                self.store.store_value(lvalue, Value::Real(num));
              }
            }
            Err(ParseRealError::Malformed) => {
              self.state.error(
                loc,
                format!(
                  "读取到的数据：{}，不符合实数的格式",
                  datum.value.to_string_lossy(self.emoji_style)
                ),
              )?;
            }
            Err(ParseRealError::Infinite) => {
              self.state.error(
                loc,
                format!(
                  "读取到的数据：{}，数值过大，超出了实数的表示范围",
                  datum.value.to_string_lossy(self.emoji_style)
                ),
              )?;
            }
          }
        }
      }
    }

    Ok(())
  }

  fn exec_field(&mut self, loc: Location, num_fields: usize) -> Result<()> {
    let filenum = self.get_filenum(true)?;
    let record_len;
    if let Some(file) = &self.files[filenum as usize] {
      if let FileMode::Random {
        record_len: len, ..
      } = &file.mode
      {
        record_len = *len as u32;
      } else {
        self.state.error(
              loc,
              format!(
                "FIELD 语句只能用于以 RANDOM 模式打开的文件，但 {} 号文件是以 {} 模式打开的",
                filenum + 1,
                file.mode
              ))?;
      }
    } else {
      self.state.error(loc, "未打开文件")?;
    }

    let mut fields = vec![];
    let mut total_len = 0u32;
    for _ in 0..num_fields {
      let lvalue = self.value_stack.pop().unwrap().1.unwrap_lvalue();
      let len = self.pop_u8(false)?;
      fields.push(RecordField { len, lvalue });
      total_len += len as u32;
    }
    fields.reverse();

    if total_len > record_len {
      self.state.error(
        loc,
        format!(
          "FIELD 语句定义的字段总长度 {} 超出了打开文件时所指定的记录长度 {}",
          total_len, record_len
        ),
      )?;
    }

    match &mut self.files[filenum as usize].as_mut().unwrap().mode {
      FileMode::Random { fields: f, .. } => {
        *f = fields;
      }
      _ => unreachable!(),
    }

    Ok(())
  }

  fn exec_for(
    &mut self,
    _loc: Location,
    name: Symbol,
    has_step: bool,
  ) -> Result<()> {
    let step = if has_step {
      self.value_stack.pop().unwrap().1.unwrap_real()
    } else {
      Mbf5::one()
    };
    let end = self.value_stack.pop().unwrap().1.unwrap_real();
    let start = self.value_stack.pop().unwrap();

    let mut prev_loop = None;
    for (i, item) in self.control_stack.iter().enumerate().rev() {
      if let ControlRecord::ForLoop(ForLoopRecord { var: prev_var, .. }) = item
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

    self.store_tmp_value(LValue::Var { name }, start)?;

    Ok(())
  }

  fn exec_next(&mut self, loc: Location, name: Option<Symbol>) -> Result<()> {
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
      let value = self.load_tmp_value(record.var).unwrap_real();
      let loc = self.code[record.addr.0].loc.clone();
      let new_value = match value + record.step {
        Ok(new_value) => new_value,
        Err(RealError::Infinite) => {
          self.state.error(
            loc.clone(),
            format!("计数器数值过大，超出了实数的表示范围。"),
          )?;
        }
        Err(_) => unreachable!(),
      };

      self.store_tmp_value(
        LValue::Var { name: record.var },
        (loc, new_value.into()),
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
      self.state.error(loc, "NEXT 语句找不到匹配的 FOR 语句")?;
    }

    Ok(())
  }

  fn exec_set(&mut self, _loc: Location, align: Alignment) -> Result<()> {
    let mut value = self.value_stack.pop().unwrap().1.unwrap_string();
    let lvalue = self.value_stack.pop().unwrap().1.unwrap_lvalue();

    let mut str = self
      .store
      .load_value(&self.interner, lvalue.clone())
      .unwrap_string();
    if value.len() > str.len() {
      value.truncate(str.len());
      str = value;
    } else {
      match align {
        Alignment::Left => {
          str.clone_from_slice(&value);
        }
        Alignment::Right => {
          let padding = str.len() - value.len();
          str[padding..].clone_from_slice(&value);
          str[..padding].fill(b' ');
        }
      }
    }
    self.store.store_value(lvalue, Value::String(str));

    Ok(())
  }

  fn exec_get_put<F>(&mut self, loc: Location, action: F) -> Result<()>
  where
    F: FnOnce(&mut Self, usize) -> Result<()>,
  {
    let record_loc = self.value_stack.last().unwrap().0.clone();
    let record = self.pop_range(-32768, 32767)? as i16;
    if record == 0 {
      self.state.error(record_loc, "记录序号不能为 0")?;
    }
    let record = (record - 1) as u16;

    let filenum = self.get_filenum(true)?;
    if let Some(file) = &mut self.files[filenum as usize] {
      if let FileMode::Random { record_len, .. } = &file.mode {
        let offset = record as u64 * *record_len as u64;
        self
          .state
          .io(loc.clone(), "设置文件指针", file.file.seek(offset))?;

        action(self, filenum as usize)
      } else {
        self.state.error(
          loc,
          format!(
            "GET 语句只能用于以 RANDOM 模式打开的文件，但 {} 号文件是以 {} 模式打开的",
            filenum + 1,
            file.mode
          )
        )?;
      }
    } else {
      self.state.error(loc, "未打开文件")?;
    }
  }

  fn assign_key(&mut self, input: ExecInput) {
    match input {
      ExecInput::Key(key) => {
        self
          .value_stack
          .push((self.code[self.pc].loc.clone(), Mbf5::from(key).into()));
      }
      _ => unreachable!(),
    }
    self.pc += 1;
  }

  fn assign_input(
    &mut self,
    input: ExecInput,
    lvalues: Vec<(Location, LValue)>,
  ) {
    match input {
      ExecInput::KeyboardInput(values) => {
        let mut comma = false;
        for ((lval_loc, lvalue), value) in lvalues.into_iter().zip(values) {
          if comma {
            self.device.print(b",");
          }
          comma = true;
          match value {
            KeyboardInput::Integer(num) => {
              self.device.print(num.to_string().as_bytes());
              self.store.store_value(lvalue, Value::Integer(num));
            }
            KeyboardInput::Real(num) => {
              self.device.print(num.to_string().as_bytes());
              self.store.store_value(lvalue, Value::Real(num));
            }
            KeyboardInput::String(s) => {
              self.device.print(b"\"");
              self.device.print(&s.to_print_form());
              self.device.print(b"\"");
              self.store.store_value(lvalue, Value::String(s));
            }
            KeyboardInput::Func { body } => {
              let (name, param) = match &lvalue {
                LValue::Fn { name, param } => {
                  self.device.print(
                    format!(
                      "FN {}({})",
                      self.interner.resolve(*name).unwrap(),
                      self.interner.resolve(*param).unwrap()
                    )
                    .as_bytes(),
                  );
                  (*name, *param)
                }
                _ => unreachable!(),
              };

              let mut sym_map = HashMap::default();
              for (sym, name) in &body.interner {
                let new_sym = self.interner.get_or_intern(name);
                sym_map.insert(sym, new_sym);
              }

              let body_addr = Addr(self.code.len());
              self.code.extend(body.code.into_iter().map(|instr| Instr {
                loc: lval_loc.clone(),
                kind: instr.kind.map_symbol(&sym_map),
              }));
              self.code.push(Instr {
                loc: lval_loc,
                kind: InstrKind::ReturnFn,
              });

              self
                .store
                .user_funcs
                .insert(name, UserFunc { param, body_addr });
            }
          };
        }
      }
      _ => unreachable!(),
    }
    self.device.print_newline();
    self.pc += 1;
  }

  fn calc_array_offset(
    &mut self,
    name: Symbol,
    dimensions: usize,
  ) -> Result<usize> {
    if !self.store.arrays.contains_key(&name) {
      let data = ArrayData::new(
        symbol_type(&self.interner, name),
        11usize.pow(dimensions as u32),
      );
      self.store.arrays.insert(
        name,
        Array {
          bounds: vec![unsafe { NonZeroUsize::new_unchecked(11) }; dimensions],
          data,
        },
      );
    }

    let array = &self.store.arrays[&name];
    let mut offset = 0;
    for i in (0..dimensions).rev() {
      let (loc, value) = self.value_stack.pop().unwrap();
      let value = value.unwrap_real();
      let bound = f64::from(value.truncate()) as isize;
      if bound < 0 {
        self.state.error(
          loc,
          format!(
            "数组下标不能为负数。该下标的值为：{}，取整后的值为：{}",
            f64::from(value),
            bound
          ),
        )?
      } else if bound as usize >= array.bounds[i].get() {
        self.state.error(
          loc,
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
    let (value_loc, value) = self.value_stack.pop().unwrap();
    let value = value.unwrap_real();

    let value = f64::from(value);
    if value <= min as f64 - 1.0 || value >= max as f64 + 1.0 {
      self.state.error(
        value_loc,
        format!("参数超出范围 {}~{}。运算结果为：{}", min, max, value),
      )?;
    }
    Ok(value as i32)
  }

  /// Returns [0, 2].
  fn get_filenum(&mut self, pop: bool) -> Result<u8> {
    let (loc, value) = if pop {
      self.value_stack.pop().unwrap()
    } else {
      self.value_stack.last().cloned().unwrap()
    };
    let int = f64::from(value.unwrap_real()) as i64;
    if int >= 1 && int <= 3 {
      Ok(int as u8 - 1)
    } else {
      self.state.error(loc, "文件号超出范围 1~3")?
    }
  }

  fn load_tmp_value(&mut self, name: Symbol) -> TmpValue {
    self
      .store
      .load_value(&self.interner, LValue::Var { name })
      .into()
  }

  fn store_tmp_value(
    &mut self,
    lvalue: LValue,
    (rvalue_loc, rvalue): (Location, TmpValue),
  ) -> Result<()> {
    let value = match lvalue.get_type(&self.interner) {
      Type::Integer => {
        let value = rvalue.unwrap_real();
        let int = f64::from(value.truncate());
        if int <= -32769.0 || int >= 32768.0 {
          self.state.error(
            rvalue_loc,
            format!(
              "运算结果数值过大，超出了整数的表示范围（-32768~32767），\
              无法赋值给整数变量。运算结果为：{}",
              f64::from(value),
            ),
          )?;
        }
        Value::Integer(int as i16)
      }
      Type::Real => Value::Real(rvalue.unwrap_real()),
      Type::String => Value::String(rvalue.unwrap_string()),
    };
    self.store.store_value(lvalue, value);
    Ok(())
  }
}

fn exec_file_input<F: FileHandle>(
  state: &mut ExecState,
  store: &mut Store,
  interner: &StringInterner,
  emoji_style: EmojiStyle,
  loc: Location,
  lvalue: LValue,
  file: &mut F,
) -> Result<()> {
  let mut buf = vec![];
  let mut quoted = false;
  {
    let mut byte = [0];
    let len = state.io(loc.clone(), "读取文件", file.read(&mut byte))?;
    if len != 0 {
      if byte[0] == b'"' {
        quoted = true;
      } else {
        buf.push(byte[0]);
      }

      loop {
        let mut byte = [0];
        let len = state.io(loc.clone(), "读取文件", file.read(&mut byte))?;
        if len == 0 {
          if quoted {
            state.error(loc.clone(), "读取字符串时遇到未匹配的双引号")?
          }
          break;
        }
        if quoted {
          if byte[0] == b'"' {
            break;
          }
        } else if byte[0] == 0xff || byte[0] == b',' {
          break;
        }
        buf.push(byte[0]);
      }
    }
  }

  let value = match lvalue.get_type(interner) {
    ty @ (Type::Integer | Type::Real) => {
      if quoted {
        state.error(
          loc,
          format!(
            "读取到的数据：\"{}\"，是用引号括起来的字符串，无法转换为数值",
            ByteString::from(buf).to_string_lossy(emoji_style)
          ),
        )?
      }

      match unsafe { std::str::from_utf8_unchecked(&buf) }.parse::<Mbf5>() {
        Ok(num) => {
          if ty == Type::Integer {
            let int = f64::from(num.truncate());
            if int <= -32769.0 || int >= 32768.0 {
              state.error(
                loc,
                format!(
                  "读取到的数值：{}，超出了整数的表示范围（-32768~32767），\
                          无法赋值给整数变量",
                  f64::from(num),
                ),
              )?;
            } else {
              Value::Integer(int as i16)
            }
          } else {
            Value::Real(num)
          }
        }
        Err(ParseRealError::Malformed) => {
          state.error(
            loc,
            format!(
              "读取到的数据：{}，不符合实数的格式",
              ByteString::from(buf).to_string_lossy(emoji_style)
            ),
          )?;
        }
        Err(ParseRealError::Infinite) => {
          state.error(
            loc,
            format!(
              "读取到的数据：{}，数值过大，超出了实数的表示范围",
              ByteString::from(buf).to_string_lossy(emoji_style)
            ),
          )?;
        }
      }
    }
    Type::String => Value::String(buf.into()),
  };

  store.store_value(lvalue, value);

  Ok(())
}

impl ExecState {
  #[must_use]
  fn error<S: ToString>(
    &mut self,
    location: Location,
    message: S,
  ) -> Result<!> {
    *self = Self::Done;
    Err(ExecResult::Error {
      location,
      message: message.to_string(),
    })
  }

  #[must_use]
  fn inkey(&mut self) -> Result<!> {
    *self = Self::WaitForKey;
    Err(ExecResult::InKey)
  }

  #[must_use]
  fn input(
    &mut self,
    lvalues: Vec<(Location, LValue)>,
    prompt: Option<String>,
    fields: Vec<KeyboardInputType>,
  ) -> Result<!> {
    *self = Self::WaitForKeyboardInput { lvalues };
    Err(ExecResult::KeyboardInput { prompt, fields })
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

  #[must_use]
  fn sleep(&mut self, duration: Duration) -> Result<!> {
    *self = Self::Normal;
    Err(ExecResult::Sleep(duration))
  }

  fn io<T>(
    &mut self,
    loc: Location,
    op: &str,
    result: io::Result<T>,
  ) -> Result<T> {
    match result {
      Ok(v) => Ok(v),
      Err(err) => {
        let err = match err.kind() {
          io::ErrorKind::NotFound => "文件不存在".to_owned(),
          io::ErrorKind::AlreadyExists => "文件已存在".to_owned(),
          io::ErrorKind::IsADirectory => "是文件夹".to_owned(),
          io::ErrorKind::PermissionDenied => "没有权限".to_owned(),
          _ => err.to_string(),
        };
        self.error(loc, format!("{}时发生错误：{}", op, err))?
      }
    }
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

impl Value {
  fn unwrap_string(self) -> ByteString {
    match self {
      Self::String(s) => s,
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

impl LValue {
  fn get_type(&self, interner: &StringInterner) -> Type {
    let name = match self {
      Self::Var { name } => *name,
      Self::Index { name, .. } => *name,
      Self::Fn { name, .. } => *name,
    };
    symbol_type(interner, name)
  }
}

impl Display for FileMode {
  fn fmt(&self, f: &mut Formatter) -> fmt::Result {
    match self {
      Self::Input => write!(f, "INPUT"),
      Self::Output => write!(f, "OUTPUT"),
      Self::Append => write!(f, "APPEND"),
      Self::Random { .. } => write!(f, "RANDOM"),
    }
  }
}

impl Store {
  fn clear(&mut self) {
    self.vars.clear();
    self.arrays.clear();
    self.user_funcs.clear();
  }

  fn store_value(&mut self, lvalue: LValue, value: Value) {
    match lvalue {
      LValue::Var { name } => {
        self.vars.insert(name, value);
      }
      LValue::Index { name, offset } => {
        match (&mut self.arrays.get_mut(&name).unwrap().data, value) {
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

  fn load_value(&mut self, interner: &StringInterner, lvalue: LValue) -> Value {
    match lvalue {
      LValue::Var { name } => {
        let ty = symbol_type(interner, name);
        self
          .vars
          .entry(name)
          .or_insert_with(|| match ty {
            Type::Integer => Value::Integer(0),
            Type::Real => Value::Real(Mbf5::zero()),
            Type::String => Value::String(ByteString::new()),
          })
          .clone()
      }
      LValue::Index { name, offset } => match &self.arrays[&name].data {
        ArrayData::Integer(arr) => Value::Integer(arr[offset]),
        ArrayData::Real(arr) => Value::Real(arr[offset]),
        ArrayData::String(arr) => Value::String(arr[offset].clone()),
      },
      _ => unreachable!(),
    }
  }
}
