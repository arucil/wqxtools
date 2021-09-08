use crate::diagnostic::Diagnostic;
use super::node::{Node, NodeId};
use id_arena::Arena;

#[derive(Debug, Clone)]
pub struct ProgramLine {
  /// Includes newline.
  pub source_len: usize,
  pub label: Option<u16>,
  pub arena: Arena<Node>,
  pub stmts: Vec<NodeId>,
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
