#[cfg(test)]
use id_arena::Arena;

#[cfg(test)]
use widestring::Utf16Str;

use super::{ExprId, NonEmptyVec, Range};
use num_derive::FromPrimitive;
#[cfg(test)]
use std::fmt::Write;
use std::fmt::{self, Debug, Display, Formatter};
use std::str::FromStr;
#[cfg(test)]
use widestring::utf16str;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Expr {
  pub kind: ExprKind,
  pub range: Range,
}

#[derive(Debug, Clone, PartialEq, Eq)]
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
  Tab,
  Spc,

  Point,
  CheckKey,
  Fopen,
  Fgetc,
  Ftell,
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

static STR_TO_SYS_FUNC_KIND: phf::Map<&str, SysFuncKind> = phf::phf_map! {
  "abs" => SysFuncKind::Abs,
  "asc" => SysFuncKind::Asc,
  "atn" => SysFuncKind::Atn,
  "chr$" => SysFuncKind::Chr,
  "cos" => SysFuncKind::Cos,
  "cvi$" => SysFuncKind::Cvi,
  "cvs$" => SysFuncKind::Cvs,
  "eof" => SysFuncKind::Eof,
  "exp" => SysFuncKind::Exp,
  "int" => SysFuncKind::Int,
  "left$" => SysFuncKind::Left,
  "len" => SysFuncKind::Len,
  "lof" => SysFuncKind::Lof,
  "log" => SysFuncKind::Log,
  "mid$" => SysFuncKind::Mid,
  "mki$" => SysFuncKind::Mki,
  "mks$" => SysFuncKind::Mks,
  "peek" => SysFuncKind::Peek,
  "pos" => SysFuncKind::Pos,
  "right$" => SysFuncKind::Right,
  "rnd" => SysFuncKind::Rnd,
  "sgn" => SysFuncKind::Sgn,
  "sin" => SysFuncKind::Sin,
  "sqr" => SysFuncKind::Sqr,
  "str$" => SysFuncKind::Str,
  "tan" => SysFuncKind::Tan,
  "val" => SysFuncKind::Val,
  "spc" => SysFuncKind::Spc,
  "tab" => SysFuncKind::Tab,

  "fopen" => SysFuncKind::Fopen,
  "fgetc" => SysFuncKind::Fgetc,
  "ftell" => SysFuncKind::Ftell,
  "point" => SysFuncKind::Point,
  "checkkey" => SysFuncKind::CheckKey,
};

impl FromStr for SysFuncKind {
  type Err = ();
  fn from_str(s: &str) -> Result<Self, ()> {
    STR_TO_SYS_FUNC_KIND.get(s).ok_or(()).copied()
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
    write!(f, "{kind}")
  }
}

impl Display for BinaryOpKind {
  fn fmt(&self, f: &mut Formatter) -> fmt::Result {
    let kind = match self {
      Self::Eq => "=",
      Self::Ne => "<>",
      Self::Gt => ">",
      Self::Lt => "<",
      Self::Ge => ">=",
      Self::Le => "<=",
      Self::Add => "加法",
      Self::Sub => "减法",
      Self::Mul => "乘法",
      Self::Div => "除法",
      Self::Pow => "乘方",
      Self::And => "AND",
      Self::Or => "OR",
    };
    write!(f, "{kind}")
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
      Self::Spc => "SPC",
      Self::Tab => "TAB",
      Self::Point => "POINT",
      Self::CheckKey => "CHECKKEY",
      Self::Fopen => "FOPEN",
      Self::Fgetc => "FGETC",
      Self::Ftell => "FTELL",
    };
    write!(f, "{kind}")
  }
}

impl Debug for UnaryOpKind {
  fn fmt(&self, f: &mut Formatter) -> fmt::Result {
    let kind = match self {
      Self::Neg => "-",
      Self::Not => "NOT",
      Self::Pos => "+",
    };
    write!(f, "{kind}")
  }
}

#[cfg(test)]
impl Expr {
  pub fn print(
    &self,
    expr_arena: &Arena<Expr>,
    text: &Utf16Str,
    f: &mut impl Write,
  ) -> fmt::Result {
    use crate::util::utf16str_ext::Utf16StrExt;

    let range = self.range.clone();
    match &self.kind {
      ExprKind::Ident => write!(f, "<ID: {}>", &text[range.range()]),
      ExprKind::StringLit => {
        write!(f, "<STR: {}>", &text[range.range()])
      }
      ExprKind::NumberLit => {
        write!(f, "<NUM: {}>", &text[range.range()])
      }
      ExprKind::SysFuncCall {
        func: (func_range, kind),
        args,
      } => {
        assert_eq!(
          text[func_range.range()].to_ascii_uppercase(),
          format!("{kind:?}")
        );
        write!(f, "{kind:?}(")?;
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
          write!(f, "FN {}(", &text[func.range()])?;
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
          text[op_range.range()]
            .to_owned()
            .replace_char(' ', utf16str!(""))
            .to_ascii_uppercase(),
          format!("{kind:?}")
        );
        write!(f, "(")?;
        expr_arena[*lhs].print(expr_arena, text, f)?;
        write!(f, " {kind:?} ")?;
        expr_arena[*rhs].print(expr_arena, text, f)?;
        write!(f, ")")
      }
      ExprKind::Unary {
        op: (op_range, kind),
        arg,
      } => {
        assert_eq!(
          text[op_range.range()].to_owned().to_ascii_uppercase(),
          format!("{kind:?}")
        );
        write!(f, "({kind:?} ")?;
        expr_arena[*arg].print(expr_arena, text, f)?;
        write!(f, ")")
      }
      ExprKind::Index { name, indices } => {
        if let Some(name) = name {
          write!(f, "{}[", &text[name.range()])?;
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
