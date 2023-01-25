use num_derive::FromPrimitive;
use num_traits::FromPrimitive;
use std::{
  fmt::{self, Debug, Formatter},
  str::FromStr,
};

use super::SysFuncKind;

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum TokenKind {
  Ident,
  Label,
  Float,
  String,
  Punc(Punc),
  Keyword(Keyword),
  SysFunc(SysFuncKind),
  Eof,
}

#[derive(Clone, Copy, PartialEq, Eq, FromPrimitive)]
pub enum Keyword {
  Auto,
  Beep,
  Box,
  Call,
  Circle,
  Clear,
  Close,
  Cls,
  Cont,
  Copy,
  Data,
  Def,
  Del,
  Dim,
  Draw,
  Edit,
  Ellipse,
  End,
  Field,
  Files,
  Flash,
  For,
  Get,
  Gosub,
  Goto,
  Graph,
  If,
  Inkey,
  Input,
  Inverse,
  Kill,
  Let,
  Line,
  List,
  Load,
  Locate,
  Lset,
  New,
  Next,
  Normal,
  Notrace,
  On,
  Open,
  Play,
  Poke,
  Pop,
  Print,
  Put,
  Read,
  Rem,
  Rename,
  Restore,
  Return,
  Rset,
  Run,
  Save,
  Stop,
  Swap,
  System,
  Text,
  Trace,
  Wend,
  While,
  Write,

  Then,
  Else,
  To,
  Step,
  Fn,
  And,
  Or,
  Not,
  At,

  Sleep,
  Fputc,
  Fread,
  Fwrite,
  Fseek,
  DebugPrint,
}

#[derive(Clone, Copy, PartialEq, Eq, FromPrimitive)]
pub enum Punc {
  Eq,
  Lt,
  Gt,
  Plus,
  Minus,
  Times,
  Slash,
  Caret,
  Colon,
  LParen,
  RParen,
  Semicolon,
  Comma,
  Hash,
}

static STR_TO_KEYWORD: phf::Map<&str, Keyword> = phf::phf_map! {
  "auto" => Keyword::Auto,
  "beep" => Keyword::Beep,
  "box" => Keyword::Box,
  "call" => Keyword::Call,
  "circle" => Keyword::Circle,
  "clear" => Keyword::Clear,
  "close" => Keyword::Close,
  "cls" => Keyword::Cls,
  "cont" => Keyword::Cont,
  "copy" => Keyword::Copy,
  "data" => Keyword::Data,
  "def" => Keyword::Def,
  "del" => Keyword::Del,
  "dim" => Keyword::Dim,
  "draw" => Keyword::Draw,
  "edit" => Keyword::Edit,
  "ellipse" => Keyword::Ellipse,
  "end" => Keyword::End,
  "field" => Keyword::Field,
  "files" => Keyword::Files,
  "flash" => Keyword::Flash,
  "for" => Keyword::For,
  "get" => Keyword::Get,
  "gosub" => Keyword::Gosub,
  "goto" => Keyword::Goto,
  "graph" => Keyword::Graph,
  "if" => Keyword::If,
  "inkey$" => Keyword::Inkey,
  "input" => Keyword::Input,
  "inverse" => Keyword::Inverse,
  "kill" => Keyword::Kill,
  "let" => Keyword::Let,
  "line" => Keyword::Line,
  "list" => Keyword::List,
  "load" => Keyword::Load,
  "locate" => Keyword::Locate,
  "lset" => Keyword::Lset,
  "new" => Keyword::New,
  "next" => Keyword::Next,
  "normal" => Keyword::Normal,
  "notrace" => Keyword::Notrace,
  "on" => Keyword::On,
  "open" => Keyword::Open,
  "play" => Keyword::Play,
  "poke" => Keyword::Poke,
  "pop" => Keyword::Pop,
  "print" => Keyword::Print,
  "put" => Keyword::Put,
  "read" => Keyword::Read,
  "rem" => Keyword::Rem,
  "rename" => Keyword::Rename,
  "restore" => Keyword::Restore,
  "return" => Keyword::Return,
  "rset" => Keyword::Rset,
  "run" => Keyword::Run,
  "save" => Keyword::Save,
  "stop" => Keyword::Stop,
  "swap" => Keyword::Swap,
  "system" => Keyword::System,
  "text" => Keyword::Text,
  "trace" => Keyword::Trace,
  "wend" => Keyword::Wend,
  "while" => Keyword::While,
  "write" => Keyword::Write,

  "then" => Keyword::Then,
  "else" => Keyword::Else,
  "to" => Keyword::To,
  "step" => Keyword::Step,
  "fn" => Keyword::Fn,
  "and" => Keyword::And,
  "or" => Keyword::Or,
  "not" => Keyword::Not,

  "sleep" => Keyword::Sleep,
  "fputc" => Keyword::Fputc,
  "fread" => Keyword::Fread,
  "fwrite" => Keyword::Fwrite,
  "fseek" => Keyword::Fseek,
  "debugprint" => Keyword::DebugPrint,
};

impl FromStr for Keyword {
  type Err = ();
  fn from_str(s: &str) -> Result<Self, ()> {
    STR_TO_KEYWORD.get(s).ok_or(()).copied()
  }
}

