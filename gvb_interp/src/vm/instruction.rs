use std::num::NonZeroUsize;

use super::{ByteString, Symbol};
use crate::{ast::{FileMode, Range, SysFuncKind}, util::mbf5::Mbf5};

#[derive(Debug, Clone)]
pub struct Instr {
  pub range: Range,
  pub kind: InstrKind,
}

#[derive(Debug, Clone, Copy)]
pub struct Addr(pub(crate) usize);

pub const DUMMY_ADDR: Addr = Addr(0);

#[derive(Debug, Clone, Copy)]
pub struct DatumIndex(pub(crate) usize);

pub const FISRT_DATUM_INDEX: DatumIndex = DatumIndex(0);

#[derive(Debug, Clone)]
pub enum InstrKind {
  DefFn {
    name: Symbol,
    param: Symbol,
    end: Addr,
  },
  DimArray {
    name: Symbol,
    dimensions: NonZeroUsize,
  },
  PushLValue {
    name: Symbol,
    dimensions: usize,
  },
  PushFnLValue {
    name: Symbol,
    param: Symbol,
  },
  SetRecordFields {
    fields: NonZeroUsize,
  },
  ForLoop {
    name: Symbol,
    has_step: bool,
  },
  NextFor {
    name: Option<Symbol>,
  },
  GoSub(Addr),
  GoTo(Addr),
  JumpIfZero(Addr),
  CallFn(Symbol),
  ReturnFn,
  Switch(NonZeroUsize),
  RestoreDataPtr(DatumIndex),
  Return,
  Pop,
  PopValue,
  PushNum(Mbf5),
  PushVar(Symbol),
  PushStr(ByteString),
  PushInKey,
  PushIndex {
    name: Symbol,
    dimensions: NonZeroUsize,
  },
  Not,
  Neg,
  Eq,
  Ne,
  Gt,
  Lt,
  Ge,
  Le,
  Add,
  Sub,
  Mul,
  Div,
  Pow,
  And,
  Or,
  SysFuncCall {
    kind: SysFuncKind,
    arity: usize,
  },
  PrintNewLine,
  PrintSpc,
  PrintTab,
  PrintValue,
  SetRow,
  SetColumn,
  Write {
    to_file: bool,
  },
  WriteEnd {
    to_file: bool,
  },
  KeyboardInput {
    prompt: Option<String>,
    fields: NonZeroUsize,
  },
  FileInput {
    fields: NonZeroUsize,
  },
  ReadData,
  OpenFile {
    mode: FileMode,
    has_len: bool,
  },
  Beep,
  DrawBox {
    has_fill: bool,
    has_mode: bool,
  },
  Call,
  DrawCircle {
    has_fill: bool,
    has_mode: bool,
  },
  Clear,
  CloseFile,
  Cls,
  NoOp,
  DrawPoint {
    has_mode: bool,
  },
  DrawEllipse {
    has_fill: bool,
    has_mode: bool,
  },
  End,
  ReadRecord,
  WriteRecord,
  Assign,
  DrawLine {
    has_mode: bool,
  },
  AlignedAssign(Alignment),
  SetTrace(bool),
  SetScreenMode(ScreenMode),
  PlayNotes,
  Poke,
  Swap,
  Restart,
  SetPrintMode(PrintMode),
  Wend,
  WhileLoop {
    start: Addr,
    end: Addr,
  },
  Sleep,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScreenMode {
  Text,
  Graph
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Alignment {
  Left,
  Right,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PrintMode {
  Normal,
  Inverse,
  Flash,
}
