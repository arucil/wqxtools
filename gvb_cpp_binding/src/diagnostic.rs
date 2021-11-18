
#[repr(C)]
pub enum Severity {
  Warning,
  Error,
}

#[repr(C)]
pub struct Diagnostic<M> {
  pub line: usize,
  pub start: usize,
  pub end: usize,
  pub message: M,
  pub severity: Severity,
}