use crate::ast::Range;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Diagnostic {
  pub severity: Severity,
  pub message: String,
  pub range: Range,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Severity {
  Warning,
  Error,
}

impl Diagnostic {
  pub fn new_error(range: Range, message: impl ToString) -> Self {
    Self {
      severity: Severity::Error,
      range,
      message: message.to_string()
    }
  }

  pub fn new_warning(range: Range, message: impl ToString) -> Self {
    Self {
      severity: Severity::Warning,
      range,
      message: message.to_string()
    }
  }
}