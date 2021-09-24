use super::{Expr, Label, Range, Stmt, StmtId};
use crate::diagnostic::Diagnostic;
use id_arena::Arena;
use smallvec::SmallVec;
use std::fmt::{Debug, Write};

#[derive(Debug, Clone)]
pub struct ProgramLine {
  /// Includes newline.
  pub source_len: usize,
  pub label: Option<(Range, Label)>,
  pub stmt_arena: Arena<Stmt>,
  pub expr_arena: Arena<Expr>,
  pub stmts: SmallVec<[StmtId; 1]>,
  pub eol: Eol,
  pub diagnostics: Vec<Diagnostic>,
}

#[derive(Debug, Clone)]
pub enum Eol {
  /// Only applies to last line.
  None,
  Lf,
  CrLf,
}

impl ProgramLine {
  pub fn to_string(&self, text: &str) -> String {
    let mut f = String::new();
    writeln!(&mut f, "label: {:?}", self.label).unwrap();
    writeln!(&mut f, "len: {}", self.source_len).unwrap();
    writeln!(&mut f, "eol: {:?}", self.eol).unwrap();
    writeln!(&mut f, "diagnostics: ").unwrap();
    for diag in &self.diagnostics {
      writeln!(&mut f, "  {:?}", diag).unwrap();
    }
    writeln!(&mut f, "-----------------").unwrap();
    for &stmt in self.stmts.iter() {
      self.stmt_arena[stmt]
        .print(&self.stmt_arena, &self.expr_arena, text, &mut f)
        .unwrap();
    }
    f
  }
}
