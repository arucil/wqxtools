use super::{Label, Range, StmtId};
use crate::parser::ParseResult;
use smallvec::SmallVec;
use std::fmt::{Debug, Write};

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

impl ParseResult<ProgramLine> {
  pub fn to_string(&self, text: &str) -> String {
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