impl From<u8> for Punc {
  fn from(c: u8) -> Self {
    match c {
      b'=' => Self::Eq,
      b'<' => Self::Lt,
      b'>' => Self::Gt,
      b'+' => Self::Plus,
      b'-' => Self::Minus,
      b'*' => Self::Times,
      b'/' => Self::Slash,
      b'^' => Self::Caret,
      b':' => Self::Colon,
      b'(' => Self::LParen,
      b')' => Self::RParen,
      b';' => Self::Semicolon,
      b',' => Self::Comma,
      b'#' => Self::Hash,
      _ => panic!("invalid char {c}"),
    }
  }
}

impl Debug for Keyword {
  fn fmt(&self, f: &mut Formatter) -> fmt::Result {
    use Keyword::*;
    let kw = match self {
      Auto => "AUTO",
      Beep => "BEEP",
      Box => "BOX",
      Call => "CALL",
      Circle => "CIRCLE",
      Clear => "CLEAR",
      Close => "CLOSE",
      Cls => "CLS",
      Cont => "CONT",
      Copy => "COPY",
      Data => "DATA",
      Def => "DEF",
      Del => "DEL",
      Dim => "DIM",
      Draw => "DRAW",
      Edit => "EDIT",
      Ellipse => "ELLIPSE",
      End => "END",
      Field => "FIELD",
      Files => "FILES",
      Flash => "FLASH",
      For => "FOR",
      Get => "GET",
      Gosub => "GOSUB",
      Goto => "GOTO",
      Graph => "GRAPH",
      If => "IF",
      Inkey => "INKEY$",
      Input => "INPUT",
      Inverse => "INVERSE",
      Kill => "KILL",
      Let => "LET",
      Line => "LINE",
      List => "LIST",
      Load => "LOAD",
      Locate => "LOCATE",
      Lset => "LSET",
      New => "NEW",
      Next => "NEXT",
      Normal => "NORMAL",
      Notrace => "NOTRACE",
      On => "On",
      Open => "OPEN",
      Play => "PLAY",
      Poke => "POKE",
      Pop => "POP",
      Print => "PRINT",
      Put => "PUT",
      Read => "READ",
      Rem => "REM",
      Rename => "RENAME",
      Restore => "RESTORE",
      Return => "RETURN",
      Rset => "RSET",
      Run => "RUN",
      Save => "SAVE",
      Stop => "STOP",
      Swap => "SWAP",
      System => "SYSTEM",
      Text => "TEXT",
      Trace => "TRACE",
      Wend => "WEND",
      While => "WHILE",
      Write => "WRITE",

      Then => "THEN",
      Else => "ELSE",
      To => "TU",
      Step => "STEP",
      Fn => "FN",
      And => "AND",
      Or => "OR",
      Not => "NOT",
      At => "AT",

      Sleep => "SLEEP",
      Fputc => "FPUTC",
      Fread => "FREAD",
      Fwrite => "FWRITE",
      Fseek => "FSEEK",
      DebugPrint => "DEBUGPRINT",
    };
    write!(f, "{kw}")
  }
}

impl Debug for Punc {
  fn fmt(&self, f: &mut Formatter) -> fmt::Result {
    let kind = match self {
      Self::Eq => "=",
      Self::Lt => "<",
      Self::Gt => ">",
      Self::Plus => "+",
      Self::Minus => "-",
      Self::Times => "*",
      Self::Slash => "/",
      Self::Caret => "^",
      Self::Colon => ":",
      Self::LParen => "(",
      Self::RParen => ")",
      Self::Semicolon => ";",
      Self::Comma => ",",
      Self::Hash => "#",
    };
    write!(f, "{kind}")
  }
}

impl Debug for TokenKind {
  fn fmt(&self, f: &mut Formatter) -> fmt::Result {
    match self {
      Self::Ident => write!(f, "<id>"),
      Self::Label => write!(f, "<label>"),
      Self::Float => write!(f, "<float>"),
      Self::String => write!(f, "<string>"),
      Self::Punc(p) => write!(f, "\"{p:?}\""),
      Self::Keyword(p) => write!(f, "{p:?}"),
      Self::SysFunc(p) => write!(f, "{p:?}"),
      Self::Eof => write!(f, "<eof>"),
    }
  }
}

impl TokenKind {
  pub const fn to_usize(self) -> usize {
    match self {
      TokenKind::Ident => 0,
      TokenKind::Label => 1,
      TokenKind::Float => 2,
      TokenKind::String => 3,
      TokenKind::Punc(p) => 4 + p as usize,
      TokenKind::Keyword(k) => 24 + k as usize,
      TokenKind::SysFunc(k) => 110 + k as usize,
      _ => unreachable!(),
    }
  }
}

impl From<TokenKind> for usize {
  fn from(t: TokenKind) -> Self {
    t.to_usize()
  }
}

impl From<usize> for TokenKind {
  fn from(n: usize) -> Self {
    match n {
      0 => Self::Ident,
      1 => Self::Label,
      2 => Self::Float,
      3 => Self::String,
      4..24 => Self::Punc(Punc::from_usize(n - 4).unwrap()),
      24..110 => Self::Keyword(Keyword::from_usize(n - 24).unwrap()),
      110..150 => Self::SysFunc(SysFuncKind::from_usize(n - 110).unwrap()),
      _ => unreachable!(),
    }
  }
}
