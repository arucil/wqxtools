use crate::diagnostic::Diagnostic;
use id_arena::Arena;
use super::*;

#[derive(Debug, Clone)]
pub struct ProgramLine {
  /// Includes newline.
  pub source_len: usize,
  pub label: Option<u16>,
  pub stmt_arena: Arena<Stmt>,
  pub expr_arena: Arena<Expr>,
  pub stmts: Vec<StmtId>,
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
