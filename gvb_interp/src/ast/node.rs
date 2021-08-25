#[derive(Debug, Clone)]
pub struct Range {
  pub start: usize,
  pub end: usize,
}

#[derive(Debug, Clone)]
pub struct Node {
  pub kind: NodeKind,
  pub range: Range,
  pub data: NodeData,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NodeKind(pub(super) u16);

#[derive(Debug, Clone)]
pub enum NodeData {
  Expr(super::expr::Expr),
}
