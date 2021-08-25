use super::ast::Range;

#[derive(Debug, Clone)]
pub struct ParseResult {
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

pub fn parse_line(line: &str) -> ParseResult {}
