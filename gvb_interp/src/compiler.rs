use crate::{ast::*, diagnostic::*};
use std::fmt::{self, Display, Formatter};

pub trait CodeEmitter {
  type Symbol;

  fn begin_label(&mut self, label: Label);
  fn end_label(&mut self);

  fn emit_no_op(&mut self);
  fn emit_nullary_op(&mut self, kind: &StmtKind);
  fn emit_generic_op(&mut self, kind: &StmtKind, arity: usize);
  fn emit_data<I: IntoIterator<Item = (String, bool)>>(&mut self, data: I);
  fn begin_def(&mut self, name: Self::Symbol, param: Self::Symbol);
  fn end_def(&mut self);
  fn emit_dim(&mut self, name: Self::Symbol, dimensions: usize);
  fn emit_field<I: IntoIterator<Item = >>(&mut self);
}

pub fn compile<E: CodeEmitter>(
  text: impl AsRef<str>,
  prog: &Program,
  code_emitter: &mut E,
) -> Vec<Diagnostic> {
  let text = text.as_ref();
  let mut state = CompileState {
    text,
    code_emitter,
    diagnostics: vec![],
  };

  state.compile(prog);

  state.diagnostics
}

struct CompileState<'a, 'b, E> {
  text: &'b str,
  code_emitter: &'a mut E,
  diagnostics: Vec<Diagnostic>,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Type {
  Number,
  String,
  Error,
}

impl<'a, 'b, E: CodeEmitter> CompileState<'a, 'b, E> {
  fn add_error(&mut self, range: Range, message: impl ToString) {
    self.diagnostics.push(Diagnostic::new_error(range, message));
  }

  fn add_warning(&mut self, range: Range, message: impl ToString) {
    self
      .diagnostics
      .push(Diagnostic::new_warning(range, message));
  }

  fn compile(&mut self, prog: &Program) {
    let mut last_label = -1;

    for line in &prog.lines {
      let mut label = None;
      if let Some((range, l)) = &line.label {
        if l.0 as i32 > last_label {
          self.code_emitter.begin_label(*l);
          label = Some(*l);
        } else {
          self.add_error(range.clone(), "行号必须递增");
        }
        last_label = l.0 as i32;
      }

      for &stmt in &line.stmts {
        self.compile_stmt(line, stmt);
      }

      if let Some(_) = label {
        self.code_emitter.end_label();
      }
    }
  }

