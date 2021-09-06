use super::Range;
use id_arena::Id;

pub type NodeId = Id<Node>;

#[derive(Debug, Clone)]
pub struct Node {
  pub kind: NodeKind,
  pub range: Range,
  pub is_recovered: bool,
}

#[derive(Debug, Clone)]
pub enum NodeKind {
  Expr(super::expr::Expr),
  Stmt(super::stmt::Stmt),
}
