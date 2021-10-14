use bstr::{ByteSlice, ByteVec};
use nanorand::{Rng, WyRand};
use std::fmt::{self, Display, Formatter};
use std::io;
use std::num::NonZeroUsize;
use std::time::Duration;

use crate::ast::{self, SysFuncKind};
use crate::compiler::compile_fn_body;
use crate::diagnostic::{contains_errors, Diagnostic};
use crate::machine::EmojiStyle;
use crate::parser::{parse_expr, read_number};
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
  num_stack: Vec<(Location, Mbf5)>,
  str_stack: Vec<(Location, ByteString)>,
  lval_stack: Vec<(Location, LValue)>,
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
  param_org_value: Value,
  next_addr: Addr,
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
  dimensions: Vec<Dimension>,
  data: ArrayData,
}

#[derive(Debug, Clone)]
struct Dimension {
  bound: NonZeroUsize,
  multiplier: usize,
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

#[derive(Debug, Clone, PartialEq, Eq)]
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

#[derive(Debug, Clone, PartialEq, Eq)]
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

impl InputFuncBody {
  pub fn new(codegen: CodeGen) -> Self {
    assert!(codegen.data.is_empty());
    Self {
      interner: codegen.interner,
      code: codegen.code,
    }
  }
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
      num_stack: vec![],
      str_stack: vec![],
      lval_stack: vec![],
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

  pub fn reset(&mut self, loc: Location, reset_pc: bool) -> Result<()> {
    self.data_ptr = 0;
    if reset_pc {
      self.pc = 0;
    }
    self.code.truncate(self.code_len);
    self.control_stack.clear();
    self.num_stack.clear();
    self.str_stack.clear();
    self.lval_stack.clear();
    self.store.clear();
    self.fn_call_stack.clear();
    //self.device.clear();
    self.close_files(loc)?;
    self.rng = WyRand::new();
    self.current_rand = self.rng.generate();
    self.state = ExecState::Normal;
    Ok(())
  }

  fn close_files(&mut self, loc: Location) -> Result<()> {
    for file in &mut self.files {
      if let Some(file) = file.take() {
        self.state.io(loc.clone(), "关闭文件", file.file.close())?;
      }
    }
    Ok(())
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
    let kind = instr.kind.clone();

    let result = self.do_exec_instr(steps, loc.clone(), kind);
    if let ExecState::Done = &self.state {
      result.and(self.close_files(loc))
    } else {
      result
    }
  }