  fn compile_stmt(&mut self, line: &ProgramLine, stmt: StmtId) {
    macro_rules! compile_draw_stmt {
      ($stmt:expr, $args:expr, $kind:ident, $min_arity:literal, $max_arity:literal) => {
    for (i, &arg) in $args.iter().enumerate() {
      let ty = self.compile_expr(line, arg);
      if i < $max_arity {
        if !ty.type_eq(Type::Number) {
        let range = &line.expr_arena[arg].range;
          self.add_error(range.clone(), format!("表达式类型错误。{} 语句的参数是{}类型，而这个表达式是{}类型", stringify!($kind), Type::Number, ty));
        }
      }
    }

    if $args.len() < $min_arity {
      self.add_error($stmt.range.clone(), concat!(stringify!($kind), " 语句至少要有 ", stringify!($min_arity), " 个参数"));
    } else if $args.len() > $max_arity {
      for &arg in &$args[$max_arity..] {
        let arg = &line.expr_arena[arg];
        self.add_error(arg.range.clone(), concat!("多余的参数。", stringify!($kind), " 语句最多允许有 ", stringify!($max_arity), " 个参数"));
      }
    } else {
      self.code_emitter.emit_generic_op(&$stmt.kind, $args.len());
    };
      }
    }

    let stmt = &line.stmt_arena[stmt];
    match &stmt.kind {
      StmtKind::Auto(_) => self.code_emitter.emit_no_op(),
      StmtKind::Beep => self.code_emitter.emit_nullary_op(&stmt.kind),
      StmtKind::Box(args) => {
        compile_draw_stmt!(&stmt, args, BOX, 4, 6);
      }
      StmtKind::Call(arg) => {
        let ty = self.compile_expr(line, *arg);
        if !ty.type_eq(Type::Number) {
          let range = &line.expr_arena[*arg].range;
          self.add_error(
            range.clone(),
            format!(
              "表达式类型错误。CALL 语句的参数是{}类型，而这个表达式是{}类型",
              Type::Number,
              ty
            ),
          );
        }
        self.code_emitter.emit_generic_op(&stmt.kind, 1);
      }
      StmtKind::Circle(args) => {
        compile_draw_stmt!(&stmt, args, CIRCLE, 3, 5);
      }
      StmtKind::Clear => self.code_emitter.emit_nullary_op(&stmt.kind),
      StmtKind::Close { filenum } => {
        let ty = self.compile_expr(line, *filenum);
        if !ty.type_eq(Type::Number) {
          let range = &line.expr_arena[*filenum].range;
          self.add_error(range.clone(), format!("表达式类型错误。CLOSE 语句的文件号必须是{}类型，而这个表达式是{}类型", Type::Number, ty));
        }
        self.code_emitter.emit_generic_op(&stmt.kind, 1);
      }
      StmtKind::Cls => self.code_emitter.emit_nullary_op(&stmt.kind),
      StmtKind::Cont => self.code_emitter.emit_nullary_op(&stmt.kind),
      StmtKind::Copy(_) => self.code_emitter.emit_no_op(),
      StmtKind::Data(data) => {
        let text = self.text;
        let datum_iter = data.iter().map(|datum| {
          let text = &text[datum.range.start..datum.range.end];
          let text = if datum.is_quoted {
            if text.ends_with('"') {
              text[1..text.len() - 1].to_owned()
            } else {
              text[1..].to_owned()
            }
          } else {
            text.to_owned()
          };
          (text, datum.is_quoted)
        });
        self.code_emitter.emit_data(datum_iter);
      }
      StmtKind::Def { name, param, body } => {
        let name = name.map(|name_range| {
          let (name, ty) = self.compile_sym(name_range);
          if !ty.type_eq(Type::Number) {
            self.add_error(
              name_range.clone(),
              format!(
                "表达式类型错误。自定义函数的类型必须是{}类型",
                Type::Number
              ),
            );
          }
          name
        });
        let param = param.map(|param_range| {
          let (param, ty) = self.compile_sym(param_range);
          if !ty.type_eq(Type::Number) {
            self.add_error(
              param_range.clone(),
              format!(
                "表达式类型错误。自定义函数的参数的类型必须是{}类型",
                Type::Number
              ),
            );
          }
          param
        });

        if let (Some(name), Some(param)) = (name, param) {
          self.code_emitter.begin_def(name, param);
          self.compile_expr(line, *body);
          self.code_emitter.end_def();
        } else {
          self.compile_expr(line, *body);
        }
      }
      StmtKind::Del(_) => self.code_emitter.emit_no_op(),
      StmtKind::Dim(vars) => {
        for &var in vars.iter() {
          let var = &line.expr_arena[var];
          if let ExprKind::Index { name, indices } = &var.kind {
            for &index in indices.iter() {
              let ty = self.compile_expr(line, index);
            }
            if let Some(name) = name {
              let (name, _) = self.compile_sym(name.clone());
              self.code_emitter.emit_dim(name, indices.len());
            }
          }
        }
      }
      StmtKind::Draw(args) => {
        compile_draw_stmt!(&stmt, args, DRAW, 2, 3);
      }
      StmtKind::Edit(_) => self.code_emitter.emit_no_op(),
      StmtKind::Ellipse(args) => {
        compile_draw_stmt!(&stmt, args, ELLIPSE, 4, 6);
      }
      StmtKind::End => self.code_emitter.emit_nullary_op(&stmt.kind),
      StmtKind::Field { filenum, fields } => {
        let ty = self.compile_expr(line, *filenum);
        if !ty.type_eq(Type::Number) {
          let range = &line.expr_arena[*filenum].range;
          self.add_error(range.clone(), format!("表达式类型错误。FIELD 语句的文件号必须是{}类型，而这个表达式是{}类型", Type::Number, ty));
        }

        for field in fields.iter() {
          let ty = self.compile_expr(line, field.len);
        }
      }
      StmtKind::Files(_) => self.code_emitter.emit_no_op(),
      StmtKind::Flash => self.code_emitter.emit_nullary_op(&stmt.kind),
      StmtKind::For {
        var,
        start,
        end,
        step,
      } => {}
      StmtKind::Get { filenum, record } => {}
      StmtKind::GoSub(label) => {}
      StmtKind::GoTo { label, .. } => {}
      StmtKind::Graph => self.code_emitter.emit_nullary_op(&stmt.kind),
      StmtKind::If { cond, conseq, alt } => {}
      StmtKind::InKey => self.code_emitter.emit_nullary_op(&stmt.kind),
      StmtKind::Input { source, vars } => {}
      StmtKind::Inverse => self.code_emitter.emit_nullary_op(&stmt.kind),
      StmtKind::Kill(_) => self.code_emitter.emit_no_op(),
      StmtKind::Let { var, value } => {}
      StmtKind::Line(args) => {}
      StmtKind::List(_) => {}
      StmtKind::Load(_) => {}
      StmtKind::Locate { row, column } => {}
      StmtKind::LSet { var, value } => {}
      StmtKind::New(_) => {}
      StmtKind::Next { vars } => {}
      StmtKind::Normal => {}
      StmtKind::NoTrace => {}
      StmtKind::On {
        cond,
        labels,
        is_sub,
      } => {}
      StmtKind::Open {
        filename,
        mode,
        filenum,
        len,
      } => {}
      StmtKind::Play(arg) => {}
      StmtKind::Poke { addr, value } => {}
      StmtKind::Pop => {}
      StmtKind::Print(elems) => {}
      StmtKind::Put { filenum, record } => {}
      StmtKind::Read(vars) => {}
      StmtKind::Rem(_) => {}
      StmtKind::Rename(_) => {}
      StmtKind::Restore(label) => {}
      StmtKind::Return => {}
      StmtKind::RSet { var, value } => {}
      StmtKind::Run => {}
      StmtKind::Save(_) => {}
      StmtKind::Stop(_) => {}
      StmtKind::Swap { left, right } => {}
      StmtKind::System => {}
      StmtKind::Text => {}
      StmtKind::Trace => {}
      StmtKind::Wend => {}
      StmtKind::While(cond) => {}
      StmtKind::Write { filenum, data } => {}
      StmtKind::NoOp => {}
    }
  }

  fn compile_expr(&mut self, line: &ProgramLine, expr: ExprId) -> Type {}

  fn compile_sym(&mut self, range: Range) -> (E::Symbol, Type) {}
}

impl Type {
  fn type_eq(&self, other: Type) -> bool {
    match (self, other) {
      (Self::Error, _) => true,
      (_, Self::Error) => true,
      (Self::Number, Self::Number) => true,
      (Self::String, Self::String) => true,
      _ => false,
    }
  }
}

impl Display for Type {
  fn fmt(&self, f: &mut Formatter) -> fmt::Result {
    match self {
      Self::Number => write!(f, "数值"),
      Self::String => write!(f, "字符串"),
      Self::Error => unreachable!(),
    }
  }
}
