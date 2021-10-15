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
  None,
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
    input: ExecInput,
    mut steps: usize,
  ) -> ExecResult {
    match std::mem::replace(&mut self.state, ExecState::Normal) {
      ExecState::Done => return ExecResult::End,
      ExecState::WaitForKey => self.assign_key(input),
      ExecState::WaitForKeyboardInput { lvalues } => {
        self.assign_input(input, lvalues)
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
                  "WRITE 语句只能用于以 OUTPUT 或 APPEND 模式打开的文件，\
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

    macro_rules! do_get_put {
      (
        $op:literal,
        $record_len:ident,
        $fields:ident,
        $file:ident => $body:expr
      ) => {
        let record_loc = self.num_stack.last().unwrap().0.clone();
        let record = self.pop_range(-32768, 32767)? as i16;
        if record == 0 {
          self.state.error(record_loc, "记录序号不能为 0")?;
        }
        let record = (record - 1) as u16;

        let filenum = self.get_filenum(true)?;
        if let Some(file) = &mut self.files[filenum as usize] {
          if let FileMode::Random { record_len, fields } = &file.mode {
            let offset = record as u64 * *record_len as u64;
            self.state.io(
              loc.clone(),
              "设置文件指针",
              file.file.seek(offset),
            )?;

            let $record_len = *record_len;
            let $fields = &fields[..];
            let $file = &mut file.file;
            $body;
          } else {
            self.state.error(
              loc,
              format!(
                "{} 语句只能用于以 RANDOM 模式打开的文件，\
                  但 {} 号文件是以 {} 模式打开的",
                $op,
                filenum + 1,
                file.mode
              ),
            )?;
          }
        } else {
          self.state.error(loc, "未打开文件")?;
        }
      };
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
      InstrKind::NewLine => {
        self.device.newline();
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
        str.drop_0x1f();
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
                "INPUT 语句只能用于以 INPUT 模式打开的文件，\
                  但 {} 号文件是以 {} 模式打开的",
                filenum + 1,
                file.mode
              ),
            )?;
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
        do_get_put!("GET", record_len, fields, file => {
          let mut buf = vec![0; record_len as usize];
          let read_len =
            self
              .state
              .io(loc.clone(), "读取文件", file.read(&mut buf))?;
          if read_len == 0 {
            self.state.error(loc, "不能在文件末尾读取记录")?;
          }
          if read_len < record_len as usize {
            self.state.error(loc, "文件大小不是记录长度的整数倍")?;
          }

          let mut offset = 0;
          for field in fields {
            self.store.store_value(
              field.lvalue.clone(),
              Value::String(
                buf[offset..offset + field.len as usize].to_owned().into(),
              ),
            );
            offset += field.len as usize;
          }
        });
      }
      InstrKind::WriteRecord => {
        do_get_put!("PUT", record_len, fields, file => {
          let mut buf = vec![0u8; record_len as usize];
          let mut offset = 0;
          for field in fields {
            let str = self
              .store
              .load_value(&self.interner, field.lvalue.clone())
              .unwrap_string();
            if str.len() == field.len as usize {
              buf[offset..offset + field.len as usize].clone_from_slice(&str);
            }
            offset += field.len as usize;
          }

          self.state.io(loc.clone(), "写入文件", file.write(&buf))?;
        });
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
        let (arg_loc, value) = self.str_stack.pop().unwrap();
        if value.is_empty() {
          self.state.error(arg_loc, "ASC 函数的参数不能为空字符串")?;
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
        let (arg_loc, value) = self.str_stack.pop().unwrap();
        if value.len() != 2 {
          self.state.error(
            arg_loc,
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
        let (arg_loc, value) = self.str_stack.pop().unwrap();
        if value.len() != 5 {
          self.state.error(
            arg_loc,
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
          Err(RealError::Nan) => unreachable!(),
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
        let len = len.min(value.len());
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
        let (arg_loc, value) = self.num_stack.pop().unwrap();
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
            arg_loc,
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
        let start = pos.min(value.len());
        let end = (start + len).min(value.len());
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
        let len = len.min(value.len());
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
        let (arg_loc, value) = self.num_stack.pop().unwrap();
        match value.sqrt() {
          Ok(value) => {
            self.num_stack.push((loc, value));
            Ok(())
          }
          Err(RealError::Nan) => self.state.error(
            arg_loc,
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
        let (arg_loc, value) = self.num_stack.pop().unwrap();
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
            arg_loc,
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

    if filename.is_empty() {
      self.state.error(name_loc, "文件名不能为空")?;
    } else if let Some(i) = filename.find_byteset(b"/\\") {
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
      self.store.store_value(
        lvalue.clone(),
        Value::String(vec![0u8; len as usize].into()),
      );
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
    self.device.newline();
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
}

pub fn compile_fn(
  input: &str,
  emoji_style: EmojiStyle,
) -> std::result::Result<InputFuncBody, Vec<Diagnostic>> {
  let (mut expr, _) = parse_expr(input);
  let mut codegen = CodeGen::new(emoji_style);
  compile_fn_body(input, &mut expr, &mut codegen);
  if contains_errors(&expr.diagnostics) {
    Err(expr.diagnostics)
  } else {
    Ok(InputFuncBody::new(codegen))
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
  loop {
    let mut byte = [0];
    let len = state.io(loc.clone(), "读取文件", file.read(&mut byte))?;
    if len == 0 {
      break;
    }

    if byte[0] == b'"' {
      quoted = true;
    } else if byte[0] == 0xff || byte[0] == b',' {
      break;
    } else {
      buf.push(byte[0]);
    }

    let mut str_end = false;
    loop {
      let mut byte = [0];
      let len = state.io(loc.clone(), "读取文件", file.read(&mut byte))?;
      if len == 0 {
        if quoted && !str_end {
          state.error(loc.clone(), "读取字符串时遇到未匹配的双引号")?
        }
        break;
      }
      if quoted {
        if str_end {
          if byte[0] == 0xff || byte[0] == b',' {
            break;
          } else {
            state.error(
              loc,
              format!(
                "读取到的数据：\"{}\"，没有以逗号或 U+00FF 字符结尾",
                ByteString::from(buf).to_string_lossy(emoji_style)
              ),
            )?
          }
        } else if byte[0] == b'"' {
          str_end = true;
          continue;
        }
      } else if byte[0] == 0xff || byte[0] == b',' {
        break;
      }
      buf.push(byte[0]);
    }

    break;
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

#[test]
fn test_u32_to_random_number() {
  assert_eq!(
    u32_to_random_number(0x61_00_00_00),
    Mbf5::from([0x7fu8, 0x42, 0, 0, 0])
  );
  assert_eq!(
    u32_to_random_number(0x00_00_00_01),
    Mbf5::from([0x61u8, 0, 0, 0, 0])
  );
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

  fn run_vm(
    mut vm: VirtualMachine<TestDevice>,
    seq: Vec<(ExecResult, ExecInput)>,
  ) {
    let mut input = ExecInput::None;
    for (result, next_input) in seq {
      let r = vm.exec(input, usize::MAX);
      assert_eq!(r, result);
      input = next_input;
    }
  }

  fn run(text: &str, seq: Vec<(ExecResult, ExecInput)>) -> String {
    let codegen = compile(text);
    let mut device = TestDevice::new();
    let vm = VirtualMachine::new(codegen, &mut device);

    run_vm(vm, seq);

    let log = device.log.borrow();
    (*log).clone()
  }

  fn run_with_file(
    text: &str,
    seq: Vec<(ExecResult, ExecInput)>,
    name: &[u8],
    file: File,
  ) -> String {
    let codegen = compile(text);
    let mut device = TestDevice::new().with_file(name.to_vec(), file);
    let vm = VirtualMachine::new(codegen, &mut device);

    run_vm(vm, seq);

    let log = device.log.borrow();
    (*log).clone()
  }

  fn run_with_files(
    text: &str,
    seq: Vec<(ExecResult, ExecInput)>,
    files: Vec<(&[u8], File, Vec<u8>)>,
  ) -> String {
    let codegen = compile(text);
    let mut device = TestDevice::new();
    for (name, file, _) in &files {
      device = device.with_file(name.to_vec(), file.clone());
    }
    let vm = VirtualMachine::new(codegen, &mut device);

    run_vm(vm, seq);

    for (name, _, data) in files {
      assert_eq!(*device.files[name].data.borrow(), data);
    }

    let log = device.log.borrow();
    (*log).clone()
  }

  fn exec_error(
    line: usize,
    start: usize,
    end: usize,
    msg: impl ToString,
  ) -> ExecResult {
    ExecResult::Error {
      location: Location {
        line,
        range: Range::new(start, end),
      },
      message: msg.to_string(),
    }
  }

  struct TestDevice {
    log: Rc<RefCell<String>>,
    mem: [u8; 65536],
    files: HashMap<Vec<u8>, File>,
    cursor: (u8, u8),
  }

  #[derive(Debug, Clone)]
  struct File {
    log: Rc<RefCell<String>>,
    pos: usize,
    data: Rc<RefCell<Vec<u8>>>,
  }

  impl TestDevice {
    fn new() -> Self {
      let log = Rc::new(RefCell::new(String::new()));
      Self {
        log: Rc::clone(&log),
        mem: [0; 65536],
        files: HashMap::default(),
        cursor: (0, 0),
      }
    }

    fn with_file(mut self, name: Vec<u8>, mut file: File) -> Self {
      file.log = self.log.clone();
      self.files.insert(name, file);
      self
    }
  }

  impl File {
    fn new(data: Vec<u8>) -> Self {
      Self {
        log: Rc::new(RefCell::new(String::new())),
        pos: 0,
        data: Rc::new(RefCell::new(data)),
      }
    }
  }

  fn add_log(log: Rc<RefCell<String>>, msg: impl AsRef<str>) {
    log.borrow_mut().push_str(msg.as_ref());
    log.borrow_mut().push('\n');
  }

  impl Device for TestDevice {
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

    fn newline(&mut self) {
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
          "open file {}, read: {:?}, write: {:?}, truncate: {:?}",
          if name.bytes().all(|b| b < 0x80) {
            format!("\"{}\"", unsafe { std::str::from_utf8_unchecked(name) })
          } else {
            format!("{:?}", &name[..])
          },
          read,
          write,
          truncate
        ),
      );
      let file = self.files[name].clone();
      if truncate {
        file.data.borrow_mut().clear();
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
        format!("get file len: {}", self.data.borrow().len()),
      );
      Ok(self.data.borrow().len() as u64)
    }

    fn seek(&mut self, pos: u64) -> io::Result<()> {
      add_log(self.log.clone(), format!("seek file: {}", pos));
      if pos > self.data.borrow().len() as u64 {
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
      if self.pos + data.len() > self.data.borrow().len() {
        self.data.borrow_mut().resize(self.pos + data.len(), 0);
      }
      self.data.borrow_mut()[self.pos..self.pos + data.len()]
        .copy_from_slice(data);
      self.pos += data.len();
      Ok(())
    }

    fn read(&mut self, data: &mut [u8]) -> io::Result<usize> {
      let mut len = data.len();
      if self.pos + len > self.data.borrow().len() {
        len = self.data.borrow().len() - self.pos;
      }
      data[..len]
        .copy_from_slice(&self.data.borrow()[self.pos..self.pos + len]);
      add_log(self.log.clone(), format!("read from file: {:?} ", data));
      self.pos += len;
      Ok(len)
    }

    fn close(self) -> io::Result<()> {
      add_log(self.log.clone(), "close file");
      Ok(())
    }
  }

  #[test]
  fn assign() {
    assert_snapshot!(run(
      r#"
10 let a =1:b=a*3+10:dim c(5):c(0)=10:c(1)=20:c(2)=30:c(3)=40:c(4)=50:c(5)=60:
20 c=c(a):print a,b,c,"abC",c(3)+c(0)*10
30 c%=32767+1
    "#
      .trim(),
      vec![(
        exec_error(
          2,
          6,
          13,
          "运算结果数值过大，超出了整数的表示范围（-32768~32767），\
            无法赋值给整数变量。运算结果为：32768"
        ),
        ExecInput::None,
      )],
    ));
  }

  #[test]
  fn draw() {
    assert_snapshot!(run(
      r#"
0 x=10:y=20:x1=11:y1=22:x2=33:y2=44:r=6:f=3:m=2
10 draw x,y+1:draw x1,y1,m
20 line x1,y1,x2,y2:line x1,y1,x2,y2,0:
30 box x1,y1,x2,y2:box x,y,x,y%,f:box x1,y1,x2+1,y2,4,m
40 circle x1,y1,r:circle x,y,r,1:circle x,y,r,0,m
50 ellipse x,y,7,3:ellipse x,y,7,3,1:ellipse x,y,7,3,f,m
    "#
      .trim(),
      vec![(ExecResult::End, ExecInput::None)],
    ));
  }

  #[test]
  fn nullary_statement() {
    assert_snapshot!(run(
      r#"
10 beep:cls:cont:flash:graph:inkey$:inverse:normal:text
20 :
30 end:print 3
    "#
      .trim(),
      vec![
        (ExecResult::InKey, ExecInput::Key(65)),
        (ExecResult::End, ExecInput::None),
      ],
    ));
  }

  #[test]
  fn ppc() {
    assert_snapshot!(run(
      r#"
10 for i=100 to 105:poke i,i-99:next:print peek(101);peek(104):call 1000
20 call -2
    "#
      .trim(),
      vec![(ExecResult::End, ExecInput::None)],
    ));
  }

  #[test]
  fn clear() {
    assert_snapshot!(run_with_file(
      r#"
10 open "foo" input as1:a=100:a(10)=2:read c1$,c2$:print a;a(10);c1$;c2$:clear
20 open "foo" output as1:read c3$:print a;a(10);c3$:gosub 30
30 data a 1, "a 2" , a 3:clear:pop
    "#
      .trim(),
      vec![(
        exec_error(2, 31, 34, "之前没有执行过 GOSUB 语句，POP 语句无法执行"),
        ExecInput::None,
      )],
      b"foo.DAT",
      File::new(vec![])
    ));
  }

  #[test]
  fn clear_loop() {
    assert_snapshot!(run(
      r#"
10 for i=1 to 3:clear:next
    "#
      .trim(),
      vec![(
        exec_error(0, 22, 26, "NEXT 语句找不到匹配的 FOR 语句"),
        ExecInput::None
      )]
    ));
  }

  #[test]
  fn r#fn() {
    assert_snapshot!(run(
      r#"
10 def fn pi(x)=atn(1)*4*x:x=3:print x;:print fn pi(1);:print x;:
20 def fn pi(y)=int(y)*10:print fn pi(3.5);
30 clear:print fn pi(1)
    "#
      .trim(),
      vec![(exec_error(2, 15, 23, "自定义函数不存在"), ExecInput::None)]
    ));
  }

  #[test]
  fn read() {
    assert_snapshot!(run(
      r#"
10 data abc, "123", 1e3
20 data ,,3e
30 read a$(10),b$,c%,d:print a$(10);b$;c%;d
40 restore:read a$,b$,c%,d:print a$;b$;c%;d
50 restore 20:read a$,b$,c%:print a$;b$;c%:read d
    "#
      .trim(),
      vec![(
        exec_error(4, 48, 49, "DATA 已经读取结束，没有更多 DATA 可供读取"),
        ExecInput::None
      )]
    ));
  }

  #[test]
  fn read_quoted_number() {
    assert_snapshot!(run(
      r#"
10 data "123"
30 read a
    "#
      .trim(),
      vec![(
        exec_error(
          1,
          8,
          9,
          "读取到的数据：\"123\"，是用引号括起来的字符串，无法转换为数值"
        ),
        ExecInput::None
      )]
    ));
  }

  #[test]
  fn dim() {
    assert_snapshot!(run(
      r#"
10 dim a,a,a$(3):a$(4)=a
    "#
      .trim(),
      vec![(
        exec_error(
          0,
          20,
          21,
          "数组下标超出上限。该下标的上限为：3，该下标的值为：4, \
            取整后的值为：4"
        ),
        ExecInput::None
      )]
    ));
  }

  #[test]
  fn redefine_array() {
    assert_snapshot!(run(
      r#"
10 dim a,a,a$(3):dim a$(2,7):
    "#
      .trim(),
      vec![(exec_error(0, 21, 23, "重复定义数组"), ExecInput::None)]
    ));
  }

  #[test]
  fn for_loop() {
    assert_snapshot!(run(
      r#"
10 for i=i to i+3:print i;:next i:print i*1e3;
20 for k=10 to 1 step -2:k=k-0.5:print k;:next:print k*1e3;
30 for i=1 to 2 step 0:print i;:i=i+1:next:print i*1e3;
40 for i=1 to 1 step 2:print i;:next:print i*1e3;
50 for i=1 to 10:for i=-10 to -9:print i;:next:next
    "#
      .trim(),
      vec![(
        exec_error(4, 47, 51, "NEXT 语句找不到匹配的 FOR 语句"),
        ExecInput::None,
      )]
    ));
  }

  #[test]
  fn jump() {
    assert_snapshot!(run(
      r#"
10 cls:goto 30
20 print inkey$;:return
30 gosub 20
    "#
      .trim(),
      vec![
        (ExecResult::InKey, ExecInput::Key(66)),
        (ExecResult::End, ExecInput::None)
      ]
    ));
  }

  #[test]
  fn r#if() {
    assert_snapshot!(run(
      r#"
10 a=1:b=2:if a>=b then print "a";:30 else print "b";:40
20 print "come";:end
30 graph:end
40 if a<>b goto print "GO";:gosub 20:text:else print "go";:gosub 20:inverse
    "#
      .trim(),
      vec![(ExecResult::End, ExecInput::None)]
    ));
  }

  #[test]
  fn input() {
    use std::convert::TryFrom;

    assert_snapshot!(run(
      r#"
10 input "foo"; a$, b
20 input c%(2), fn f(y)
30 def fn g(x)=x*x
40 print a$; b; c%(2); fn f(3);
    "#
      .trim(),
      vec![
        (
          ExecResult::KeyboardInput {
            prompt: Some("foo".to_owned()),
            fields: vec![KeyboardInputType::String, KeyboardInputType::Real],
          },
          ExecInput::KeyboardInput(vec![
            KeyboardInput::String(b"ABc".to_vec().into()),
            KeyboardInput::Real(Mbf5::try_from(3.5f64).unwrap()),
          ]),
        ),
        (
          ExecResult::KeyboardInput {
            prompt: None,
            fields: vec![
              KeyboardInputType::Integer,
              KeyboardInputType::Func {
                name: "F".to_owned(),
                param: "Y".to_owned()
              }
            ],
          },
          {
            let body = compile_fn("fn g(y)+2", EmojiStyle::New).unwrap();
            ExecInput::KeyboardInput(vec![
              KeyboardInput::Integer(37),
              KeyboardInput::Func { body },
            ])
          }
        ),
        (ExecResult::End, ExecInput::None)
      ]
    ));
  }

  #[test]
  fn locate() {
    assert_snapshot!(run(
      r#"
10 locate 3:locate ,10:locate 5,1:locate 6
    "#
      .trim(),
      vec![(
        exec_error(0, 41, 42, "参数超出范围 1~5。运算结果为：6"),
        ExecInput::None
      )]
    ));
  }

  #[test]
  fn locate_error_column() {
    assert_snapshot!(run(
      r#"
10 locate 4, 2 0 +1:
    "#
      .trim(),
      vec![(
        exec_error(0, 13, 19, "参数超出范围 1~20。运算结果为：21"),
        ExecInput::None
      )]
    ));
  }

  #[test]
  fn set() {
    assert_snapshot!(run(
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
      vec![(ExecResult::End, ExecInput::None)]
    ));
  }

  #[test]
  fn on() {
    assert_snapshot!(run(
      r#"
10 a=10:gosub 30:a=4:gosub 30:a=7:gosub 30
20 on 2 gosub 40, 50:end
30 on (a>5)+(a<10)*2 goto 40, 50:print "A";:return
40 print "B";:return
50 print "C";:return
    "#
      .trim(),
      vec![(ExecResult::End, ExecInput::None)]
    ));
  }

  #[test]
  fn print() {
    assert_snapshot!(run(
      r#"
10 print:print;:print "foo":locate ,10:print spc(2)tab(13);
20 a$="ABCDEFG":a=3:print a$tab(3)a
    "#
      .trim(),
      vec![(ExecResult::End, ExecInput::None)]
    ));
  }

  #[test]
  fn run_stmt() {
    assert_snapshot!(run_with_file(
      r#"
10 open "aaa" for input as 1:a$=a$+"E"
20 if asc(inkey$) > 40 then print a$;:end else run
    "#
      .trim(),
      vec![
        (ExecResult::InKey, ExecInput::Key(30)),
        (ExecResult::InKey, ExecInput::Key(60)),
        (ExecResult::End, ExecInput::None)
      ],
      b"aaa.DAT",
      File::new(vec![])
    ));
  }

  #[test]
  fn swap() {
    assert_snapshot!(run(
      r#"
10 a%=30:a%(2)=555:swap a%,a%(2):print a%;a%(2);
20 a$="abc":b$="ABC-#":swap b$,a$:print a$;b$;
    "#
      .trim(),
      vec![(ExecResult::End, ExecInput::None)]
    ));
  }

  #[test]
  fn while_loop() {
    assert_snapshot!(run(
      r#"
10 while a<2:a=a+1:print a;:wend:
20 while a>0:goto 50:wend:
30 gosub 40:wend
40 :while a<2:a=a+1:print a;:return:wend
50 print ".";:a=a-1:wend:print "no"
    "#
      .trim(),
      vec![(
        exec_error(2, 12, 16, "WEND 语句找不到匹配的 WHILE 语句"),
        ExecInput::None
      )]
    ));
  }

  #[test]
  fn write() {
    assert_snapshot!(run(
      r#"
10 a=10:write 1, "foo", "Ab"+chr$(0)+"c": write 1 a*2+1, "a和C",:
    "#
      .trim(),
      vec![(ExecResult::End, ExecInput::None)]
    ));
  }

  #[test]
  fn sleep() {
    assert_snapshot!(run(
      r#"
10 sleep -100:sleep 0:sleep 200:
    "#
      .trim(),
      vec![
        (ExecResult::Sleep(Duration::from_millis(200)), ExecInput::None),
        (ExecResult::End, ExecInput::None)
      ]
    ));
  }

  #[test]
  fn for_replaces_while() {
    assert_snapshot!(run(
      r#"
10 for i=1 to 10:while 1:for i=1 to 2:cls:next i:wend
    "#
      .trim(),
      vec![(
        exec_error(0, 49, 53, "WEND 语句找不到匹配的 WHILE 语句"),
        ExecInput::None
      )]
    ));
  }

  #[test]
  fn for_replaces_sub() {
    assert_snapshot!(run(
      r#"
10 for i=1 to 2:print i;:gosub 20:
20 next i:return
    "#
      .trim(),
      vec![(
        exec_error(1, 10, 16, "之前没有执行过 GOSUB 语句，RETURN 语句无法执行"),
        ExecInput::None
      )]
    ));
  }

  #[test]
  fn while_replaces_for() {
    assert_snapshot!(run(
      r#"
10 while i<2:for j=1 to 10:i=i+1:print i;:wend:next j
    "#
      .trim(),
      vec![(
        exec_error(0, 47, 53, "NEXT 语句找不到匹配的 FOR 语句"),
        ExecInput::None
      )]
    ));
  }

  #[test]
  fn while_replaces_sub() {
    assert_snapshot!(run(
      r#"
10 while i<2:i=i+1:print i;:gosub 20
20 wend:return
    "#
      .trim(),
      vec![(
        exec_error(1, 8, 14, "之前没有执行过 GOSUB 语句，RETURN 语句无法执行"),
        ExecInput::None
      )]
    ));
  }

  #[test]
  fn sub_replaces_while() {
    assert_snapshot!(run(
      r#"
10 gosub 20:wend
20 while 1:return:wend
    "#
      .trim(),
      vec![(
        exec_error(0, 12, 16, "WEND 语句找不到匹配的 WHILE 语句"),
        ExecInput::None
      )]
    ));
  }

  mod file {
    use super::*;

    #[test]
    fn close() {
      assert_snapshot!(run_with_files(
        r#"
10 open "A和B" input as 1:close 1:open "foo.dat" for output as #1
    "#
        .trim(),
        vec![(ExecResult::End, ExecInput::None)],
        vec![
          (
            &[65, 186, 205, 66, 46, 68, 65, 84],
            File::new(vec![]),
            vec![]
          ),
          (b"foo.dat", File::new(vec![]), vec![]),
        ]
      ));
    }

    #[test]
    fn open_empty_filename() {
      assert_snapshot!(run(
        r#"
10 open "" for input as 1
    "#
        .trim(),
        vec![(exec_error(0, 8, 10, "文件名不能为空"), ExecInput::None)]
      ));
    }

    #[test]
    fn reopen_file() {
      assert_snapshot!(run_with_file(
        r#"
10 open "f" for output as 2:open "g" for input as 2:
    "#
        .trim(),
        vec![(exec_error(0, 28, 51, "重复打开 2 号文件"), ExecInput::None)],
        b"f.DAT",
        File::new(vec![]),
      ));
    }

    #[test]
    fn field() {
      assert_snapshot!(run_with_file(
        r#"
10 open "f" random as 2 len=4:field 2, 1 as a$,2 as b$(3),1 as c$
20 print len(a$);asc(a$);len(b$(3));len(c$);:get 2, 2:print a$;b$(3);c$;
    "#
        .trim(),
        vec![(ExecResult::End, ExecInput::None)],
        b"f.DAT",
        File::new(b"ABCDEFGHIJKL".to_vec())
      ));
    }

    #[test]
    fn field_record_too_short() {
      assert_snapshot!(run_with_file(
        r#"
10 open "f" random as 2 len=3:field 2, 1 as a$,2 as b$(3),1 as c$
    "#
        .trim(),
        vec![(
          exec_error(
            0,
            30,
            65,
            "FIELD 语句定义的字段总长度 4 超出了打开文件时所指定的记录长度 3"
          ),
          ExecInput::None
        )],
        b"f.DAT",
        File::new(vec![])
      ));
    }

    #[test]
    fn field_not_open() {
      assert_snapshot!(run(
        r#"
10 field 2, 1 as a$:::
    "#
        .trim(),
        vec![(exec_error(0, 3, 19, "未打开文件"), ExecInput::None)]
      ));
    }

    #[test]
    fn field_mode_error() {
      assert_snapshot!(run_with_file(
        r#"
10 open "f" append as 2:field 2, 1 as a$,2 as b$(3),1 as c$
    "#
        .trim(),
        vec![(
          exec_error(
            0,
            24,
            59,
            "FIELD 语句只能用于以 RANDOM 模式打开的文件，\
              但 2 号文件是以 APPEND 模式打开的"
          ),
          ExecInput::None
        )],
        b"f.DAT",
        File::new(vec![])
      ));
    }

    #[test]
    fn field_not_fill_record() {
      assert_snapshot!(run_with_file(
        r#"
10 open "f" for random as 2 len=4:field 2, 1 as a$,2 as b$(3)
20 get 2, 1:print a$;b$(3)
    "#
        .trim(),
        vec![(ExecResult::End, ExecInput::None)],
        b"f.DAT",
        File::new(b"ABCDEFGHIJKL".to_vec())
      ));
    }

    #[test]
    fn get_file_too_short() {
      assert_snapshot!(run_with_file(
        r#"
10 open "f" for random as 2 len=4:field 2, 1 as a$,2 as b$(3)
20 get 2, 3
    "#
        .trim(),
        vec![(exec_error(1, 3, 11, "文件大小不是记录长度的整数倍"), ExecInput::None,)],
        b"f.DAT",
        File::new(b"ABCDEFGHIJK".to_vec())
      ));
    }

    #[test]
    fn get_at_eof() {
      assert_snapshot!(run_with_file(
        r#"
10 open "f" for random as 2 len=4:field 2, 1 as a$,2 as b$(3)
20 get 2, 4
    "#
        .trim(),
        vec![(exec_error(1, 3, 11, "不能在文件末尾读取记录"), ExecInput::None)],
        b"f.DAT",
        File::new(b"ABCDEFGHIJKL".to_vec())
      ));
    }

    #[test]
    fn get_after_eof() {
      assert_snapshot!(run_with_file(
        r#"
10 open "f" for random as 2 len=4:field 2, 1 as a$,2 as b$(3)
20 get 2, 5
    "#
        .trim(),
        vec![(
          exec_error(1, 3, 11, "设置文件指针时发生错误：out of range"),
          ExecInput::None
        )],
        b"f.DAT",
        File::new(b"ABCDEFGHIJKL".to_vec())
      ));
    }

    #[test]
    fn get_mode_error() {
      assert_snapshot!(run_with_file(
        r#"
10 open "f" for input as 2
20 get 2, 5
    "#
        .trim(),
        vec![(
          exec_error(
            1,
            3,
            11,
            "GET 语句只能用于以 RANDOM 模式打开的文件，\
              但 2 号文件是以 INPUT 模式打开的"
          ),
          ExecInput::None
        )],
        b"f.DAT",
        File::new(b"ABCDEFGHIJKL".to_vec())
      ));
    }

    #[test]
    fn put_mode_error() {
      assert_snapshot!(run_with_file(
        r#"
10 open "f" output as 2
20 put 2, 1
    "#
        .trim(),
        vec![(
          exec_error(
            1,
            3,
            11,
            "PUT 语句只能用于以 RANDOM 模式打开的文件，\
              但 2 号文件是以 OUTPUT 模式打开的"
          ),
          ExecInput::None
        )],
        b"f.DAT",
        File::new(b"ABCDEFGHIJKL".to_vec())
      ));
    }

    #[test]
    fn put() {
      assert_snapshot!(run_with_files(
        r#"
10 open "f.dat" for random as 2 len=3:field 2, 1 as a$,2 as b$(3)
20 lset a$="A":lset b$(3)="BC":put 2, 1
30 lset a$="1":lset b$(3)="23":put 2, 2
40 lset a$="x":lset b$(3)="yz":put 2, 3
50 lset a$="@":lset b$(3)=" .":put 2, 2
60 close 2
    "#
        .trim(),
        vec![(ExecResult::End, ExecInput::None)],
        vec![(b"f.dat", File::new(vec![]), b"ABC@ .xyz".to_vec())]
      ));
    }

    #[test]
    fn put_not_fill_record() {
      assert_snapshot!(run_with_files(
        r#"
10 open "f.dat" for random as 2 len=4:field 2, 1 as a$,2 as b$(3)
20 lset a$=" ":lset b$(3)="0.":put 2, 2
    "#
        .trim(),
        vec![(ExecResult::End, ExecInput::None)],
        vec![(
          b"f.dat",
          File::new(b"ABCDEFGHIJKLMN".to_vec()),
          b"ABCD 0.\0IJKLMN".to_vec()
        )]
      ));
    }

    #[test]
    fn put_after_eof() {
      assert_snapshot!(run_with_file(
        r#"
10 open "f" for random as 2 len=4
20 put 2, 5
    "#
        .trim(),
        vec![(
          exec_error(1, 3, 11, "设置文件指针时发生错误：out of range"),
          ExecInput::None
        )],
        b"f.DAT",
        File::new(b"ABCDEFGHIJKL".to_vec())
      ));
    }

    #[test]
    fn input() {
      assert_snapshot!(run_with_file(
        r#"
10 open "f" input as 3
20 input #3, a$, b$(3)
25 input #3 , c, d$
30 print a$;b$(3);c;d$
    "#
        .trim(),
        vec![(ExecResult::End, ExecInput::None)],
        b"f.DAT",
        File::new(b"AB,\",ab\xff12\"\xff1e3".to_vec())
      ));
    }

    #[test]
    fn input_mode_error() {
      assert_snapshot!(run_with_file(
        r#"
10 open "f" output as 3
20 input #3, a
    "#
        .trim(),
        vec![(
          exec_error(
            1,
            3,
            14,
            "INPUT 语句只能用于以 INPUT 模式打开的文件，\
              但 3 号文件是以 OUTPUT 模式打开的"
          ),
          ExecInput::None
        )],
        b"f.DAT",
        File::new(b"AB,\",ab\xff12\"\xff1e3".to_vec())
      ));
    }

    #[test]
    fn input_invalid_number() {
      assert_snapshot!(run_with_file(
        r#"
10 open "f" input as 3
20 input #3, a
    "#
        .trim(),
        vec![(
          exec_error(1, 13, 14, "读取到的数据：AB，不符合实数的格式"),
          ExecInput::None
        )],
        b"f.DAT",
        File::new(b"AB,\",ab\xff12\"\xff1e3".to_vec())
      ));
    }

    #[test]
    fn input_quoted_number() {
      assert_snapshot!(run_with_file(
        r#"
10 open "f" input as 3
20 input #3, a$,b%
    "#
        .trim(),
        vec![(
          exec_error(
            1,
            16,
            18,
            "读取到的数据：\",ab 12\"，是用引号括起来的字符串，无法转换为数值"
          ),
          ExecInput::None
        )],
        b"f.DAT",
        File::new(b"AB,\",ab 12\"\xff1e3".to_vec())
      ));
    }

    #[test]
    fn input_invalid_quoted_string() {
      assert_snapshot!(run_with_file(
        r#"
10 open "f" input as 3
20 input #3, a$
30 input #3, a$
    "#
        .trim(),
        vec![(
          exec_error(
            2,
            13,
            15,
            "读取到的数据：\",ab 12\"，没有以逗号或 U+00FF 字符结尾"
          ),
          ExecInput::None
        )],
        b"f.DAT",
        File::new(b"AB,\",ab 12\"1e3".to_vec())
      ));
    }

    #[test]
    fn input_unclosed_quoted_string() {
      assert_snapshot!(run_with_file(
        r#"
10 open "f" input as 3
20 input #3, a$
30 input #3, a$
    "#
        .trim(),
        vec![(
          exec_error(2, 13, 15, "读取字符串时遇到未匹配的双引号"),
          ExecInput::None
        )],
        b"f.DAT",
        File::new(b"AB,\",ab\xff12".to_vec())
      ));
    }

    #[test]
    fn write_mode_error() {
      assert_snapshot!(run_with_file(
        r#"
10 open "f" input as 3
20 write #3, "a"
    "#
        .trim(),
        vec![(
          exec_error(
            1,
            13,
            16,
            "WRITE 语句只能用于以 OUTPUT 或 APPEND 模式打开的文件，\
              但 3 号文件是以 INPUT 模式打开的"
          ),
          ExecInput::None
        )],
        b"f.DAT",
        File::new(b"AB,\",ab\xff12".to_vec())
      ));
    }

    #[test]
    fn output_write() {
      assert_snapshot!(run_with_files(
        r#"
10 open "f" for output as 1
20 write #1, 1e3, ". +"
30 write #1,"A和B"
    "#
        .trim(),
        vec![(ExecResult::End, ExecInput::None)],
        vec![(
          b"f.DAT",
          File::new(b"0123456789".to_vec()),
          b"1000,\". +\"\xff\"A\xba\xcdB\"\xff".to_vec()
        )]
      ));
    }

    #[test]
    fn append_write() {
      assert_snapshot!(run_with_files(
        r#"
0 open "f" for output as 2: write #2,123456:close #2
10 open "f" for append as 1
20 write #1, 1e3, ". +"
30 write #1,"A和B"
    "#
        .trim(),
        vec![(ExecResult::End, ExecInput::None)],
        vec![(
          b"f.DAT",
          File::new(b"  abcdefghi".to_vec()),
          b"123456\xff1000,\". +\"\xff\"A\xba\xcdB\"\xff".to_vec()
        )]
      ));
    }

    #[test]
    fn multiple_files() {
      assert_snapshot!(run_with_files(
        r#"
0 open "a" for random as 1 len =2:field 1, 2 asa$(2):
10 open "b.DAT" for input as 2
20 open "c.dat" for output as 3
30 get 1, 3:input #2, b:write #3, a$(2)+str$(b)
40 lset a$(2)=mki$(b):put 1,1
50 input #2, b$:lset a$(2)=b$:put 1,3
    "#
        .trim(),
        vec![(ExecResult::End, ExecInput::None)],
        vec![
          (
            b"a.DAT",
            File::new(b"abcdefgh".to_vec()),
            b"\x7b\x00cd+ gh".to_vec(),
          ),
          (
            b"b.DAT",
            File::new(b"123\xff+ ,%/".to_vec()),
            b"123\xff+ ,%/".to_vec()
          ),
          (
            b"c.dat",
            File::new(b"+-*/".to_vec()),
            b"\"ef123\"\xff".to_vec()
          ),
        ]
      ));
    }
  }

  mod expr {
    use super::*;

    #[test]
    fn r#fn() {
      assert_snapshot!(run(
        r#"
10 def fn f(x)=x*x+fn g(x):def fn g(x)=x*2+7
20 x=1:print fn f(10);:print x;
    "#
        .trim(),
        vec![(ExecResult::End, ExecInput::None)]
      ));
    }

    #[test]
    fn add_overflow() {
      assert_snapshot!(run(
        r#"
10 print 1.70141183e+38+0.00000001e38
    "#
        .trim(),
        vec![(
          exec_error(
            0,
            9,
            37,
            "运算结果数值过大，超出了实数的表示范围。\
            加法运算的两个运算数分别为：1.70141183E+38，1E+30"
          ),
          ExecInput::None
        )]
      ));
    }

    #[test]
    fn sub_overflow() {
      assert_snapshot!(run(
        r#"
10 print -1.70141183e+38-0.00000001e38
    "#
        .trim(),
        vec![(
          exec_error(
            0,
            9,
            38,
            "运算结果数值过大，超出了实数的表示范围。\
            减法运算的两个运算数分别为：-1.70141183E+38，1E+30"
          ),
          ExecInput::None
        )]
      ));
    }

    #[test]
    fn mul_overflow() {
      assert_snapshot!(run(
        r#"
10 print 1e30*1e10
    "#
        .trim(),
        vec![(
          exec_error(
            0,
            9,
            18,
            "运算结果数值过大，超出了实数的表示范围。\
            乘法运算的两个运算数分别为：1E+30，1E+10"
          ),
          ExecInput::None
        )]
      ));
    }

    #[test]
    fn div_overflow() {
      assert_snapshot!(run(
        r#"
10 print 1e30/1e-10
    "#
        .trim(),
        vec![(
          exec_error(
            0,
            9,
            19,
            "运算结果数值过大，超出了实数的表示范围。\
            除法运算的两个运算数分别为：1E+30，1E-10"
          ),
          ExecInput::None
        )]
      ));
    }

    #[test]
    fn div_by_0() {
      assert_snapshot!(run(
        r#"
10 print 1e30/(a-b)
    "#
        .trim(),
        vec![(exec_error(0, 9, 19, "除以 0"), ExecInput::None)]
      ));
    }

    #[test]
    fn pow_overflow() {
      assert_snapshot!(run(
        r#"
10 print 10^40
    "#
        .trim(),
        vec![(
          exec_error(
            0,
            9,
            14,
            "运算结果数值过大，超出了实数的表示范围。底数为：10，指数为：40"
          ),
          ExecInput::None
        )]
      ));
    }

    #[test]
    fn pow_out_of_domain() {
      assert_snapshot!(run(
        r#"
10 print (-3.2)^(-5.2)
    "#
        .trim(),
        vec![(
          exec_error(
            0,
            9,
            22,
            "超出乘方运算的定义域。底数为：-3.2，指数为：-5.2"
          ),
          ExecInput::None
        )]
      ));
    }

    #[test]
    fn logical() {
      assert_snapshot!(run(
        r#"
10 print 3 and 0; 0 and -30; 0 and 13-13; rnd(1) and +50
20 print 3 or 0; 0 or -30; 0 or 13-13; rnd(1) or +50
30 print not 0; not -7
    "#
        .trim(),
        vec![(ExecResult::End, ExecInput::None)]
      ));
    }

    #[test]
    fn gt() {
      assert_snapshot!(run(
        r#"
10 print 10 > 9; 9 > 10; -7 > -7;
20 print "abc" > "abC"; "Abx" > "abC"; "abc" > "abc";
30 print "a" > ""; "ab" > "abc"; "aBc" > "ab"; "abc" > "ab"
    "#
        .trim(),
        vec![(ExecResult::End, ExecInput::None)]
      ));
    }

    #[test]
    fn lt() {
      assert_snapshot!(run(
        r#"
10 print 10 < 9; 9 < 10; -7 < -7;
20 print "abc" < "abC"; "Abx" < "abC"; "abc" < "abc";
30 print "a" < ""; "ab" < "abc"; "aBc" < "ab"; "abc" < "ab"
    "#
        .trim(),
        vec![(ExecResult::End, ExecInput::None)]
      ));
    }

    #[test]
    fn ge() {
      assert_snapshot!(run(
        r#"
10 print 10 >= 9; 9 >= 10; -7 >= -7;
20 print "abc" >= "abC"; "Abx" >= "abC"; "abc" >= "abc";
30 print "a" >= ""; "ab" >= "abc"; "aBc" >= "ab"; "abc" >= "ab"
    "#
        .trim(),
        vec![(ExecResult::End, ExecInput::None)]
      ));
    }

    #[test]
    fn le() {
      assert_snapshot!(run(
        r#"
10 print 10 <= 9; 9 <= 10; -7 <= -7;
20 print "abc" <= "abC"; "Abx" <= "abC"; "abc" <= "abc";
30 print "a" <= ""; "ab" <= "abc"; "aBc" <= "ab"; "abc" <= "ab"
    "#
        .trim(),
        vec![(ExecResult::End, ExecInput::None)]
      ));
    }

    #[test]
    fn eq() {
      assert_snapshot!(run(
        r#"
10 print 10 = 9; 9 = 10; -7 = -7;
20 print "abc" = "abC"; "Abx" = "abC"; "abc" = "abc";
30 print "a" = ""; "ab" = "abc"; "aBc" = "ab"; "abc" = "ab"
    "#
        .trim(),
        vec![(ExecResult::End, ExecInput::None)]
      ));
    }

    #[test]
    fn ne() {
      assert_snapshot!(run(
        r#"
10 print 10 <> 9; 9 <> 10; -7 <> -7;
20 print "abc" <> "abC"; "Abx" <> "abC"; "abc" <> "abc";
30 print "a" <> ""; "ab" <> "abc"; "aBc" <> "ab"; "abc" <> "ab"
    "#
        .trim(),
        vec![(ExecResult::End, ExecInput::None)]
      ));
    }

    #[test]
    fn array() {
      assert_snapshot!(run(
        r#"
10 dim foo(1,2):x%(0)=1::x%(4)=1:x%(6)=0:x%(8)=1:x%(10)=2:
20 foo(0,0)=10:foo(0,1)=100:foo(0, 2)=1000:foo(1,0)=2:foo(1,1)=4:foo(1,2)=6
30 print foo(x%(0),x%(6));
40 print foo(x%(2),x%(8));
50 print foo(x%(4),x%(10));
60 print b(11);
    "#
        .trim(),
        vec![(
          exec_error(
            5,
            11,
            13,
            "数组下标超出上限。该下标的上限为：10，该下标的值为：11, \
              取整后的值为：11"
          ),
          ExecInput::None
        )]
      ));
    }

    mod sys_func {
      use super::*;

      #[test]
      fn abs() {
        assert_snapshot!(run(
          r#"
10 print abs(-7); abs(0); abs(13+1);
    "#
          .trim(),
          vec![(ExecResult::End, ExecInput::None)]
        ));
      }

      #[test]
      fn asc() {
        assert_snapshot!(run(
          r#"
10 print asc("A"); asc("123"); asc("");
    "#
          .trim(),
          vec![(exec_error(0, 35, 37, "ASC 函数的参数不能为空字符串"), ExecInput::None)]
        ));
      }

      #[test]
      fn atn() {
        assert_snapshot!(run(
          r#"
10 print atn(1); atn(-1); atn(0)
    "#
          .trim(),
          vec![(ExecResult::End, ExecInput::None)]
        ));
      }

      #[test]
      fn chr() {
        assert_snapshot!(run(
          r#"
10 print chr$(32); chr$(0); chr$(255); chr$(300)
    "#
          .trim(),
          vec![(
            exec_error(0, 44, 47, "参数超出范围 0~255。运算结果为：300"),
            ExecInput::None
          )]
        ));
      }

      #[test]
      fn cos() {
        assert_snapshot!(run(
          r#"
10 print cos(0); cos(atn(1)*4/3); cos(atn(1)*2)
    "#
          .trim(),
          vec![(ExecResult::End, ExecInput::None)]
        ));
      }

      #[test]
      fn cvi() {
        assert_snapshot!(run(
          r#"
10 print cvi$(chr$(224+8)+chr$(3)); cvi$(chr$(123)+chr$(0)); cvi$("abc");
    "#
          .trim(),
          vec![(
            exec_error(
              0,
              66,
              71,
              "CVI$ 函数的参数字符串长度不等于 2。参数字符串长度为：3"
            ),
            ExecInput::None
          )]
        ));
      }

      #[test]
      fn cvs() {
        assert_snapshot!(run(
          r#"
10 print cvs$(chr$(152)+chr$(53)+chr$(68)+chr$(122)+chr$(0)); cvs$("ab");
    "#
          .trim(),
          vec![(
            exec_error(
              0,
              67,
              71,
              "CVS$ 函数的参数字符串长度不等于 5。参数字符串长度为：2"
            ),
            ExecInput::None
          )]
        ));
      }

      #[test]
      fn eof() {
        assert_snapshot!(run_with_file(
          r#"
10 open "f" input as 3
20 input #3,b$,b$: print eof(3);
30 input #3,c:print eof(3);
40 input #3,d$:print eof(3);
50 input #3,d$:print eof(3);
60 print eof(1);
    "#
          .trim(),
          vec![(exec_error(5, 9, 15, "未打开文件",), ExecInput::None)],
          b"f.DAT",
          File::new(b"AB,\",ab\"\xff12\xff\xff".to_vec())
        ));
      }

      #[test]
      fn eof_mode_error() {
        assert_snapshot!(run_with_file(
          r#"
10 open "f" random as 2
20 print eof(2)
    "#
          .trim(),
          vec![(
            exec_error(
              1,
              9,
              15,
              "EOF 函数只能用于以 INPUT 模式打开的文件，\
                但 2 号文件是以 RANDOM 模式打开的"
            ),
            ExecInput::None
          )],
          b"f.DAT",
          File::new(b"AB,\",ab\"\xff12\xff\xff".to_vec())
        ));
      }

      #[test]
      fn exp() {
        assert_snapshot!(run(
          r#"
10 print exp(1); exp(2); exp(-1);
    "#
          .trim(),
          vec![(ExecResult::End, ExecInput::None)]
        ));
      }

      #[test]
      fn int() {
        assert_snapshot!(run(
          r#"
10 print int(21758); int(174989.546); int(0); int(-147.275); int(-1326790)
    "#
          .trim(),
          vec![(ExecResult::End, ExecInput::None)]
        ));
      }

      #[test]
      fn left() {
        assert_snapshot!(run(
          r#"
10 a$="ABCD":print left$(a$,2); left$(a$,1); left$(a$,10); left$("",3); left$(a$,0);
    "#
          .trim(),
          vec![(exec_error(0, 81, 82, "参数超出范围 1~255。运算结果为：0"), ExecInput::None)]
        ));
      }

      #[test]
      fn len() {
        assert_snapshot!(run(
          r#"
10 print len("abcd"); len("A和B 从"); len("");
    "#
          .trim(),
          vec![(ExecResult::End, ExecInput::None)]
        ));
      }

      #[test]
      fn lof() {
        assert_snapshot!(run_with_file(
          r#"
10 open "f" random as 2 len=3:field 2, 2 as a$, 1 as b$
20 print lof(2);:put 2,3
30 print lof(2);:put 2,4
40 print lof(2);
50 print lof(1);
    "#
          .trim(),
          vec![(exec_error(4, 9, 15, "未打开文件",), ExecInput::None)],
          b"f.DAT",
          File::new(b"0123456789".to_vec())
        ));
      }

      #[test]
      fn lof_mode_error() {
        assert_snapshot!(run_with_file(
          r#"
10 open "f" append as 2
20 print lof(2)
    "#
          .trim(),
          vec![(
            exec_error(
              1,
              9,
              15,
              "LOF 函数只能用于以 RANDOM 模式打开的文件，\
                但 2 号文件是以 APPEND 模式打开的"
            ),
            ExecInput::None
          )],
          b"f.DAT",
          File::new(b"AB,\",ab\"\xff12\xff\xff".to_vec())
        ));
      }

      #[test]
      fn log() {
        assert_snapshot!(run(
          r#"
10 print log(1); log(exp(1)); log(2); log(0)
    "#
          .trim(),
          vec![(
            exec_error(
              0,
              38,
              44,
              "运算结果数值过大，超出实数的表示范围。参数值是：0"
            ),
            ExecInput::None
          )]
        ));
      }

      #[test]
      fn log_neg() {
        assert_snapshot!(run(
          r#"
10 print log(-2);
    "#
          .trim(),
          vec![(
            exec_error(0, 13, 15, "超出 LOG 函数的定义域。参数值是：-2"),
            ExecInput::None
          )]
        ));
      }

      #[test]
      fn mid() {
        assert_snapshot!(run(
          r#"
10 a$="ABCDEF":print mid$(a$,3,2); mid$(a$,3,7); mid$(a$,3,0); mid$(a$,7,2)
20 print mid$(a$,3); mid$(a$,10); mid$(a$,0)
    "#
          .trim(),
          vec![(
            exec_error(1, 42, 43, "参数超出范围 1~255。运算结果为：0"),
            ExecInput::None
          )]
        ));
      }

      #[test]
      fn mki() {
        assert_snapshot!(run(
          r#"
10 print mki$(1000); mki$(123); asc(mki$(-32768)); asc(mid$(mki$(-32768),2)); mki$(32768);
    "#
          .trim(),
          vec![(
            exec_error(
              0,
              83,
              88,
              "参数超出范围 -32768~32767。运算结果为：32768"
            ),
            ExecInput::None
          )]
        ));
      }

      #[test]
      fn mks() {
        assert_snapshot!(run(
          r#"
10 print mks$(1); mks$(-1); mks$(-11879546);
    "#
          .trim(),
          vec![(ExecResult::End, ExecInput::None)]
        ));
      }

      #[test]
      fn peek() {
        assert_snapshot!(run(
          r#"
10 poke 12345, 78
20 print peek(12345); peek(12345-65536)
    "#
          .trim(),
          vec![(ExecResult::End, ExecInput::None)]
        ));
      }

      #[test]
      fn pos() {
        assert_snapshot!(run(
          r#"
10 locate ,7: print pos(-2749);
20 locate ,13:print pos(14);
    "#
          .trim(),
          vec![(ExecResult::End, ExecInput::None)]
        ));
      }

      #[test]
      fn right() {
        assert_snapshot!(run(
          r#"
10 a$="ABCD":print right$(a$,2); right$(a$,1); right$(a$,10); right$("",3); right$(a$,0);
    "#
          .trim(),
          vec![(exec_error(0, 86, 87, "参数超出范围 1~255。运算结果为：0"), ExecInput::None)]
        ));
      }

      #[test]
      fn rnd() {
        assert_snapshot!(run(
          r#"
10 print rnd(-300); rnd(1); rnd(1); rnd(0); rnd(0)
20 print rnd(-300); rnd(1); rnd(1);
    "#
          .trim(),
          vec![(ExecResult::End, ExecInput::None)]
        ));
      }

      #[test]
      fn sgn() {
        assert_snapshot!(run(
          r#"
10 print sgn(1247); sgn(0); sgn(-12479.14)
    "#
          .trim(),
          vec![(ExecResult::End, ExecInput::None)]
        ));
      }

      #[test]
      fn sin() {
        assert_snapshot!(run(
          r#"
10 print sin(atn(1)*2); sin(atn(1)*2/3); sin(0); sin(-atn(1)*2);
    "#
          .trim(),
          vec![(ExecResult::End, ExecInput::None)]
        ));
      }

      #[test]
      fn sqr() {
        assert_snapshot!(run(
          r#"
10 print sqr(16); sqr(10); sqr(0); sqr(-52)
    "#
          .trim(),
          vec![(
            exec_error(0, 39, 42, "超出 SQR 函数的定义域。参数值是：-52",),
            ExecInput::None
          )]
        ));
      }

      #[test]
      fn str() {
        assert_snapshot!(run(
          r#"
10 print str$(-0.003765e-1); str$(0); str$(1741579.23);
    "#
          .trim(),
          vec![(ExecResult::End, ExecInput::None)]
        ));
      }

      #[test]
      fn tan() {
        assert_snapshot!(run(
          r#"
10 print tan(0); tan(1); tan(-1); tan(atn(1)*2);
    "#
          .trim(),
          vec![(ExecResult::End, ExecInput::None)]
        ));
      }

      #[test]
      fn val() {
        assert_snapshot!(run(
          r#"
10 print val("1e3"); val(""); val("abc"); val("- 1.52f%U7"); val("1 .  2 e + 2K");
    "#
          .trim(),
          vec![(
            ExecResult::End,
            ExecInput::None
          )]
        ));
      }
    }
  }
}
