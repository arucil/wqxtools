use super::node::Range;
use indextree::NodeId;
use smallvec::SmallVec;
use num_derive::{FromPrimitive, ToPrimitive};

#[derive(Debug, Clone)]
pub enum Expr {
  Ident,
  StringLit,
  NumberLit,
  SysFuncCall {
    func: (Range, SysFuncKind),
    args: SmallVec<[NodeId; 1]>,
  },
  UserFuncCall {
    /// ident
    func: NodeId,
    args: SmallVec<[NodeId; 1]>,
  },
  Binary {
    lhs: NodeId,
    op: (Range, BinaryOpKind),
    rhs: NodeId,
  },
  Unary {
    op: (Range, UnaryOpKind),
    arg: NodeId,
  },
  Index {
    /// ident
    name: NodeId,
    indices: SmallVec<[NodeId; 1]>,
  },
  Paren {
    expr: NodeId,
  },
  Inkey,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, FromPrimitive, ToPrimitive)]
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