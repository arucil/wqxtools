#![allow(clippy::write_with_newline)]

use super::{Label, Range, StmtId};
use smallvec::SmallVec;
#[cfg(test)]
use std::fmt::Write;
use std::fmt::{self, Debug, Display, Formatter};

#[cfg(test)]
use widestring::Utf16Str;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProgramLine {
  /// Includes newline.
  pub source_len: usize,
  pub label: Option<(Range, Label)>,
  pub stmts: SmallVec<[StmtId; 1]>,
  pub eol: Eol,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Eol {
  /// Only applies to last line.
  None,
  Lf,
  CrLf,
}

#[cfg(test)]
impl crate::parser::ParseResult<ProgramLine> {
  pub fn to_string(&self, text: &Utf16Str) -> String {
    let mut f = String::new();
    writeln!(&mut f, "label: {:?}", self.content.label).unwrap();
    writeln!(&mut f, "len: {}", self.content.source_len).unwrap();
    writeln!(&mut f, "eol: {:?}", self.content.eol).unwrap();
    writeln!(&mut f, "diagnostics: ").unwrap();
    for diag in &self.diagnostics {
      writeln!(&mut f, "  {:?}", diag).unwrap();
    }
    writeln!(&mut f, "-----------------").unwrap();
    for &stmt in self.content.stmts.iter() {
      self.stmt_arena[stmt]
        .print(&self.stmt_arena, &self.expr_arena, text, &mut f)
        .unwrap();
    }
    f
  }
}

impl Display for Eol {
  fn fmt(&self, f: &mut Formatter) -> fmt::Result {
    match self {
      Self::None => Ok(()),
      Self::Lf => write!(f, "\n"),
      Self::CrLf => write!(f, "\r\n"),
    }
  }
}

impl Eol {
  pub fn byte_len(&self) -> usize {
    match self {
      Self::None => 0,
      Self::Lf => 1,
      Self::CrLf => 2,
    }
  }
}
