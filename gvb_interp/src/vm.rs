use std::num::NonZeroUsize;

use crate::ast::Range;
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
  stack: Vec<StackRecord>,
  value_stack: Vec<TmpValue>,
  interner: StringInterner,
  vars: HashMap<Symbol, Value>,
  arrays: HashMap<Symbol, Array>,
  user_funcs: HashMap<Symbol, UserFunc>,
  memory_man: MemoryManager,
  file_man: FileManager,
  state: ExecState,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum ExecState {
  Normal,
  WaitForKeyboardInput,
  AsmSuspend(),
}

#[derive(Debug, Clone)]
enum StackRecord {
  ForLoop {
    addr: Addr,
    var: Symbol,
    target: Mbf5,
    step: Mbf5,
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
  LValue(TmpLValue),
  String(ByteString),
  Number(Mbf5Accum),
}

#[derive(Debug, Clone)]
enum TmpLValue {
  Array { name: Symbol, offset: usize },
  Var { name: Symbol },
}

/// persistent value
#[derive(Debug, Clone)]
pub enum Value {
  Integer(u16),
  Number(Mbf5),
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
  Number(Vec<Mbf5>),
  String(ByteString),
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
  }
}

#[derive(Debug, Clone)]
pub enum KeyboardInputType {
  String,
  Number,
  Func {
    name: String,
    param: String,
  },
}

#[derive(Debug, Clone)]
pub enum ExecInput {
  KeyboardInput(Vec<KeyboardInput>),
}

#[derive(Debug, Clone)]
pub enum KeyboardInput {
  String(ByteString),
  Number(Mbf5),
  Func {
    name: String,
    param: String,
    body: String,
  }
}

impl VirtualMachine {
  pub fn new(g: CodeGen, memory_man: MemoryManager, file_man: FileManager) -> Self {
    Self {
      data: g.data,
      data_ptr: DatumIndex(0),
      pc: Addr(0),
      code: g.code,
      screen_mode: ScreenMode::Text,
      print_mode: PrintMode::Normal,
      stack: vec![],
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
    if let Some(input) = input {
      match &self.code[self.pc].kind {
        InstrKind::KeyboardInput { fields, .. } => {
          let fields = fields.get();
        }
      }
    }
  }

  fn store_lvalue(&mut self, lvalue: TmpLValue, value: Value) {
  }
}
