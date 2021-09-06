use super::{Range, NodeId, NonEmptyVec};

#[derive(Debug, Clone)]
pub enum Expr {
  Ident,
  StringLit,
  NumberLit,
  SysFuncCall {
    func: (Range, SysFuncKind),
    args: NonEmptyVec<[NodeId; 1]>,
  },
  UserFuncCall {
    /// ident
    func: Range,
    arg: NodeId,
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
    name: Range,
    indices: NonEmptyVec<[NodeId; 1]>,
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
