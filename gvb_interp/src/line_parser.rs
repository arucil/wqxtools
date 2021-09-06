use super::ast::{Range, Program};

#[derive(Debug, Clone)]
pub struct ParseResult {
  pub program: Program,
  pub diagnostics: Vec<Diagnostic>,
}

#[derive(Debug, Clone)]
pub struct Diagnostic {
  pub severity: Severity,
  pub message: String,
  pub range: Range,
}

#[derive(Debug, Clone)]
pub enum Severity {
  Info,
  Warning,
  Error,
}

struct State {
  diagnostics: Vec<Diagnostic>,
}

pub fn parse_line(line: &str) -> ParseResult {}