  fn do_exec_instr(
    &mut self,
    steps: &mut usize,
    loc: Location,
    kind: InstrKind,
  ) -> Result<()> {
    macro_rules! write_file {
      ($file:ident, $w:expr) => {
        self.state.io(loc.clone(), "写入文件", $file.write($w))?;
      };
    }

    macro_rules! do_write {
      (
        $to_file:ident,
        $end:ident,
        $file:ident => $write_file:expr,
        $write_screen:expr
      ) => {{
        if $to_file {
          let filenum = self.get_filenum($end)?;
          if let Some(file) = &mut self.files[filenum as usize] {
            if let FileMode::Output | FileMode::Append = file.mode {
              let $file = &mut file.file;
              $write_file;
              if $end {
                write_file!($file, &[0xffu8]);
              } else {
                write_file!($file, b",");
              }
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
          $write_screen;
          if !$end {
            self.device.print(b",");
          }
        };
      }}
    }

    match kind {
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
      InstrKind::DimArray {
        name,
        dimensions: num_dimensions,
      } => {
        if self.store.arrays.contains_key(&name) {
          self.state.error(loc, "重复定义数组")?;
        }
        let mut size = 1;
        let mut multiplier = 1;
        let mut dimensions = vec![];
        let start = self.num_stack.len() - num_dimensions.get();
        self.num_stack[start..].reverse();
        for _ in 0..num_dimensions.get() {
          let (loc, value) = self.num_stack.pop().unwrap();
          let bound = f64::from(value.truncate()) as isize;
          if bound < 0 {
            self.state.error(
              loc,
              format!("数组下标不能为负数。该下标的值为：{}", f64::from(value)),
            )?
          }
          let bound = bound as usize + 1;
          size *= bound;
          dimensions.push(Dimension {
            bound: unsafe { NonZeroUsize::new_unchecked(bound) },
            multiplier,
          });
          multiplier *= bound;
        }
        let data = ArrayData::new(symbol_type(&self.interner, name), size);
        self.store.arrays.insert(name, Array { dimensions, data });
      }
      InstrKind::PushVarLValue { name } => {
        self.lval_stack.push((loc, LValue::Var { name }));
      }
      InstrKind::PushIndexLValue { name, dimensions } => {
        let offset = self.calc_array_offset(name, dimensions)?;
        self.lval_stack.push((loc, LValue::Index { name, offset }));
      }
      InstrKind::PushFnLValue { name, param } => {
        self.lval_stack.push((loc, LValue::Fn { name, param }));
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
        let value = self.num_stack.pop().unwrap().1;
        if value.is_zero() {
          self.pc = target.0;
        } else {
          self.pc += 1;
        }
        return Ok(());
      }
      InstrKind::CallFn(func) => {
        if let Some(func) = self.store.user_funcs.get(&func).cloned() {
          let arg = self.num_stack.pop().unwrap();
          let param_org_value = self
            .store
            .load_value(&self.interner, LValue::Var { name: func.param });
          self.fn_call_stack.push(FnCallRecord {
            param: func.param,
            param_org_value,
            next_addr: Addr(self.pc + 1),
          });
          self.store_num(LValue::Var { name: func.param }, arg)?;
          self.pc = func.body_addr.0;
        } else {
          self.state.error(loc, "自定义函数不存在")?;
        }
        return Ok(());
      }
      InstrKind::ReturnFn => {
        let record = self.fn_call_stack.pop().unwrap();
        self.store.store_value(
          LValue::Var { name: record.param },
          record.param_org_value,
        );
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
      InstrKind::PopNum => {
        self.num_stack.pop().unwrap();
      }
      InstrKind::PopStr => {
        self.str_stack.pop().unwrap();
      }
      InstrKind::PushNum(num) => {
        self.num_stack.push((loc, num));
      }
      InstrKind::PushVar(name) => {
        match self.store.load_value(&self.interner, LValue::Var { name }) {
          Value::Integer(n) => self.num_stack.push((loc, n.into())),
          Value::Real(n) => self.num_stack.push((loc, n)),
          Value::String(s) => self.str_stack.push((loc, s)),
        }
      }
      InstrKind::PushStr(str) => {
        self.str_stack.push((loc, str));
      }
      InstrKind::PushInKey => {
        self.state.inkey()?;
      }
      InstrKind::PushIndex { name, dimensions } => {
        let offset = self.calc_array_offset(name, dimensions)?;
        match &self.store.arrays[&name].data {
          ArrayData::Integer(arr) => {
            self.num_stack.push((loc, Mbf5::from(arr[offset])));
          }
          ArrayData::Real(arr) => {
            self.num_stack.push((loc, arr[offset]));
          }
          ArrayData::String(arr) => {
            self.str_stack.push((loc, arr[offset].clone().into()));
          }
        };
      }
      InstrKind::Not => {
        let value = self.num_stack.pop().unwrap().1;
        self.num_stack.push((loc, Mbf5::from(value.is_zero())));
      }
      InstrKind::Neg => {
        let value = self.num_stack.pop().unwrap().1;
        self.num_stack.push((loc, -value));
      }
      InstrKind::CmpNum(cmp) => {
        let rhs = self.num_stack.pop().unwrap().1;
        let lhs = self.num_stack.pop().unwrap().1;
        self.num_stack.push((loc, Mbf5::from(cmp.cmp(lhs, rhs))));
      }
      InstrKind::CmpStr(cmp) => {
        let rhs = self.str_stack.pop().unwrap().1;
        let lhs = self.str_stack.pop().unwrap().1;
        self.num_stack.push((loc, Mbf5::from(cmp.cmp(lhs, rhs))));
      }
      InstrKind::Concat => {
        let mut rhs = self.str_stack.pop().unwrap().1;
        let mut lhs = self.str_stack.pop().unwrap().1;
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
        self.str_stack.push((loc, lhs));
      }
      InstrKind::Add => {
        let rhs = self.num_stack.pop().unwrap().1;
        let lhs = self.num_stack.pop().unwrap().1;
        match lhs + rhs {
          Ok(result) => self.num_stack.push((loc, result)),
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
        }
      }
      InstrKind::Sub => {
        let rhs = self.num_stack.pop().unwrap().1;
        let lhs = self.num_stack.pop().unwrap().1;
        match lhs - rhs {
          Ok(result) => self.num_stack.push((loc, result)),
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
        let rhs = self.num_stack.pop().unwrap().1;
        let lhs = self.num_stack.pop().unwrap().1;
        match lhs * rhs {
          Ok(result) => self.num_stack.push((loc, result)),
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
        let rhs = self.num_stack.pop().unwrap().1;
        if rhs.is_zero() {
          self.state.error(loc, "除以 0")?;
        }
        let lhs = self.num_stack.pop().unwrap().1;
        match lhs / rhs {
          Ok(result) => self.num_stack.push((loc, result)),
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
        let rhs = self.num_stack.pop().unwrap().1;
        let lhs = self.num_stack.pop().unwrap().1;
        match lhs.pow(rhs) {
          Ok(result) => self.num_stack.push((loc, result)),
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
        let rhs = self.num_stack.pop().unwrap().1;
        let lhs = self.num_stack.pop().unwrap().1;
        self
          .num_stack
          .push((loc, Mbf5::from(!lhs.is_zero() && !rhs.is_zero())));
      }
      InstrKind::Or => {
        let rhs = self.num_stack.pop().unwrap().1;
        let lhs = self.num_stack.pop().unwrap().1;
        self
          .num_stack
          .push((loc, Mbf5::from(!lhs.is_zero() || !rhs.is_zero())));
      }
      InstrKind::SysFuncCall { kind, arity } => {
        self.exec_sys_func(loc, kind, arity)?;
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
      InstrKind::PrintNum => {
        let value = self.num_stack.pop().unwrap().1;
        self.device.print(value.to_string().as_bytes());
      }
      InstrKind::PrintStr => {
        let mut value = self.str_stack.pop().unwrap().1;
        value.drop_null();
        value.drop_0x1f();
        self.device.print(&value);
      }
      InstrKind::Flush => {
        self.device.flush();
      }
      InstrKind::SetRow => {
        let row = self.pop_range(1, 5)? as u8 - 1;
        self.device.set_row(row);
      }
      InstrKind::SetColumn => {
        let col = self.pop_range(1, 20)? as u8 - 1;
        self.device.set_column(col);
      }
      InstrKind::WriteNum { to_file, end } => {
        let num = self.num_stack.pop().unwrap().1;
        do_write!(
          to_file,
          end,
          file => {
            write_file!(file, num.to_string().as_bytes());
          },
          {
            self.device.print(num.to_string().as_bytes());
          }
        );
      }
      InstrKind::WriteStr { to_file, end } => {
        let mut str = self.str_stack.pop().unwrap().1;
        str.push(b'"');
        str.drop_null();
        do_write!(
          to_file,
          end,
          file => {
            write_file!(file, b"\"");
            write_file!(file, &str);
          },
          {
            self.device.print(b"\"");
            self.device.print(&str);
          }
        );
      }
      InstrKind::KeyboardInput {
        prompt,
        fields: num_fields,
      } => {
        let mut lvalues = vec![];
        let mut fields = vec![];
        for _ in 0..num_fields.get() {
          let (lval_loc, lvalue) = self.lval_stack.pop().unwrap();
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
        lvalues.reverse();
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

        let offset = self.lval_stack.len() - num_fields.get();
        for (lval_loc, lvalue) in self.lval_stack.drain(offset..) {
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
        self.reset(loc, false)?;
      }
      InstrKind::CloseFile => {
        let filenum = self.get_filenum(true)?;
        if let Some(file) = self.files[filenum as usize].take() {
          self.state.io(loc, "关闭文件", file.file.close())?;
        } else {
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
      InstrKind::AssignNum => {
        let (_, lvalue) = self.lval_stack.pop().unwrap();
        let num = self.num_stack.pop().unwrap();
        self.store_num(lvalue, num)?;
      }
      InstrKind::AssignStr => {
        let (_, lvalue) = self.lval_stack.pop().unwrap();
        let str = self.str_stack.pop().unwrap().1;
        self.store.store_value(lvalue, Value::String(str));
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
        let value = self.str_stack.pop().unwrap().1;
        self.device.play_notes(&value);
      }
      InstrKind::Poke => {
        let value = self.pop_u8(false)?;
        let addr = self.pop_range(-65535, 65535)? as u16;
        self.device.set_byte(addr, value);
      }
      InstrKind::Swap => {
        let lvalue2 = self.lval_stack.pop().unwrap().1;
        let lvalue1 = self.lval_stack.pop().unwrap().1;
        let value1 = self.store.load_value(&self.interner, lvalue1.clone());
        let value2 = self.store.load_value(&self.interner, lvalue2.clone());
        self.store.store_value(lvalue2, value1);
        self.store.store_value(lvalue1, value2);
      }
      InstrKind::Restart => {
        self.device.set_screen_mode(ScreenMode::Text);
        self.device.cls();
        self.reset(loc, true)?;
        return Ok(());
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
        let value = self.num_stack.pop().unwrap().1;
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
        let value = self.num_stack.pop().unwrap().1;
        if value.is_positive() {
          self.pc += 1;
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
    arity: NonZeroUsize,
  ) -> Result<()> {
    match kind {
      SysFuncKind::Abs => {
        let value = self.num_stack.pop().unwrap().1;
        self.num_stack.push((loc, value.abs()));
        Ok(())
      }
      SysFuncKind::Asc => {
        let value = self.str_stack.pop().unwrap().1;
        if value.is_empty() {
          self.state.error(loc, "ASC 函数的参数为空字符串")?;
        }
        self.num_stack.push((loc, Mbf5::from(value[0])));
        Ok(())
      }
      SysFuncKind::Atn => {
        let value = self.num_stack.pop().unwrap().1;
        self.num_stack.push((loc, value.atan()));
        Ok(())
      }
      SysFuncKind::Chr => {
        let value = self.pop_u8(false)?;
        self.str_stack.push((loc, ByteString::from(vec![value])));
        Ok(())
      }
      SysFuncKind::Cos => {
        let value = self.num_stack.pop().unwrap().1;
        self.num_stack.push((loc, value.cos()));
        Ok(())
      }
      SysFuncKind::Cvi => {
        let value = self.str_stack.pop().unwrap().1;
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
        self
          .num_stack
          .push((loc, Mbf5::from((lo + (hi << 8)) as i16)));
        Ok(())
      }
      SysFuncKind::Cvs => {
        let value = self.str_stack.pop().unwrap().1;
        if value.len() != 5 {
          self.state.error(
            loc,
            format!(
              "CVS$ 函数的参数字符串长度不等于 5。参数字符串长度为：{}",
              value.len()
            ),
          )?;
        }
        self.num_stack.push((
          loc,
          Mbf5::from([value[0], value[1], value[2], value[3], value[4]]),
        ));
        Ok(())
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
          let pos =
            self
              .state
              .io(loc.clone(), "获取文件指针", file.file.pos())?;
          self.num_stack.push((loc, Mbf5::from(pos >= len)));
          Ok(())
        } else {
          self.state.error(loc, "未打开文件")?;
        }
      }
      SysFuncKind::Exp => {
        let value = self.num_stack.pop().unwrap().1;
        match value.exp() {
          Ok(value) => {
            self.num_stack.push((loc, value));
            Ok(())
          }
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
        let value = self.num_stack.pop().unwrap().1;
        self.num_stack.push((loc, value.truncate()));
        Ok(())
      }
      SysFuncKind::Left => {
        let len = self.pop_u8(true)? as usize;
        let value = self.str_stack.pop().unwrap().1;
        let len = len.max(value.len());
        self
          .str_stack
          .push((loc, ByteString::from(value[..len].to_vec())));
        Ok(())
      }
      SysFuncKind::Len => {
        let value = self.str_stack.pop().unwrap().1;
        self.num_stack.push((loc, Mbf5::from(value.len() as u32)));
        Ok(())
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
          let len =
            self
              .state
              .io(loc.clone(), "获取文件大小", file.file.len())?;
          self.num_stack.push((loc, Mbf5::from(len)));
          Ok(())
        } else {
          self.state.error(loc, "未打开文件")?;
        }
      }
      SysFuncKind::Log => {
        let value = self.num_stack.pop().unwrap().1;
        match value.ln() {
          Ok(value) => {
            self.num_stack.push((loc, value));
            Ok(())
          }
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
        let len = if arity.get() == 3 {
          self.pop_u8(false)? as usize
        } else {
          255
        };
        let pos = (self.pop_u8(true)? - 1) as usize;
        let value = self.str_stack.pop().unwrap().1;
        let start = pos.max(value.len());
        let end = (start + len).max(value.len());
        self
          .str_stack
          .push((loc, ByteString::from(value[start..end].to_vec())));
        Ok(())
      }
      SysFuncKind::Mki => {
        let value = self.pop_range(-32768, 32767)? as i16;
        let lo = (value & 0xff) as u8;
        let hi = (value >> 8) as u8;
        self.str_stack.push((loc, ByteString::from(vec![lo, hi])));
        Ok(())
      }
      SysFuncKind::Mks => {
        let value = self.num_stack.pop().unwrap().1;
        self
          .str_stack
          .push((loc, ByteString::from(<[u8; 5]>::from(value).to_vec())));
        Ok(())
      }
      SysFuncKind::Peek => {
        let addr = self.pop_range(-65535, 65535)? as u16;
        let value = self.device.get_byte(addr);
        self.num_stack.push((loc, Mbf5::from(value)));
        Ok(())
      }
      SysFuncKind::Pos => {
        self.num_stack.pop().unwrap();
        self
          .num_stack
          .push((loc, Mbf5::from(self.device.get_column())));
        Ok(())
      }
      SysFuncKind::Right => {
        let len = self.pop_u8(true)? as usize;
        let value = self.str_stack.pop().unwrap().1;
        let len = len.max(value.len());
        self
          .str_stack
          .push((loc, ByteString::from(value[value.len() - len..].to_vec())));
        Ok(())
      }
      SysFuncKind::Rnd => {
        let value = self.num_stack.pop().unwrap().1;
        if value.is_zero() {
          self
            .num_stack
            .push((loc, u32_to_random_number(self.current_rand)));
          return Ok(());
        }
        if value.is_negative() {
          self.rng.reseed(&<[u8; 5]>::from(value));
        }
        let value: u32 = self.rng.generate();
        self.current_rand = value;
        self.num_stack.push((loc, u32_to_random_number(value)));
        Ok(())
      }
      SysFuncKind::Sgn => {
        let value = self.num_stack.pop().unwrap().1;
        let num = if value.is_positive() {
          Mbf5::one().into()
        } else if value.is_negative() {
          Mbf5::neg_one().into()
        } else {
          Mbf5::zero().into()
        };
        self.num_stack.push((loc, num));
        Ok(())
      }
      SysFuncKind::Sin => {
        let value = self.num_stack.pop().unwrap().1;
        self.num_stack.push((loc, value.sin()));
        Ok(())
      }
      SysFuncKind::Sqr => {
        let value = self.num_stack.pop().unwrap().1;
        match value.sqrt() {
          Ok(value) => {
            self.num_stack.push((loc, value));
            Ok(())
          }
          Err(RealError::Nan) => self.state.error(
            loc,
            format!("超出 SQR 函数的定义域。参数值是：{}", value),
          )?,
          Err(RealError::Infinite) => unreachable!(),
        }
      }
      SysFuncKind::Str => {
        let value = self.num_stack.pop().unwrap().1;
        self
          .str_stack
          .push((loc, ByteString::from(value.to_string().into_bytes())));
        Ok(())
      }
      SysFuncKind::Tan => {
        let value = self.num_stack.pop().unwrap().1;
        match value.tan() {
          Ok(value) => {
            self.num_stack.push((loc, value));
            Ok(())
          }
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
        let mut value = self.str_stack.pop().unwrap().1;
        value.retain(|&b| b != b' ');
        let (len, _) = read_number(&*value, false);
        let num = unsafe { std::str::from_utf8_unchecked(&value[..len]) }
          .parse::<Mbf5>()
          .unwrap_or(Mbf5::zero());
        self.num_stack.push((loc, num));
        Ok(())
      }
      SysFuncKind::Tab | SysFuncKind::Spc => unreachable!(),
    }
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
    let (name_loc, mut filename) = self.str_stack.pop().unwrap();
    filename.drop_null();
    filename.drop_0x1f();

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

    let mut file = self.state.io(
      loc.clone(),
      "打开文件",
      self.device.open_file(&filename, read, write, truncate),
    )?;

    if let FileMode::Append = &mode {
      let len = self.state.io(loc.clone(), "获取文件大小", file.len())?;
      self.state.io(loc, "设置文件指针", file.seek(len))?;
    }

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

    let lvalue = self.lval_stack.pop().unwrap().1;
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
          )
        )?;
      }
    } else {
      self.state.error(loc, "未打开文件")?;
    }

    let mut fields = vec![];
    let mut total_len = 0u32;
    for _ in 0..num_fields {
      let lvalue = self.lval_stack.pop().unwrap().1;
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
      self.num_stack.pop().unwrap().1
    } else {
      Mbf5::one()
    };
    let end = self.num_stack.pop().unwrap().1;
    let start = self.num_stack.pop().unwrap();

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

    self.store_num(LValue::Var { name }, start)?;

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
      let value = self
        .store
        .load_value(&self.interner, LValue::Var { name: record.var })
        .unwrap_real();
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

      self.store_num(LValue::Var { name: record.var }, (loc, new_value))?;

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
    let mut value = self.str_stack.pop().unwrap().1;
    let lvalue = self.lval_stack.pop().unwrap().1;

    let mut dest = self
      .store
      .load_value(&self.interner, lvalue.clone())
      .unwrap_string();
    if value.len() > dest.len() {
      value.truncate(dest.len());
      dest = value;
    } else {
      match align {
        Alignment::Left => {
          dest[..value.len()].clone_from_slice(&value);
        }
        Alignment::Right => {
          let padding = dest.len() - value.len();
          dest[padding..].clone_from_slice(&value);
          dest[..padding].fill(b' ');
        }
      }
    }
    self.store.store_value(lvalue, Value::String(dest));

    Ok(())
  }

  fn exec_get_put<F>(&mut self, loc: Location, action: F) -> Result<()>
  where
    F: FnOnce(&mut Self, usize) -> Result<()>,
  {
    let record_loc = self.num_stack.last().unwrap().0.clone();
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
          .str_stack
          .push((self.code[self.pc].loc.clone(), ByteString::from(vec![key])));
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
              self.device.print(&s);
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
    dimensions: NonZeroUsize,
  ) -> Result<usize> {
    let dimensions = dimensions.get();

    if !self.store.arrays.contains_key(&name) {
      let data = ArrayData::new(
        symbol_type(&self.interner, name),
        11usize.pow(dimensions as u32),
      );
      self.store.arrays.insert(
        name,
        Array {
          dimensions: (0..dimensions)
            .fold((vec![], 1), |(mut d, mult), _| {
              d.push(Dimension {
                bound: unsafe { NonZeroUsize::new_unchecked(11) },
                multiplier: mult,
              });
              (d, mult * 11)
            })
            .0,
          data,
        },
      );
    }

    let array = &self.store.arrays[&name];
    let mut offset = 0;
    for i in (0..dimensions).rev() {
      let (loc, value) = self.num_stack.pop().unwrap();
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
      } else if bound as usize >= array.dimensions[i].bound.get() {
        self.state.error(
          loc,
          format!(
            "数组下标超出上限。该下标的上限为：{}，该下标的值为：{}, 取整后的值为：{}",
            array.dimensions[i].bound.get() - 1,
            f64::from(value),
            bound
          ),
        )?
      }

      offset += bound as usize * array.dimensions[i].multiplier;
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
    let (value_loc, value) = self.num_stack.pop().unwrap();

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
      self.num_stack.pop().unwrap()
    } else {
      self.num_stack.last().cloned().unwrap()
    };
    let int = f64::from(value) as i64;
    if int >= 1 && int <= 3 {
      Ok(int as u8 - 1)
    } else {
      self.state.error(loc, "文件号超出范围 1~3")?
    }
  }

  fn store_num(
    &mut self,
    lvalue: LValue,
    (loc, num): (Location, Mbf5),
  ) -> Result<()> {
    let value = match lvalue.get_type(&self.interner) {
      Type::Integer => {
        let int = f64::from(num.truncate());
        if int <= -32769.0 || int >= 32768.0 {
          self.state.error(
            loc,
            format!(
              "运算结果数值过大，超出了整数的表示范围（-32768~32767），\
              无法赋值给整数变量。运算结果为：{}",
              f64::from(num),
            ),
          )?;
        }
        Value::Integer(int as i16)
      }
      Type::Real => Value::Real(num),
      _ => unreachable!(),
    };
    self.store.store_value(lvalue, value);
    Ok(())
  }

  pub fn compile_fn_body(
    &self,
    input: &str,
  ) -> std::result::Result<InputFuncBody, Vec<Diagnostic>> {
    let (mut expr, _) = parse_expr(input);
    let mut codegen = CodeGen::new(self.emoji_style);
    compile_fn_body(input, &mut expr, &mut codegen);
    if contains_errors(&expr.diagnostics) {
      Err(expr.diagnostics)
    } else {
      Ok(InputFuncBody::new(codegen))
    }
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
          io::ErrorKind::FileTooLarge => "文件大小超出64KB的限制".to_owned(),
          _ => err.to_string(),
        };
        self.error(loc, format!("{}时发生错误：{}", op, err))?
      }
    }
  }
}

impl Value {
  fn unwrap_real(self) -> Mbf5 {
    match self {
      Self::Real(n) => n,
      _ => unreachable!(),
    }
  }

  fn unwrap_string(self) -> ByteString {
    match self {
      Self::String(s) => s,
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

#[cfg(test)]
mod tests {
  use super::*;
  use crate::ast::Range;
  use crate::compiler::compile_prog;
  use crate::diagnostic::Severity;
  use crate::machine::EmojiStyle;
  use crate::parser::parse_prog;
  use crate::vm::codegen::CodeGen;
  use insta::assert_snapshot;
  use pretty_assertions::assert_eq;
  use std::cell::RefCell;
  use std::rc::Rc;

  fn compile(text: &str) -> CodeGen {
    let mut prog = parse_prog(text);
    let mut codegen = CodeGen::new(EmojiStyle::New);
    compile_prog(text, &mut prog, &mut codegen);
    for (i, line) in prog.lines.iter().enumerate() {
      let diags: Vec<_> = line
        .diagnostics
        .iter()
        .filter(|d| d.severity == Severity::Error)
        .cloned()
        .collect();
      assert_eq!(diags, vec![], "line {}", i);
    }
    codegen
  }

  struct DummyDevice {
    log: Rc<RefCell<String>>,
    mem: [u8; 65536],
    file: File,
    cursor: (u8, u8),
  }

  #[derive(Debug, Clone)]
  struct File {
    log: Rc<RefCell<String>>,
    pos: usize,
    data: Vec<u8>,
  }

  impl DummyDevice {
    fn new() -> Self {
      let log = Rc::new(RefCell::new(String::new()));
      Self {
        log: Rc::clone(&log),
        mem: [0; 65536],
        file: File {
          log,
          pos: 0,
          data: vec![],
        },
        cursor: (0, 0),
      }
    }
  }

  fn add_log(log: Rc<RefCell<String>>, msg: impl AsRef<str>) {
    log.borrow_mut().push_str(msg.as_ref());
    log.borrow_mut().push('\n');
  }

  impl Device for DummyDevice {
    type File = File;

    fn get_row(&self) -> u8 {
      add_log(self.log.clone(), format!("get row: {}", self.cursor.0));
      self.cursor.0
    }

    fn get_column(&self) -> u8 {
      add_log(self.log.clone(), format!("get column: {}", self.cursor.1));
      self.cursor.1
    }

    fn set_row(&mut self, row: u8) {
      add_log(self.log.clone(), format!("set row to {}", row));
      self.cursor.0 = row;
    }

    fn set_column(&mut self, column: u8) {
      add_log(self.log.clone(), format!("set column to {}", column));
      self.cursor.1 = column;
    }

    fn print(&mut self, str: &[u8]) {
      if str.iter().all(|&b| b < 0x80) {
        add_log(
          self.log.clone(),
          format!("print \"{}\"", unsafe {
            std::str::from_utf8_unchecked(str)
          }),
        );
      } else {
        add_log(self.log.clone(), format!("print {:?}", str));
      }
    }

    fn print_newline(&mut self) {
      add_log(self.log.clone(), "print newline");
    }

    fn flush(&mut self) {
      add_log(self.log.clone(), "flush");
    }

    fn draw_point(&mut self, x: u8, y: u8, mode: DrawMode) {
      add_log(
        self.log.clone(),
        format!("draw point at ({}, {}), {:?}", x, y, mode),
      );
    }

    fn draw_line(&mut self, x1: u8, y1: u8, x2: u8, y2: u8, mode: DrawMode) {
      add_log(
        self.log.clone(),
        format!(
          "draw line from ({}, {}) to ({}, {}), {:?}",
          x1, y1, x2, y2, mode
        ),
      );
    }

    fn draw_box(
      &mut self,
      x1: u8,
      y1: u8,
      x2: u8,
      y2: u8,
      fill: bool,
      mode: DrawMode,
    ) {
      add_log(
        self.log.clone(),
        format!(
          "draw box from ({}, {}) to ({}, {}), fill: {:?}, {:?}",
          x1, y1, x2, y2, fill, mode
        ),
      );
    }

    fn draw_circle(&mut self, x: u8, y: u8, r: u8, fill: bool, mode: DrawMode) {
      add_log(
        self.log.clone(),
        format!(
          "draw circle at ({}, {}), radius: {}, fill: {:?}, {:?}",
          x, y, r, fill, mode
        ),
      );
    }

    fn draw_ellipse(
      &mut self,
      x: u8,
      y: u8,
      rx: u8,
      ry: u8,
      fill: bool,
      mode: DrawMode,
    ) {
      add_log(
        self.log.clone(),
        format!(
          "draw ellipse at ({}, {}), rx: {}, ry: {}, fill: {:?}, {:?}",
          x, y, rx, ry, fill, mode
        ),
      );
    }

    fn clear(&mut self) {
      add_log(self.log.clone(), "clear");
    }

    fn get_byte(&self, addr: u16) -> u8 {
      add_log(
        self.log.clone(),
        format!("peek {}: {}", addr, self.mem[addr as usize]),
      );
      self.mem[addr as usize]
    }

    fn set_byte(&mut self, addr: u16, value: u8) {
      add_log(self.log.clone(), format!("poke {}, {}", addr, value));
      self.mem[addr as usize] = value;
    }

    fn open_file(
      &mut self,
      name: &[u8],
      read: bool,
      write: bool,
      truncate: bool,
    ) -> io::Result<Self::File> {
      add_log(
        self.log.clone(),
        format!(
          "open file \"{}\", read: {:?}, write: {:?}, truncate: {:?}",
          unsafe { std::str::from_utf8_unchecked(name) },
          read,
          write,
          truncate
        ),
      );
      let mut file = self.file.clone();
      if truncate {
        file.data.clear();
      }
      Ok(file)
    }

    fn cls(&mut self) {
      add_log(self.log.clone(), "cls");
    }

    fn exec_asm(&mut self, steps: &mut usize, start_addr: Option<u16>) -> bool {
      add_log(
        self.log.clone(),
        format!("call {:?}, steps: {}", start_addr, steps),
      );
      true
    }

    fn set_screen_mode(&mut self, mode: ScreenMode) {
      add_log(self.log.clone(), format!("set screen mode to {:?}", mode));
    }

    fn set_print_mode(&mut self, mode: PrintMode) {
      add_log(self.log.clone(), format!("set print mode to {:?}", mode));
    }

    fn sleep_unit(&self) -> std::time::Duration {
      std::time::Duration::from_millis(1)
    }

    fn beep(&mut self) {
      add_log(self.log.clone(), "beep");
    }

    fn play_notes(&mut self, notes: &[u8]) {
      add_log(
        self.log.clone(),
        format!("play notes \"{}\"", unsafe {
          std::str::from_utf8_unchecked(notes)
        }),
      );
    }
  }

  impl FileHandle for File {
    fn len(&self) -> io::Result<u64> {
      add_log(
        self.log.clone(),
        format!("get file len: {}", self.data.len()),
      );
      Ok(self.data.len() as u64)
    }

    fn seek(&mut self, pos: u64) -> io::Result<()> {
      add_log(self.log.clone(), format!("seek file: {}", pos));
      if pos > self.data.len() as u64 {
        Err(io::Error::new(io::ErrorKind::Other, "out of range"))
      } else {
        self.pos = pos as usize;
        Ok(())
      }
    }

    fn pos(&self) -> io::Result<u64> {
      add_log(self.log.clone(), format!("get file pos: {}", self.pos));
      Ok(self.pos as u64)
    }

    fn write(&mut self, data: &[u8]) -> io::Result<()> {
      add_log(self.log.clone(), format!("write to file: {:?} ", data));
      if self.pos + data.len() > self.data.len() {
        self.data.resize(self.pos + data.len(), 0);
      }
      self.data[self.pos..self.pos + data.len()].copy_from_slice(data);
      Ok(())
    }

    fn read(&mut self, data: &mut [u8]) -> io::Result<usize> {
      let mut len = data.len();
      if self.pos + len > self.data.len() {
        len = self.data.len() - self.pos;
      }
      data.copy_from_slice(&self.data[self.pos..self.pos + len]);
      add_log(self.log.clone(), format!("read from file: {:?} ", data));
      Ok(len)
    }

    fn close(self) -> io::Result<()> {
      add_log(self.log.clone(), "close file");
      Ok(())
    }
  }

  #[test]
  fn assign() {
    let codegen = compile(
      r#"
10 let a =1:b=a*3+10:dim c(5):c(0)=10:c(1)=20:c(2)=30:c(3)=40:c(4)=50:c(5)=60:
20 c=c(a):print a,b,c,"abC",c(3)+c(0)*10
30 c%=32767+1
    "#
      .trim(),
    );
    let mut device = DummyDevice::new();
    let mut vm = VirtualMachine::new(codegen, &mut device);

    let result = vm.exec(None, usize::MAX);
    assert_eq!(
      result,
      ExecResult::Error {
        location: Location {
          line: 2,
          range: Range::new(6, 13),
        },
        message: "运算结果数值过大，超出了整数的表示范围（-32768~32767），\
          无法赋值给整数变量。运算结果为：32768"
          .to_owned(),
      }
    );

    assert_snapshot!(device.log.borrow());
  }

  #[test]
  fn draw() {
    let codegen = compile(
      r#"
0 x=10:y=20:x1=11:y1=22:x2=33:y2=44:r=6:f=3:m=2
10 draw x,y+1:draw x1,y1,m
20 line x1,y1,x2,y2:line x1,y1,x2,y2,0:
30 box x1,y1,x2,y2:box x,y,x,y%,f:box x1,y1,x2+1,y2,4,m
40 circle x1,y1,r:circle x,y,r,1:circle x,y,r,0,m
50 ellipse x,y,7,3:ellipse x,y,7,3,1:ellipse x,y,7,3,f,m
    "#
      .trim(),
    );
    let mut device = DummyDevice::new();
    let mut vm = VirtualMachine::new(codegen, &mut device);

    let result = vm.exec(None, usize::MAX);
    assert_eq!(result, ExecResult::End);

    assert_snapshot!(device.log.borrow());
  }

  #[test]
  fn nullary_statement() {
    let codegen = compile(
      r#"
10 beep:cls:cont:flash:graph:inkey$:inverse:normal:text
20 :
30 end:print 3
    "#
      .trim(),
    );
    let mut device = DummyDevice::new();
    let mut vm = VirtualMachine::new(codegen, &mut device);

    let result = vm.exec(None, usize::MAX);
    assert_eq!(result, ExecResult::InKey);

    let result = vm.exec(Some(ExecInput::Key(65)), usize::MAX);
    assert_eq!(result, ExecResult::End);

    assert_snapshot!(device.log.borrow());
  }

  #[test]
  fn ppc() {
    let codegen = compile(
      r#"
10 for i=100 to 105:poke i,i-99:next:print peek(101);peek(104):call 1000
20 call -2
    "#
      .trim(),
    );
    let mut device = DummyDevice::new();
    let mut vm = VirtualMachine::new(codegen, &mut device);

    let result = vm.exec(None, usize::MAX);
    assert_eq!(result, ExecResult::End);

    assert_snapshot!(device.log.borrow());
  }

  #[test]
  fn clear() {
    let codegen = compile(
      r#"
10 open "foo" input as1:a=100:a(10)=2:read c1$,c2$:print a;a(10);c1$;c2$:clear
20 open "foo" output as1:read c3$:print a;a(10);c3$:gosub 30
30 data a 1, "a 2" , a 3:clear:pop
    "#
      .trim(),
    );
    let mut device = DummyDevice::new();
    let mut vm = VirtualMachine::new(codegen, &mut device);

    let result = vm.exec(None, usize::MAX);
    assert_eq!(
      result,
      ExecResult::Error {
        location: Location {
          line: 2,
          range: Range::new(31, 34),
        },
        message: "之前没有执行过 GOSUB 语句，POP 语句无法执行".to_owned(),
      }
    );

    assert_snapshot!(device.log.borrow());
  }

  #[test]
  fn clear_loop() {
    let codegen = compile(
      r#"
10 for i=1 to 3:clear:next
    "#
      .trim(),
    );
    let mut device = DummyDevice::new();
    let mut vm = VirtualMachine::new(codegen, &mut device);

    let result = vm.exec(None, usize::MAX);
    assert_eq!(
      result,
      ExecResult::Error {
        location: Location {
          line: 0,
          range: Range::new(22, 26),
        },
        message: "NEXT 语句找不到匹配的 FOR 语句".to_owned(),
      }
    );

    assert_snapshot!(device.log.borrow());
  }

  #[test]
  fn r#fn() {
    let codegen = compile(
      r#"
10 def fn pi(x)=atn(1)*4*x:x=3:print x;:print fn pi(1);:print x;:
20 def fn pi(y)=int(y)*10:print fn pi(3.5);
30 clear:print fn pi(1)
    "#
      .trim(),
    );
    let mut device = DummyDevice::new();
    let mut vm = VirtualMachine::new(codegen, &mut device);

    let result = vm.exec(None, usize::MAX);
    assert_eq!(
      result,
      ExecResult::Error {
        location: Location {
          line: 2,
          range: Range::new(15, 23),
        },
        message: "自定义函数不存在".to_owned(),
      }
    );

    assert_snapshot!(device.log.borrow());
  }

  #[test]
  fn read() {
    let codegen = compile(
      r#"
10 data abc, "123", 1e3
20 data ,,3e
30 read a$(10),b$,c%,d:print a$(10);b$;c%;d
40 restore:read a$,b$,c%,d:print a$;b$;c%;d
50 restore 20:read a$,b$,c%:print a$;b$;c%:read d
    "#
      .trim(),
    );
    let mut device = DummyDevice::new();
    let mut vm = VirtualMachine::new(codegen, &mut device);

    let result = vm.exec(None, usize::MAX);
    assert_eq!(
      result,
      ExecResult::Error {
        location: Location {
          line: 4,
          range: Range::new(48, 49),
        },
        message: "DATA 已经读取结束，没有更多 DATA 可供读取".to_owned(),
      }
    );

    assert_snapshot!(device.log.borrow());
  }

  #[test]
  fn dim() {
    let codegen = compile(
      r#"
10 dim a,a,a$(3):a$(4)=a
    "#
      .trim(),
    );
    let mut device = DummyDevice::new();
    let mut vm = VirtualMachine::new(codegen, &mut device);

    let result = vm.exec(None, usize::MAX);
    assert_eq!(
      result,
      ExecResult::Error {
        location: Location {
          line: 0,
          range: Range::new(20, 21),
        },
        message: "数组下标超出上限。该下标的上限为：3，该下标的值为：4, 取整后的值为：4"
          .to_owned(),
      }
    );

    assert_snapshot!(device.log.borrow());
  }

  #[test]
  fn redefine_array() {
    let codegen = compile(
      r#"
10 dim a,a,a$(3):dim a$(2,7):
    "#
      .trim(),
    );
    let mut device = DummyDevice::new();
    let mut vm = VirtualMachine::new(codegen, &mut device);

    let result = vm.exec(None, usize::MAX);
    assert_eq!(
      result,
      ExecResult::Error {
        location: Location {
          line: 0,
          range: Range::new(21, 23),
        },
        message: "重复定义数组".to_owned(),
      }
    );

    assert_snapshot!(device.log.borrow());
  }

  #[test]
  fn for_loop() {
    let codegen = compile(
      r#"
10 for i=i to i+3:print i;:next i:print i*1e3;
20 for k=10 to 1 step -2:k=k-0.5:print k;:next:print k*1e3;
30 for i=1 to 2 step 0:print i;:i=i+1:next:print i*1e3;
40 for i=1 to 1 step 2:print i;:next:print i*1e3;
50 for i=1 to 10:for i=-10 to -9:print i;:next:next
    "#
      .trim(),
    );
    let mut device = DummyDevice::new();
    let mut vm = VirtualMachine::new(codegen, &mut device);

    let result = vm.exec(None, usize::MAX);
    assert_eq!(
      result,
      ExecResult::Error {
        location: Location {
          line: 4,
          range: Range::new(47, 51),
        },
        message: "NEXT 语句找不到匹配的 FOR 语句".to_owned(),
      }
    );

    assert_snapshot!(device.log.borrow());
  }

  #[test]
  fn jump() {
    let codegen = compile(
      r#"
10 cls:goto 30
20 print inkey$;:return
30 gosub 20
    "#
      .trim(),
    );
    let mut device = DummyDevice::new();
    let mut vm = VirtualMachine::new(codegen, &mut device);

    let result = vm.exec(None, usize::MAX);
    assert_eq!(result, ExecResult::InKey);

    let result = vm.exec(Some(ExecInput::Key(66)), usize::MAX);
    assert_eq!(result, ExecResult::End);

    assert_snapshot!(device.log.borrow());
  }

  #[test]
  fn r#if() {
    let codegen = compile(
      r#"
10 a=1:b=2:if a>=b then print "a";:30 else print "b";:40
20 print "come";:end
30 graph:end
40 if a<>b goto print "GO";:gosub 20:text:else print "go";:gosub 20:inverse
    "#
      .trim(),
    );
    let mut device = DummyDevice::new();
    let mut vm = VirtualMachine::new(codegen, &mut device);

    let result = vm.exec(None, usize::MAX);
    assert_eq!(result, ExecResult::End);

    assert_snapshot!(device.log.borrow());
  }

  #[test]
  fn input() {
    use std::convert::TryFrom;

    let codegen = compile(
      r#"
10 input "foo"; a$, b
20 input c%(2), fn f(y)
30 def fn g(x)=x*x
40 print a$; b; c%(2); fn f(3);
    "#
      .trim(),
    );
    let mut device = DummyDevice::new();
    let mut vm = VirtualMachine::new(codegen, &mut device);

    let result = vm.exec(None, usize::MAX);
    assert_eq!(
      result,
      ExecResult::KeyboardInput {
        prompt: Some("foo".to_owned()),
        fields: vec![KeyboardInputType::String, KeyboardInputType::Real],
      }
    );

    let result = vm.exec(
      Some(ExecInput::KeyboardInput(vec![
        KeyboardInput::String(b"ABc".to_vec().into()),
        KeyboardInput::Real(Mbf5::try_from(3.5f64).unwrap()),
      ])),
      usize::MAX,
    );
    assert_eq!(
      result,
      ExecResult::KeyboardInput {
        prompt: None,
        fields: vec![
          KeyboardInputType::Integer,
          KeyboardInputType::Func {
            name: "F".to_owned(),
            param: "Y".to_owned()
          }
        ],
      }
    );

    let body = vm.compile_fn_body("fn g(y)+2").unwrap();
    let result = vm.exec(
      Some(ExecInput::KeyboardInput(vec![
        KeyboardInput::Integer(37),
        KeyboardInput::Func { body },
      ])),
      usize::MAX,
    );
    assert_eq!(result, ExecResult::End);

    assert_snapshot!(device.log.borrow());
  }

  #[test]
  fn locate() {
    let codegen = compile(
      r#"
10 locate 3:locate ,10:locate 5,1:locate 6
    "#
      .trim(),
    );
    let mut device = DummyDevice::new();
    let mut vm = VirtualMachine::new(codegen, &mut device);

    let result = vm.exec(None, usize::MAX);
    assert_eq!(
      result,
      ExecResult::Error {
        location: Location {
          line: 0,
          range: Range::new(41, 42),
        },
        message: format!("参数超出范围 1~5。运算结果为：6"),
      }
    );

    assert_snapshot!(device.log.borrow());
  }

  #[test]
  fn locate_error_column() {
    let codegen = compile(
      r#"
10 locate 4, 2 0 +1:
    "#
      .trim(),
    );
    let mut device = DummyDevice::new();
    let mut vm = VirtualMachine::new(codegen, &mut device);

    let result = vm.exec(None, usize::MAX);
    assert_eq!(
      result,
      ExecResult::Error {
        location: Location {
          line: 0,
          range: Range::new(13, 19),
        },
        message: format!("参数超出范围 1~20。运算结果为：21"),
      }
    );

    assert_snapshot!(device.log.borrow());
  }

  #[test]
  fn set() {
    let codegen = compile(
      r#"
10 a$="12345"
20 lset a$="":print a$;
30 lset a$="ab":print a$;
40 lset a$="abcdefg":print a$;
45 lset b$(3)="ab":print b$(3);
50 rset a$="":print a$;
60 rset a$="ab":print a$;
70 rset a$="1234567890":print a$;
80 rset b$(3)="ab":print b$(3);
    "#
      .trim(),
    );
    let mut device = DummyDevice::new();
    let mut vm = VirtualMachine::new(codegen, &mut device);

    let result = vm.exec(None, usize::MAX);
    assert_eq!(result, ExecResult::End);

    assert_snapshot!(device.log.borrow());
  }

  #[test]
  fn on() {
    let codegen = compile(
      r#"
10 a=10:gosub 30:a=4:gosub 30:a=7:gosub 30
20 on 2 gosub 40, 50:end
30 on (a>5)+(a<10)*2 goto 40, 50:print "A";:return
40 print "B";:return
50 print "C";:return
    "#
      .trim(),
    );
    let mut device = DummyDevice::new();
    let mut vm = VirtualMachine::new(codegen, &mut device);

    let result = vm.exec(None, usize::MAX);
    assert_eq!(result, ExecResult::End);

    assert_snapshot!(device.log.borrow());
  }

  #[test]
  fn print() {
    let codegen = compile(
      r#"
10 print:print;:print "foo":locate ,10:print spc(2)tab(13);
20 a$="ABCDEFG":a=3:print a$tab(3)a
    "#
      .trim(),
    );
    let mut device = DummyDevice::new();
    let mut vm = VirtualMachine::new(codegen, &mut device);

    let result = vm.exec(None, usize::MAX);
    assert_eq!(result, ExecResult::End);

    assert_snapshot!(device.log.borrow());
  }

  #[test]
  fn run() {
    let codegen = compile(
      r#"
10 open "aaa" for input as 1:a$=a$+"E"
20 if asc(inkey$) > 40 then print a$;:end else run
    "#
      .trim(),
    );
    let mut device = DummyDevice::new();
    let mut vm = VirtualMachine::new(codegen, &mut device);

    let result = vm.exec(None, usize::MAX);
    assert_eq!(result, ExecResult::InKey);

    let result = vm.exec(Some(ExecInput::Key(30)), usize::MAX);
    assert_eq!(result, ExecResult::InKey);

    let result = vm.exec(Some(ExecInput::Key(60)), usize::MAX);
    assert_eq!(result, ExecResult::End);

    assert_snapshot!(device.log.borrow());
  }

  #[test]
  fn swap() {
    let codegen = compile(
      r#"
10 a%=30:a%(2)=555:swap a%,a%(2):print a%;a%(2);
20 a$="abc":b$="ABC-#":swap b$,a$:print a$;b$;
    "#
      .trim(),
    );
    let mut device = DummyDevice::new();
    let mut vm = VirtualMachine::new(codegen, &mut device);

    let result = vm.exec(None, usize::MAX);
    assert_eq!(result, ExecResult::End);

    assert_snapshot!(device.log.borrow());
  }

  #[test]
  fn while_loop() {
    let codegen = compile(
      r#"
10 while a<2:a=a+1:print a;:wend:
20 while a>0:goto 50:wend:
30 gosub 40:wend
40 :while a<2:a=a+1:print a;:return:wend
50 print ".";:a=a-1:wend:print "no"
    "#
      .trim(),
    );
    let mut device = DummyDevice::new();
    let mut vm = VirtualMachine::new(codegen, &mut device);

    let result = vm.exec(None, usize::MAX);
    assert_eq!(
      result,
      ExecResult::Error {
        location: Location {
          line: 2,
          range: Range::new(12, 16),
        },
        message: format!("WEND 语句找不到匹配的 WHILE 语句"),
      }
    );

    assert_snapshot!(device.log.borrow());
  }

  #[test]
  fn sleep() {
    let codegen = compile(
      r#"
10 sleep -100:sleep 0:sleep 200:
    "#
      .trim(),
    );
    let mut device = DummyDevice::new();
    let mut vm = VirtualMachine::new(codegen, &mut device);

    let result = vm.exec(None, usize::MAX);
    assert_eq!(result, ExecResult::Sleep(Duration::from_millis(200)));

    let result = vm.exec(None, usize::MAX);
    assert_eq!(result, ExecResult::End);

    assert_snapshot!(device.log.borrow());
  }

  mod file {
    use super::*;
    use pretty_assertions::assert_eq;
  }

  mod expr {
    use super::*;
    use pretty_assertions::assert_eq;

    #[test]
    fn r#fn() {
      let codegen = compile(
        r#"
10 def fn f(x)=x*x+fn g(x):def fn g(x)=x*2+7
20 x=1:print fn f(10);:print x;
    "#
        .trim(),
      );
      let mut device = DummyDevice::new();
      let mut vm = VirtualMachine::new(codegen, &mut device);

      let result = vm.exec(None, usize::MAX);
      assert_eq!(result, ExecResult::End);

      assert_snapshot!(device.log.borrow());
    }

    #[test]
    fn array() {
      let codegen = compile(
        r#"
10 dim foo(1,2):x%(0)=1::x%(4)=1:x%(6)=0:x%(8)=1:x%(10)=2:
20 foo(0,0)=10:foo(0,1)=100:foo(0, 2)=1000:foo(1,0)=2:foo(1,1)=4:foo(1,2)=6
30 print foo(x%(0),x%(6));
40 print foo(x%(2),x%(8));
50 print foo(x%(4),x%(10));
60 print b(11);
    "#
        .trim(),
      );
      let mut device = DummyDevice::new();
      let mut vm = VirtualMachine::new(codegen, &mut device);

      let result = vm.exec(None, usize::MAX);
      assert_eq!(
        result,
        ExecResult::Error {
          location: Location {
            line: 5,
            range: Range::new(11, 13),
          },
          message: format!(
            "数组下标超出上限。该下标的上限为：10，该下标的值为：11, 取整后的值为：11"
          ),
        }
      );

      assert_snapshot!(device.log.borrow());
    }

    mod sys_func {
      use super::*;
      use pretty_assertions::assert_eq;
    }
  }
}
