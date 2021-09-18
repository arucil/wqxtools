use id_arena::Arena;

use super::{ExprId, NonEmptyVec, Range};
use num_derive::FromPrimitive;
use std::fmt::{self, Debug, Formatter, Write};
use std::str::FromStr;

#[derive(Debug, Clone)]
pub struct Expr {
  pub kind: ExprKind,
  pub range: Range,
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
    func: Option<Range>,
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
    name: Option<Range>,
    indices: NonEmptyVec<[ExprId; 1]>,
  },
  Inkey,
  Error,
}

#[derive(Clone, Copy, PartialEq, Eq, FromPrimitive)]
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

#[derive(Clone, Copy, PartialEq, Eq)]
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

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum UnaryOpKind {
  Not,
  Neg,
  Pos,
}

impl Expr {
  pub fn new(kind: ExprKind, range: Range) -> Self {
    Self { kind, range }
  }
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

impl Debug for BinaryOpKind {
  fn fmt(&self, f: &mut Formatter) -> fmt::Result {
    let kind = match self {
      Self::Eq => "=",
      Self::Ne => "<>",
      Self::Gt => ">",
      Self::Lt => "<",
      Self::Ge => ">=",
      Self::Le => "<=",
      Self::Add => "+",
      Self::Sub => "-",
      Self::Mul => "*",
      Self::Div => "/",
      Self::Pow => "^",
      Self::And => "AND",
      Self::Or => "OR",
    };
    write!(f, "{}", kind)
  }
}

impl Debug for SysFuncKind {
  fn fmt(&self, f: &mut Formatter) -> fmt::Result {
    let kind = match self {
      Self::Abs => "ABS",
      Self::Asc => "ASC",
      Self::Atn => "ATN",
      Self::Chr => "CHR$",
      Self::Cos => "COS",
      Self::Cvi => "CVI$",
      Self::Cvs => "CVS$",
      Self::Eof => "EOF",
      Self::Exp => "EXP",
      Self::Int => "INT",
      Self::Left => "LEFT$",
      Self::Len => "LEN",
      Self::Lof => "LOF",
      Self::Log => "LOG",
      Self::Mid => "MID$",
      Self::Mki => "MKI$",
      Self::Mks => "MKS$",
      Self::Peek => "PEEK",
      Self::Pos => "POS",
      Self::Right => "RIGHT$",
      Self::Rnd => "RND",
      Self::Sgn => "SGN",
      Self::Sin => "SIN",
      Self::Sqr => "SQR",
      Self::Str => "STR$",
      Self::Tan => "TAN",
      Self::Val => "VAL",
    };
    write!(f, "{}", kind)
  }
}

impl Debug for UnaryOpKind {
  fn fmt(&self, f: &mut Formatter) -> fmt::Result {
    let kind = match self {
      Self::Neg => "-",
      Self::Not => "NOT",
      Self::Pos => "+",
    };
    write!(f, "{}", kind)
  }
}

impl Expr {
  pub fn print(
    &self,
    expr_arena: &Arena<Expr>,
    text: &str,
    f: &mut impl Write,
  ) -> fmt::Result {
    let range = self.range.clone();
    match &self.kind {
      ExprKind::Ident => write!(f, "<ID: {}>", &text[range.start..range.end]),
      ExprKind::StringLit => {
        write!(f, "<STR: {}>", &text[range.start..range.end])
      }
      ExprKind::NumberLit => {
        write!(f, "<NUM: {}>", &text[range.start..range.end])
      }
      ExprKind::SysFuncCall {
        func: (func_range, kind),
        args,
      } => {
        assert_eq!(
          text[func_range.start..func_range.end].to_ascii_uppercase(),
          format!("{:?}", kind)
        );
        write!(f, "{:?}(", kind)?;
        let mut comma = false;
        for &arg in args.iter() {
          if comma {
            write!(f, ", ")?;
          }
          comma = true;
          expr_arena[arg].print(expr_arena, text, f)?;
        }
        write!(f, ")")
      }
      ExprKind::UserFuncCall { func, arg } => {
        if let Some(func) = func {
          write!(f, "FN {}(", &text[func.start..func.end])?;
        } else {
          write!(f, "FN ???(")?;
        }
        expr_arena[*arg].print(expr_arena, text, f)?;
        write!(f, ")")
      }
      ExprKind::Binary {
        lhs,
        op: (op_range, kind),
        rhs,
      } => {
        assert_eq!(
          text[op_range.start..op_range.end]
            .to_owned()
            .replace(" ", "")
            .to_ascii_uppercase(),
          format!("{:?}", kind)
        );
        write!(f, "(")?;
        expr_arena[*lhs].print(expr_arena, text, f)?;
        write!(f, " {:?} ", kind)?;
        expr_arena[*rhs].print(expr_arena, text, f)?;
        write!(f, ")")
      }
      ExprKind::Unary {
        op: (op_range, kind),
        arg,
      } => {
        assert_eq!(
          text[op_range.start..op_range.end]
            .to_owned()
            .to_ascii_uppercase(),
          format!("{:?}", kind)
        );
        write!(f, "({:?} ", kind)?;
        expr_arena[*arg].print(expr_arena, text, f)?;
        write!(f, ")")
      }
      ExprKind::Index { name, indices } => {
        if let Some(name) = name {
          write!(f, "{}[", &text[name.start..name.end])?;
        } else {
          write!(f, "???[")?;
        }
        let mut comma = false;
        for &arg in indices.iter() {
          if comma {
            write!(f, ", ")?;
          }
          comma = true;
          expr_arena[arg].print(expr_arena, text, f)?;
        }
        write!(f, "]")
      }
      ExprKind::Inkey => write!(f, "<INKEY$>"),
      ExprKind::Error => write!(f, "<ERROR>"),
    }
  }
}
