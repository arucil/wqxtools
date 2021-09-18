use super::{Expr, Stmt};
use id_arena::Id;

pub type ExprId = Id<Expr>;
pub type StmtId = Id<Stmt>;

pub(crate) trait NodeBuilder {
  fn new_stmt(&mut self, stmt: Stmt) -> StmtId;
  fn new_expr(&mut self, expr: Expr) -> ExprId;
  fn stmt_node(&self, stmt: StmtId) -> &Stmt;
  fn expr_node(&self, expr: ExprId) -> &Expr;
}
