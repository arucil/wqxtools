use super::node::Node;
use indextree::{Arena, NodeId};

#[derive(Debug, Clone)]
pub struct ProgramLine {
  pub label: u32,
  pub arena: Arena<Node>,
  pub stmts: Vec<NodeId>,
}
