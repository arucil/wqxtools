use super::node::{Node, NodeId};
use id_arena::Arena;

#[derive(Debug, Clone)]
pub struct ProgramLine {
  pub label: u32,
  pub arena: Arena<Node>,
  pub stmts: Vec<NodeId>,
}
