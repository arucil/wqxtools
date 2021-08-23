use indextree::{Arena, NodeId};
use super::node::Node;

#[derive(Debug, Clone)]
pub struct ProgramLine {
  pub label: u32,
  pub arena: Arena<Node>,
  pub stmts: Vec<NodeId>,
}
