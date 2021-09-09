use super::{ExprId, NonEmptyVec, Range};
use std::str::FromStr;

#[derive(Debug, Clone)]
pub struct Expr {
  pub kind: ExprKind,
  pub range: Range,
  pub is_recovered: bool,
}

#[derive(Debug, Clone)]
pub enum ExprKind {
  Ident,
  StringLit,
  NumberLit,
  SysFuncCall {
    func: (Range, SysFuncKind),
    args: NonEmptyVec<[ExprId; 1]>,
  },
  UserFuncCall {
    /// ident
    func: Range,
    arg: ExprId,
  },
  Binary {
    lhs: ExprId,
    op: (Range, BinaryOpKind),
    rhs: ExprId,
  },
  Unary {
    op: (Range, UnaryOpKind),
    arg: ExprId,
  },
  Index {
    name: Range,
    indices: NonEmptyVec<[ExprId; 1]>,
  },
  Inkey,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SysFuncKind {
  Abs,
  Asc,
  Atn,
  Chr,
  Cos,
  Cvi,
  Cvs,
  Eof,
  Exp,
  Int,
  Left,
  Len,
  Lof,
  Log,
  Mid,
  Mki,
  Mks,
  Peek,
  Pos,
  Right,
  Rnd,
  Sgn,
  Sin,
  Sqr,
  Str,
  Tan,
  Val,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BinaryOpKind {
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
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnaryOpKind {
  Not,
  Neg,
  Pos,
}

impl FromStr for SysFuncKind {
  type Err = ();
  fn from_str(s: &str) -> Result<Self, ()> {
    match s {
      "abs" => Ok(Self::Abs),
      "asc" => Ok(Self::Asc),
      "atn" => Ok(Self::Atn),
      "chr$" => Ok(Self::Chr),
      "cos" => Ok(Self::Cos),
      "cvi$" => Ok(Self::Cvi),
      "cvs$" => Ok(Self::Cvs),
      "eof" => Ok(Self::Eof),
      "exp" => Ok(Self::Exp),
      "int" => Ok(Self::Int),
      "left$" => Ok(Self::Left),
      "len" => Ok(Self::Len),
      "lof" => Ok(Self::Lof),
      "log" => Ok(Self::Log),
      "mid$" => Ok(Self::Mid),
      "mki$" => Ok(Self::Mki),
      "mks$" => Ok(Self::Mks),
      "peek" => Ok(Self::Peek),
      "pos" => Ok(Self::Pos),
      "right$" => Ok(Self::Right),
      "rnd" => Ok(Self::Rnd),
      "sgn" => Ok(Self::Sgn),
      "sin" => Ok(Self::Sin),
      "sqr" => Ok(Self::Sqr),
      "str$" => Ok(Self::Str),
      "tan" => Ok(Self::Tan),
      "val" => Ok(Self::Val),
      _ => Err(()),
    }
  }
}
