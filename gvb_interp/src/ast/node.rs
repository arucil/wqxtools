use super::{Expr, Stmt, Range};
use id_arena::Id;

pub type ExprId = Id<Expr>;
pub type StmtId = Id<Stmt>;

pub(crate) trait NodeBuilder {
  fn new_stmt(&mut self, stmt: Stmt) -> StmtId;
  fn new_expr(&mut self, expr: Expr) -> ExprId;
  fn stmt_range(&self, stmt: StmtId) -> Range;
}