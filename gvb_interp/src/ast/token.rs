use super::expr::SysFuncKind;
use super::node::NodeKind;
use num_derive::{FromPrimitive, ToPrimitive};
use num_traits::{FromPrimitive, ToPrimitive};
use std::convert::TryFrom;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TokenKind {
  Ident,
  Int,
  Real,
  String,
  Punc(Punc),
  Keyword(Keyword),
  SysFunc(SysFuncKind),
}

impl Into<NodeKind> for TokenKind {
  fn into(self) -> NodeKind {
    match self {
      Self::Ident => NodeKind(0),
      Self::Int => NodeKind(1),
      Self::Real => NodeKind(2),
      Self::String => NodeKind(3),
      Self::Punc(n) => NodeKind(100 + n.to_u16().unwrap()),
      Self::Keyword(n) => NodeKind(200 + n.to_u16().unwrap()),
      Self::SysFunc(n) => NodeKind(300 + n.to_u16().unwrap()),
    }
  }
}

impl TryFrom<NodeKind> for TokenKind {
  type Error = ();

  fn try_from(value: NodeKind) -> Result<Self, ()> {
    match value.0 {
      0 => Ok(Self::Ident),
      1 => Ok(Self::Int),
      2 => Ok(Self::Real),
      3 => Ok(Self::String),
      100..200 => {
        Punc::from_u16(value.0).map_or(Err(()), |n| Ok(Self::Punc(n)))
      }
    }
  }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, FromPrimitive, ToPrimitive)]
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

  Sleep,
  Paint,
  Fputc,
  Fread,
  Fwrite,
  Fseek,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, FromPrimitive, ToPrimitive)]
pub enum Punc {
  Eq,
  Ne,
  Le,
  Ge,
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
  Semi,
  Comma,
  Hash,
}
