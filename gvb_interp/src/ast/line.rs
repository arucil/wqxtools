use crate::diagnostic::Diagnostic;
use id_arena::Arena;
use super::*;

#[derive(Debug, Clone)]
pub struct ProgramLine {
  /// Includes newline.
  pub source_len: usize,
  pub label: Option<Label>,
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
