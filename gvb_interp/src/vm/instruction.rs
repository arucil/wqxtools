#[cfg(test)]
use crate::machine::EmojiVersion;
use std::fmt::{self, Debug, Formatter};
use std::num::NonZeroUsize;
#[cfg(test)]
use string_interner::StringInterner;

use super::{ByteString, Symbol};
use crate::{
  ast::{FileMode, Range, SysFuncKind},
  util::mbf5::Mbf5,
  HashMap,
};

#[derive(Clone, PartialEq, Eq)]
pub struct Location {
  pub line: usize,
  pub range: Range,
}

#[derive(Clone)]
pub struct Instr {
  pub loc: Location,
  pub kind: InstrKind,
}

#[derive(Debug, Clone, Copy)]
pub struct Addr(pub(crate) usize);

pub const DUMMY_ADDR: Addr = Addr(0);

#[derive(Debug, Clone, Copy)]
pub struct DatumIndex(pub(crate) usize);

pub const FISRT_DATUM_INDEX: DatumIndex = DatumIndex(0);

#[derive(Clone)]
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
  PushVarLValue {
    name: Symbol,
  },
  PushIndexLValue {
    name: Symbol,
    dimensions: NonZeroUsize,
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
  PopNum,
  PopStr,
  PushNum(Mbf5),
  PushVar(Symbol),
  // TODO index of string table
  PushStr(ByteString),
  PushInKey,
  PushIndex {
    name: Symbol,
    dimensions: NonZeroUsize,
  },
  Not,
  Neg,
  CmpNum(CmpKind),
  CmpStr(CmpKind),
  Add,
  Sub,
  Mul,
  Div,
  Pow,
  Concat,
  And,
  Or,
  SysFuncCall {
    kind: SysFuncKind,
    arity: NonZeroUsize,
  },
  NewLine,
  PrintSpc,
  PrintTab,
  PrintNum,
  PrintStr,
  Flush,
  SetRow,
  SetColumn,
  WriteNum {
    to_file: bool,
    end: bool,
  },
  WriteStr {
    to_file: bool,
    end: bool,
  },
  KeyboardInput {
    has_prompt: bool,
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
  AssignNum,
  AssignStr,
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
  Fputc,
  Fread,
  Fwrite,
  Fseek,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScreenMode {
  Text,
  Graph,
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CmpKind {
  Eq,
  Ne,
  Gt,
  Lt,
  Ge,
  Le,
}

impl InstrKind {
  pub fn map_symbol(self, sym_map: &HashMap<Symbol, Symbol>) -> Self {
    match self {
      Self::DefFn { name, param, end } => Self::DefFn {
        name: sym_map[&name],
        param: sym_map[&param],
        end,
      },
      Self::DimArray { name, dimensions } => Self::DimArray {
        name: sym_map[&name],
        dimensions,
      },
      Self::PushVarLValue { name } => Self::PushVarLValue {
        name: sym_map[&name],
      },
      Self::PushIndexLValue { name, dimensions } => Self::PushIndexLValue {
        name: sym_map[&name],
        dimensions,
      },
      Self::PushFnLValue { name, param } => Self::PushFnLValue {
        name: sym_map[&name],
        param: sym_map[&param],
      },
      Self::ForLoop { name, has_step } => Self::ForLoop {
        name: sym_map[&name],
        has_step,
      },
      Self::NextFor { name } => Self::NextFor {
        name: name.map(|name| sym_map[&name]),
      },
      Self::CallFn(name) => Self::CallFn(sym_map[&name]),
      Self::PushVar(name) => Self::PushVar(sym_map[&name]),
      Self::PushIndex { name, dimensions } => Self::PushIndex {
        name: sym_map[&name],
        dimensions,
      },
      _ => self,
    }
  }
}

impl CmpKind {
  pub fn cmp<T: PartialOrd>(self, lhs: T, rhs: T) -> bool {
    match self {
      Self::Eq => lhs == rhs,
      Self::Ne => lhs != rhs,
      Self::Gt => lhs > rhs,
      Self::Lt => lhs < rhs,
      Self::Ge => lhs >= rhs,
      Self::Le => lhs <= rhs,
    }
  }
}

#[cfg(test)]
impl Instr {
  pub fn print(
    &self,
    interner: &StringInterner,
    emoji_version: EmojiVersion,
  ) -> String {
    format!(
      "{:<10?}{}",
      self.loc,
      self.kind.print(interner, emoji_version)
    )
  }
}

#[cfg(test)]
impl InstrKind {
  pub fn print(
    &self,
    interner: &StringInterner,
    emoji_version: EmojiVersion,
  ) -> String {
    macro_rules! sym {
      ($sym:ident) => {
        interner.resolve(*$sym).unwrap()
      };
    }
    match self {
      Self::DefFn { name, param, end } => {
        format!("def fn {}({}), end: {}", sym!(name), sym!(param), end.0)
      }
      Self::DimArray { name, dimensions } => {
        format!("dim array {}, dimension: {}", sym!(name), dimensions)
      }
      Self::PushVarLValue { name } => {
        format!("push var lvalue {}", sym!(name))
      }
      Self::PushIndexLValue { name, dimensions } => {
        format!(
          "push index lvalue {}, dimensions: {}",
          sym!(name),
          dimensions
        )
      }
      Self::PushFnLValue { name, param } => {
        format!("push lvalue FN {}({})", sym!(name), sym!(param))
      }
      Self::SetRecordFields { fields } => {
        format!("set record fields, num fields: {}", fields)
      }
      Self::ForLoop { name, has_step } => format!(
        "start for loop, var: {}, has_step: {}",
        sym!(name),
        has_step
      ),
      Self::NextFor { name } => format!(
        "next for loop, var: {}",
        match name {
          Some(name) => format!("Some({})", sym!(name)),
          None => format!("None"),
        }
      ),
      Self::GoSub(addr) => format!("gosub {}", addr.0),
      Self::GoTo(addr) => format!("goto {}", addr.0),
      Self::JumpIfZero(addr) => format!("if zero goto {}", addr.0),
      Self::CallFn(name) => format!("call FN {}", sym!(name)),
      Self::ReturnFn => format!("return from FN"),
      Self::Switch(branches) => format!("switch, num branches: {}", branches),
      Self::RestoreDataPtr(ptr) => format!("restore data ptr: {}", ptr.0),
      Self::Return => format!("return"),
      Self::Pop => format!("pop sub"),
      Self::PopNum => format!("pop num"),
      Self::PopStr => format!("pop str"),
      Self::PushNum(num) => format!("push number {}", num),
      Self::PushVar(name) => format!("push var {}", sym!(name)),
      Self::PushStr(str) => {
        format!("push string \"{}\"", str.to_string_lossy(emoji_version))
      }
      Self::PushInKey => format!("push inkey"),
      Self::PushIndex { name, dimensions } => {
        format!("push index {}, dimensions: {}", sym!(name), dimensions)
      }
      Self::Not => format!("not"),
      Self::Neg => format!("neg"),
      Self::CmpStr(op) => format!("str {:?}", op),
      Self::CmpNum(op) => format!("num {:?}", op),
      Self::Add => format!("add"),
      Self::Sub => format!("sub"),
      Self::Mul => format!("mul"),
      Self::Div => format!("div"),
      Self::Pow => format!("pow"),
      Self::And => format!("and"),
      Self::Or => format!("or"),
      Self::Concat => format!("concat"),
      Self::SysFuncCall { kind, arity } => {
        format!("call sys func {:?}, arity: {}", kind, arity)
      }
      Self::NewLine => format!("newline"),
      Self::PrintSpc => format!("print SPC"),
      Self::PrintTab => format!("print TAB"),
      Self::PrintNum => format!("print num"),
      Self::PrintStr => format!("print str"),
      Self::Flush => format!("flush"),
      Self::SetRow => format!("set row"),
      Self::SetColumn => format!("set column"),
      Self::WriteNum { to_file, end } => {
        format!(
          "write num {}to {}",
          if *end { "end " } else { "" },
          if *to_file { "file" } else { "screen" }
        )
      }
      Self::WriteStr { to_file, end } => {
        format!(
          "write str {}to {}",
          if *end { "end " } else { "" },
          if *to_file { "file" } else { "screen" }
        )
      }
      Self::KeyboardInput { has_prompt, fields } => {
        format!(
          "keyboard input, has_prompt: {:?}, num fields: {}",
          has_prompt, fields
        )
      }
      Self::FileInput { fields } => {
        format!("file input, num fields: {}", fields)
      }
      Self::ReadData => format!("read data"),
      Self::OpenFile { mode, has_len } => {
        format!("open file, mode: {:?}, has_len: {}", mode, has_len)
      }
      Self::Beep => format!("beep"),
      Self::DrawBox { has_fill, has_mode } => {
        format!("draw box, has_fill: {}, has_mode: {}", has_fill, has_mode)
      }
      Self::Call => format!("call asm"),
      Self::DrawCircle { has_fill, has_mode } => format!(
        "draw circle, has_fill: {}, has_mode: {}",
        has_fill, has_mode
      ),
      Self::Clear => format!("clear"),
      Self::CloseFile => format!("close file"),
      Self::Cls => format!("cls"),
      Self::NoOp => format!("no op"),
      Self::DrawPoint { has_mode } => {
        format!("draw point, has_mode: {}", has_mode)
      }
      Self::DrawEllipse { has_fill, has_mode } => format!(
        "draw ellipse, has_fill: {}, has_mode: {}",
        has_fill, has_mode
      ),
      Self::End => format!("end"),
      Self::ReadRecord => format!("read record"),
      Self::WriteRecord => format!("write record"),
      Self::AssignNum => format!("assign num"),
      Self::AssignStr => format!("assign str"),
      Self::DrawLine { has_mode } => {
        format!("draw line, has_mode: {}", has_mode)
      }
      Self::AlignedAssign(align) => {
        format!("aligned assign, align: {:?}", align)
      }
      Self::SetTrace(mode) => format!("set trace mode: {}", mode),
      Self::SetScreenMode(mode) => format!("set screen mode: {:?}", mode),
      Self::PlayNotes => format!("play notes"),
      Self::Poke => format!("poke"),
      Self::Swap => format!("swap"),
      Self::Restart => format!("restart"),
      Self::SetPrintMode(mode) => format!("set print mode: {:?}", mode),
      Self::Wend => format!("wend"),
      Self::WhileLoop { start, end } => format!(
        "start while loop, cond start addr: {}, end addr: {}",
        start.0, end.0
      ),
      Self::Sleep => format!("sleep"),
      Self::Fputc => format!("fputc"),
      Self::Fread => format!("fread"),
      Self::Fwrite => format!("fwrite"),
      Self::Fseek => format!("fseek"),
    }
  }
}

impl Debug for Location {
  fn fmt(&self, f: &mut Formatter) -> fmt::Result {
    f.pad(&format!("{}:{:?}", self.line, self.range))
  }
}
