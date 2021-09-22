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
  Paint,
  Fputc,
  Fread,
  Fwrite,
  Fseek,
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

impl FromStr for Keyword {
  type Err = ();
  fn from_str(s: &str) -> Result<Self, ()> {
    match s {
      "auto" => Ok(Self::Auto),
      "beep" => Ok(Self::Beep),
      "box" => Ok(Self::Box),
      "call" => Ok(Self::Call),
      "circle" => Ok(Self::Circle),
      "clear" => Ok(Self::Clear),
      "close" => Ok(Self::Close),
      "cls" => Ok(Self::Cls),
      "cont" => Ok(Self::Cont),
      "copy" => Ok(Self::Copy),
      "data" => Ok(Self::Data),
      "def" => Ok(Self::Def),
      "del" => Ok(Self::Del),
      "dim" => Ok(Self::Dim),
      "draw" => Ok(Self::Draw),
      "edit" => Ok(Self::Edit),
      "ellipse" => Ok(Self::Ellipse),
      "end" => Ok(Self::End),
      "field" => Ok(Self::Field),
      "files" => Ok(Self::Files),
      "flash" => Ok(Self::Flash),
      "for" => Ok(Self::For),
      "get" => Ok(Self::Get),
      "gosub" => Ok(Self::Gosub),
      "goto" => Ok(Self::Goto),
      "graph" => Ok(Self::Graph),
      "if" => Ok(Self::If),
      "inkey$" => Ok(Self::Inkey),
      "input" => Ok(Self::Input),
      "inverse" => Ok(Self::Inverse),
      "kill" => Ok(Self::Kill),
      "let" => Ok(Self::Let),
      "line" => Ok(Self::Line),
      "list" => Ok(Self::List),
      "load" => Ok(Self::Load),
      "locate" => Ok(Self::Locate),
      "lset" => Ok(Self::Lset),
      "new" => Ok(Self::New),
      "next" => Ok(Self::Next),
      "normal" => Ok(Self::Normal),
      "notrace" => Ok(Self::Notrace),
      "on" => Ok(Self::On),
      "open" => Ok(Self::Open),
      "play" => Ok(Self::Play),
      "poke" => Ok(Self::Poke),
      "pop" => Ok(Self::Pop),
      "print" => Ok(Self::Print),
      "put" => Ok(Self::Put),
      "read" => Ok(Self::Read),
      "rem" => Ok(Self::Rem),
      "rename" => Ok(Self::Rename),
      "restore" => Ok(Self::Restore),
      "return" => Ok(Self::Return),
      "rset" => Ok(Self::Rset),
      "run" => Ok(Self::Run),
      "save" => Ok(Self::Save),
      "stop" => Ok(Self::Stop),
      "swap" => Ok(Self::Swap),
      "system" => Ok(Self::System),
      "text" => Ok(Self::Text),
      "trace" => Ok(Self::Trace),
      "wend" => Ok(Self::Wend),
      "while" => Ok(Self::While),
      "write" => Ok(Self::Write),

      "then" => Ok(Self::Then),
      "else" => Ok(Self::Else),
      "to" => Ok(Self::To),
      "step" => Ok(Self::Step),
      "fn" => Ok(Self::Fn),
      "and" => Ok(Self::And),
      "or" => Ok(Self::Or),
      "not" => Ok(Self::Not),

      "sleep" => Ok(Self::Sleep),
      "paint" => Ok(Self::Paint),
      "fputc" => Ok(Self::Fputc),
      "fread" => Ok(Self::Fread),
      "fwrite" => Ok(Self::Fwrite),
      "fseek" => Ok(Self::Fseek),
      _ => Err(()),
    }
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
      _ => panic!("invalid char {}", c),
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
      Paint => "PAINT",
      Fputc => "FPUTC",
      Fread => "FREAD",
      Fwrite => "FWRITE",
      Fseek => "FSEEK",
    };
    write!(f, "{}", kw)
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
    write!(f, "{}", kind)
  }
}

impl Debug for TokenKind {
  fn fmt(&self, f: &mut Formatter) -> fmt::Result {
    match self {
      Self::Ident => write!(f, "<id>"),
      Self::Label => write!(f, "<label>"),
      Self::Float => write!(f, "<float>"),
      Self::String => write!(f, "<string>"),
      Self::Punc(p) => write!(f, "\"{:?}\"", p),
      Self::Keyword(p) => write!(f, "{:?}", p),
      Self::SysFunc(p) => write!(f, "{:?}", p),
      Self::Eof => write!(f, "<eof>"),
    }
  }
}

impl TokenKind {
  pub const fn to_usize(&self) -> usize {
    match self {
      TokenKind::Ident => 0,
      TokenKind::Label => 1,
      TokenKind::Float => 2,
      TokenKind::String => 3,
      TokenKind::Punc(p) => 4 + *p as usize,
      TokenKind::Keyword(k) => 24 + *k as usize,
      TokenKind::SysFunc(k) => 110 + *k as usize,
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
      4 => unreachable!("SysFunc"),
      4..24 => Self::Punc(Punc::from_usize(n - 4).unwrap()),
      24..110 => Self::Keyword(Keyword::from_usize(n - 24).unwrap()),
      110..150 => Self::SysFunc(SysFuncKind::from_usize(n - 110).unwrap()),
      _ => unreachable!(),
    }
  }
}
