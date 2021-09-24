use crate::util::mbf5::{FloatError, Mbf5, Mbf5Accum};
use crate::{ast::*, diagnostic::*};
use smallvec::SmallVec;
use std::collections::HashMap;
use std::convert::TryFrom;
use std::fmt::{self, Display, Formatter};
use std::num::NonZeroUsize;

pub trait CodeEmitter {
  type Symbol: Copy;
  type Addr: Copy;
  type DatumIndex: Copy;

  fn begin_label(&mut self, label: Label);
  fn end_label(&mut self);
  fn find_label(&mut self, label: Label) -> Option<Self::Addr>;

  fn emit_no_op(&mut self, range: Range);

  fn emit_op(&mut self, range: Range, kind: &StmtKind, arity: usize);

  fn emit_datum(
    &mut self,
    range: Range,
    datum: String,
    is_quoted: bool,
  ) -> Self::DatumIndex;

  fn begin_def_fn(
    &mut self,
    range: Range,
    name: Self::Symbol,
    param: Self::Symbol,
  );
  fn end_def_fn(&mut self);

  fn emit_dim(
    &mut self,
    range: Range,
    name: Self::Symbol,
    dimensions: NonZeroUsize,
  );

  fn emit_lvalue(
    &mut self,
    range: Range,
    name: Self::Symbol,
    dimensions: usize,
  );

  fn emit_fn_lvalue(
    &mut self,
    range: Range,
    name: Self::Symbol,
    param: Self::Symbol,
  );

  fn emit_field(&mut self, range: Range, fields: NonZeroUsize);

  fn emit_for(&mut self, range: Range, var: Self::Symbol, has_step: bool);

  fn emit_next(&mut self, range: Range, var: Option<Self::Symbol>);

  fn make_symbol(&mut self, name: String) -> Self::Symbol;

  fn emit_gosub(&mut self, range: Range) -> Self::Addr;

  fn emit_goto(&mut self, range: Range) -> Self::Addr;

  fn patch_jump_addr(&mut self, addr: Self::Addr, label_addr: Self::Addr);

  fn emit_je(&mut self, range: Range) -> Self::Addr;

  fn current_addr(&self) -> Self::Addr;

  fn emit_on(&mut self, range: Range, labels: NonZeroUsize);

  fn emit_set_row(&mut self, range: Range);
  fn emit_set_column(&mut self, range: Range);

  fn emit_restore(&mut self, range: Range) -> Self::Addr;

  fn patch_datum_index(&mut self, addr: Self::Addr, index: Self::DatumIndex);

  fn emit_keyboard_input(
    &mut self,
    range: Range,
    prompt: Option<String>,
    fields: NonZeroUsize,
  );

  fn emit_file_input(&mut self, fields: NonZeroUsize);

  fn emit_open(&mut self, mode: FileMode, has_len: bool);

  fn emit_read(&mut self, range: Range);

  fn emit_print_newline(&mut self, range: Range);
  fn emit_print_spc(&mut self, range: Range);
  fn emit_print_tab(&mut self, range: Range);
  fn emit_print_value(&mut self, range: Range);

  fn emit_pop(&mut self);

  fn emit_write(&mut self, range: Range, to_file: bool);
  fn emit_write_end(&mut self, range: Range, to_file: bool);

  fn emit_number(&mut self, range: Range, num: Mbf5);
  fn emit_var(&mut self, range: Range, sym: Self::Symbol);
  fn emit_string(&mut self, range: Range, str: String);
  fn emit_inkey(&mut self, range: Range);
  fn emit_index(
    &mut self,
    range: Range,
    name: Self::Symbol,
    dimensions: NonZeroUsize,
  );
  fn emit_unary_expr(&mut self, range: Range, kind: UnaryOpKind);
  fn emit_binary_expr(&mut self, range: Range, kind: BinaryOpKind);
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
    pending_jump_labels: vec![],
    pending_datum_indices: vec![],
    data_start: HashMap::new(),
    line: std::ptr::null(),
  };

  state.compile(prog);

  state.diagnostics
}

struct CompileState<'a, 'b, E: CodeEmitter> {
  text: &'b str,
  code_emitter: &'a mut E,
  diagnostics: Vec<Diagnostic>,
  pending_jump_labels: Vec<(E::Addr, Range, Option<Label>)>,
  pending_datum_indices: Vec<(E::Addr, Range, Label)>,
  data_start: HashMap<Label, E::DatumIndex>,
  line: *const ProgramLine,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Type {
  Integer,
  Real,
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

  fn expr_node(&self, expr: ExprId) -> &'a Expr {
    unsafe { &(*self.line).expr_arena[expr] }
  }

  fn stmt_node(&self, stmt: StmtId) -> &'a Stmt {
    unsafe { &(*self.line).stmt_arena[stmt] }
  }

  fn label(&self) -> Option<Label> {
    unsafe { (*self.line).label.map(|x| x.1) }
  }

  fn compile(&mut self, prog: &Program) {
    let mut last_label = -1;

    for line in &prog.lines {
      self.line = line as *const _;
      if let Some((range, l)) = &line.label {
        if l.0 as i32 > last_label {
          self.code_emitter.begin_label(*l);
        } else {
          self.add_error(range.clone(), "行号必须递增");
        }
        last_label = l.0 as i32;
      }

      for &stmt in &line.stmts {
        self.compile_stmt(stmt);
      }

      if self.label().is_some() {
        self.code_emitter.end_label();
      }
    }

    self.resolve_labels();
    self.resolve_datum_indices();
  }

  fn resolve_labels(&mut self) {
    for (addr, range, label) in self.pending_jump_labels.drain(..) {
      let l = label.unwrap_or(Label(0));
      if let Some(label_addr) = self.code_emitter.find_label(l) {
        self.code_emitter.patch_jump_addr(addr, label_addr);
      } else {
        self.add_error(
          range,
          if label.is_some() {
            format!("行号不存在")
          } else {
            format!("行号 0 不存在（省略行号则默认行号是 0）")
          },
        );
      }
    }
  }

  fn resolve_datum_indices(&mut self) {
    for (addr, range, label) in self.pending_datum_indices.drain(..) {
      if let Some(&index) = self.data_start.get(&label) {
        self.code_emitter.patch_datum_index(addr, index);
      } else {
        self.add_warning(
          range,
          "行号不存在，RESTORE 语句将会把 DATA 指针重置到程序开头",
        );
      }
    }
  }

  fn compile_stmt(&mut self, stmt: StmtId) {
    macro_rules! compile_draw_stmt {
      (
        $stmt:expr,
        $args:expr,
        $kind:ident,
        $min_arity:literal,
        $max_arity:literal
      ) => {{
        for (i, &arg) in $args.iter().enumerate() {
          let ty = self.compile_expr(arg);
          if i < $max_arity {
            if !ty.matches(Type::Real) {
              let range = &self.expr_node(arg).range;
              self.add_error(
                range.clone(),
                format!("表达式类型错误。{} 语句的参数是{}类型，而这个表达式是{}类型",
                  stringify!($kind),
                  Type::Real,
                  ty));
            }
          }
        }

        if $args.len() < $min_arity {
          self.add_error(
            $stmt.range.clone(),
            concat!(
              stringify!($kind),
              " 语句至少要有 ",
              stringify!($min_arity),
              " 个参数"));
        } else if $args.len() > $max_arity {
          for &arg in &$args[$max_arity..] {
            let arg = &self.expr_node(arg);
            self.add_error(
              arg.range.clone(),
              concat!(
                "多余的参数。",
                stringify!($kind),
                " 语句最多允许有 ",
                stringify!($max_arity),
                " 个参数"));
          }
        } else {
          self.code_emitter.emit_op(
            $stmt.range.clone(),
            &$stmt.kind,
            $args.len());
        };
      }}
    }

    let stmt = &self.stmt_node(stmt);
    let range = stmt.range.clone();
    match &stmt.kind {
      StmtKind::Auto(_) => self.code_emitter.emit_no_op(range),
      StmtKind::Beep => self.code_emitter.emit_op(range, &stmt.kind, 0),
      StmtKind::Box(args) => {
        compile_draw_stmt!(&stmt, args, BOX, 4, 6);
      }
      StmtKind::Call(arg) => self.compile_unary_stmt(
        range,
        &stmt.kind,
        *arg,
        Type::Real,
        "CALL",
        "参数",
      ),
      StmtKind::Circle(args) => {
        compile_draw_stmt!(&stmt, args, CIRCLE, 3, 5);
      }
      StmtKind::Clear => self.code_emitter.emit_op(range, &stmt.kind, 0),
      StmtKind::Close { filenum } => self.compile_unary_stmt(
        range,
        &stmt.kind,
        *filenum,
        Type::Real,
        "CLOSE",
        "文件号",
      ),
      StmtKind::Cls => self.code_emitter.emit_op(range, &stmt.kind, 0),
      StmtKind::Cont => self.code_emitter.emit_op(range, &stmt.kind, 0),
      StmtKind::Copy(_) => self.code_emitter.emit_no_op(range),
      StmtKind::Data(data) => self.compile_data(data),
      StmtKind::Def { name, param, body } => {
        self.compile_def(range, name, param, *body)
      }
      StmtKind::Del(_) => self.code_emitter.emit_no_op(range),
      StmtKind::Dim(vars) => self.compile_dim(vars),
      StmtKind::Draw(args) => {
        compile_draw_stmt!(&stmt, args, DRAW, 2, 3);
      }
      StmtKind::Edit(_) => self.code_emitter.emit_no_op(range),
      StmtKind::Ellipse(args) => {
        compile_draw_stmt!(&stmt, args, ELLIPSE, 4, 6);
      }
      StmtKind::End => self.code_emitter.emit_op(range, &stmt.kind, 0),
      StmtKind::Field { filenum, fields } => {
        self.compile_field(range, *filenum, fields)
      }
      StmtKind::Files(_) => self.code_emitter.emit_no_op(range),
      StmtKind::Flash => self.code_emitter.emit_op(range, &stmt.kind, 0),
      StmtKind::For {
        var,
        start,
        end,
        step,
      } => self.compile_for(range, var, *start, *end, *step),
      StmtKind::Get { filenum, record } => {
        self.compile_get_put(range, &stmt.kind, *filenum, *record, "GET")
      }
      StmtKind::GoSub(label) => {
        self.compile_go(range, label, true);
      }
      StmtKind::GoTo { label, .. } => {
        self.compile_go(range, label, false);
      }
      StmtKind::Graph => self.code_emitter.emit_op(range, &stmt.kind, 0),
      StmtKind::If { cond, conseq, alt } => {
        self.compile_if(range, *cond, conseq, alt)
      }
      StmtKind::InKey => self.code_emitter.emit_op(range, &stmt.kind, 0),
      StmtKind::Input { source, vars } => {
        self.compile_input(range, source, vars)
      }
      StmtKind::Inverse => self.code_emitter.emit_op(range, &stmt.kind, 0),
      StmtKind::Kill(_) => self.code_emitter.emit_no_op(range),
      StmtKind::Let { var, value } => {
        let (_, _) = self.compile_lvalue(*var);
        let _ = self.compile_expr(*value);
        self.code_emitter.emit_op(range, &stmt.kind, 2);
      }
      StmtKind::Line(args) => compile_draw_stmt!(&stmt, args, LINE, 4, 5),
      StmtKind::List(_) => self.code_emitter.emit_no_op(range),
      StmtKind::Load(_) => self.code_emitter.emit_no_op(range),
      StmtKind::Locate { row, column } => self.compile_locate(*row, *column),
      StmtKind::LSet { var, value } => {
        self.compile_set(range, &stmt.kind, *var, *value, "LSET")
      }
      StmtKind::New(_) => self.code_emitter.emit_no_op(range),
      StmtKind::Next { vars } => self.compile_next(range, vars),
      StmtKind::Normal => self.code_emitter.emit_op(range, &stmt.kind, 0),
      StmtKind::NoTrace => self.code_emitter.emit_op(range, &stmt.kind, 0),
      StmtKind::On {
        cond,
        labels,
        is_sub,
      } => {
        self.compile_on(range, *cond, labels, *is_sub);
      }
      StmtKind::Open {
        filename,
        mode,
        filenum,
        len,
      } => self.compile_open(range, *filename, *mode, *filenum, *len),
      StmtKind::Play(arg) => self.compile_unary_stmt(
        range,
        &stmt.kind,
        *arg,
        Type::String,
        "PLAY",
        "参数",
      ),
      StmtKind::Poke { addr, value } => {
        self.compile_poke(range, &stmt.kind, *addr, *value)
      }
      StmtKind::Pop => self.code_emitter.emit_op(range, &stmt.kind, 0),
      StmtKind::Print(elems) => self.compile_print(range, elems),
      StmtKind::Put { filenum, record } => {
        self.compile_get_put(range, &stmt.kind, *filenum, *record, "PUT")
      }
      StmtKind::Read(vars) => self.compile_read(range, vars),
      StmtKind::Rem(_) => self.code_emitter.emit_no_op(range),
      StmtKind::Rename(_) => self.code_emitter.emit_no_op(range),
      StmtKind::Restore(label) => self.compile_restore(range, label),
      StmtKind::Return => self.code_emitter.emit_op(range, &stmt.kind, 0),
      StmtKind::RSet { var, value } => {
        self.compile_set(range, &stmt.kind, *var, *value, "RSET")
      }
      StmtKind::Run => self.code_emitter.emit_op(range, &stmt.kind, 0),
      StmtKind::Save(_) => self.code_emitter.emit_no_op(range),
      StmtKind::Stop(_) => self.code_emitter.emit_no_op(range),
      StmtKind::Swap { left, right } => {
        let (_, ty1) = self.compile_lvalue(*left);
        let (_, ty2) = self.compile_lvalue(*right);
        if !ty1.exact_matches(ty2) {
          self
            .add_error(range.clone(), "SWAP 语句的两个变量/数组的类型必须相等");
        }
        self.code_emitter.emit_op(range, &stmt.kind, 2);
      }
      StmtKind::System => {
        self.add_error(range, "该语句已废弃");
      }
      StmtKind::Text => self.code_emitter.emit_op(range, &stmt.kind, 0),
      StmtKind::Trace => self.code_emitter.emit_op(range, &stmt.kind, 0),
      StmtKind::Wend => self.code_emitter.emit_op(range, &stmt.kind, 0),
      StmtKind::While(cond) => self.compile_unary_stmt(
        range,
        &stmt.kind,
        *cond,
        Type::Real,
        "WHILE",
        "条件",
      ),
      StmtKind::Write { filenum, data } => {
        self.compile_write(range, *filenum, data)
      }
      StmtKind::NoOp => self.code_emitter.emit_no_op(range),
    }
  }

  fn compile_unary_stmt(
    &mut self,
    range: Range,
    kind: &StmtKind,
    arg: ExprId,
    arg_type: Type,
    stmt_name: &str,
    param_name: &str,
  ) {
    let ty = self.compile_expr(arg);
    if !ty.matches(arg_type) {
      let range = &self.expr_node(arg).range;
      self.add_error(
        range.clone(),
        format!(
          "表达式类型错误。{} 语句的{}是{}类型，而这个表达式是{}类型",
          stmt_name, param_name, arg_type, ty
        ),
      );
    }
    self.code_emitter.emit_op(range, kind, 1);
  }

  fn compile_data(&mut self, data: &NonEmptyVec<[Datum; 1]>) {
    let mut data_index = None;
    for datum in data.iter() {
      let text = &self.text[datum.range.start..datum.range.end];
      let text = if datum.is_quoted {
        if text.ends_with('"') {
          text[1..text.len() - 1].to_owned()
        } else {
          text[1..].to_owned()
        }
      } else {
        text.to_owned()
      };
      let index = self.code_emitter.emit_datum(
        datum.range.clone(),
        text,
        datum.is_quoted,
      );
      if data_index.is_none() {
        data_index = Some(index);
      }
    }

    if let Some(label) = self.label() {
      self.data_start.entry(label).or_insert(data_index.unwrap());
    }
  }

  fn compile_def(
    &mut self,
    range: Range,
    name: &Option<Range>,
    param: &Option<Range>,
    body: ExprId,
  ) {
    let name = name.map(|name_range| {
      let (name, ty) = self.compile_sym(name_range);
      if !ty.matches(Type::Real) {
        self.add_error(
          name_range.clone(),
          format!("变量类型错误。自定义函数的类型必须是{}类型", Type::Real),
        );
      }
      name
    });

    let param = param.map(|param_range| {
      let (param, ty) = self.compile_sym(param_range);
      if !ty.matches(Type::Real) {
        self.add_error(
          param_range.clone(),
          format!(
            "参数类型错误。自定义函数的参数的类型必须是{}类型",
            Type::Real
          ),
        );
      }
      param
    });

    if let (Some(name), Some(param)) = (name, param) {
      self.code_emitter.begin_def_fn(range, name, param);
      self.compile_expr(body);
      self.code_emitter.end_def_fn();
    } else {
      self.compile_expr(body);
    }
  }

  fn compile_dim(&mut self, vars: &NonEmptyVec<[ExprId; 1]>) {
    for &var in vars.iter() {
      let var = &self.expr_node(var);
      if let ExprKind::Index { name, indices } = &var.kind {
        for &index in indices.iter() {
          let ty = self.compile_expr(index);
        }
        if let Some(name_range) = name {
          let (name, _) = self.compile_sym(name_range.clone());
          self
            .code_emitter
            .emit_dim(name_range.clone(), name, unsafe {
              NonZeroUsize::new_unchecked(indices.len())
            });
        }
      }
    }
  }

  fn compile_field(
    &mut self,
    range: Range,
    filenum: ExprId,
    fields: &[FieldSpec],
  ) {
    let ty = self.compile_expr(filenum);
    if !ty.matches(Type::Real) {
      let range = &self.expr_node(filenum).range;
      self.add_error(
        range.clone(),
        format!(
          "表达式类型错误。FIELD 语句的文件号必须是{}类型，而这个表达式是{}类型",
          Type::Real,
          ty));
    }

    for field in fields {
      let ty = self.compile_expr(field.len);
      if !ty.matches(Type::Real) {
        let range = &self.expr_node(field.len).range;
        self.add_error(
          range.clone(),
          format!(
            "表达式类型错误。FIELD 语句的字段长度必须是{}类型，而这个表达式是{}类型",
            Type::Real,
            ty));
      }

      let (is_array, ty) = self.compile_lvalue(field.var);
      if !ty.matches(Type::Real) {
        let range = &self.expr_node(field.var).range;
        self.add_error(
          range.clone(),
          format!(
            "字段类型错误。FIELD 语句的字段必须是{}类型，而这个{}是{}类型",
            Type::Real,
            if is_array { "数组" } else { "变量" },
            ty
          ),
        );
      }
    }

    self
      .code_emitter
      .emit_field(range, unsafe { NonZeroUsize::new_unchecked(fields.len()) });
  }

  fn compile_get_put(
    &mut self,
    range: Range,
    kind: &StmtKind,
    filenum: ExprId,
    record: ExprId,
    name: &str,
  ) {
    let ty = self.compile_expr(filenum);
    if !ty.matches(Type::Real) {
      let range = &self.expr_node(filenum).range;
      self.add_error(
        range.clone(),
        format!(
          "表达式类型错误。{} 语句的文件号参数是{}类型，而这个表达式是{}类型",
          name,
          Type::Real,
          ty
        ),
      );
    }

    let ty = self.compile_expr(record);
    if !ty.matches(Type::Real) {
      let range = &self.expr_node(record).range;
      self.add_error(
        range.clone(),
        format!(
          "表达式类型错误。{} 语句的记录号参数是{}类型，而这个表达式是{}类型",
          name,
          Type::Real,
          ty
        ),
      );
    }

    self.code_emitter.emit_op(range, kind, 2);
  }

  fn compile_read(&mut self, range: Range, vars: &NonEmptyVec<[ExprId; 1]>) {
    for &var in vars.iter() {
      self.compile_lvalue(var);
      let range = self.expr_node(var).range.clone();
      self.code_emitter.emit_read(range);
    }
  }

  fn compile_restore(&mut self, range: Range, label: &Option<(Range, Label)>) {
    let addr = self.code_emitter.emit_restore(range);
    if let Some((range, label)) = label {
      self
        .pending_datum_indices
        .push((addr, range.clone(), *label));
    }
  }

  fn compile_set(
    &mut self,
    range: Range,
    kind: &StmtKind,
    var: ExprId,
    value: ExprId,
    stmt_name: &str,
  ) {
    let (is_array, ty) = self.compile_lvalue(var);
    if !ty.matches(Type::String) {
      let range = &self.expr_node(var).range;
      self.add_error(
        range.clone(),
        format!(
          "字段类型错误。{} 语句的字段必须是{}类型，而这个{}是{}类型",
          stmt_name,
          Type::String,
          if is_array { "数组" } else { "变量" },
          ty
        ),
      );
    }

    let ty = self.compile_expr(value);
    if !ty.matches(Type::String) {
      let range = &self.expr_node(value).range;
      self.add_error(
        range.clone(),
        format!(
          "表达式类型错误。{} 语句的值参数必须是{}类型，而这个表达式是{}类型",
          stmt_name,
          Type::String,
          ty
        ),
      );
    }

    self.code_emitter.emit_op(range, kind, 2);
  }

  fn compile_write(
    &mut self,
    range: Range,
    filenum: Option<ExprId>,
    data: &NonEmptyVec<[WriteElement; 1]>,
  ) {
    if let Some(filenum) = filenum {
      let ty = self.compile_expr(filenum);
      if !ty.matches(Type::Real) {
        let range = &self.expr_node(filenum).range;
        self.add_error(
          range.clone(),
          format!(
            "表达式类型错误。WRITE 语句的文件号必须是{}类型，而这个表达式是{}类型",
            Type::Real,
            ty));
      }
    }

    let to_file = filenum.is_some();

    for (i, datum) in data.iter().enumerate() {
      self.compile_expr(datum.datum);
      if i < data.len() - 1 && !datum.comma {
        let range = self.expr_node(datum.datum).range.clone();
        self.add_warning(
          range,
          "这个值会被 WRITE 语句忽略，请在表达式末尾加上逗号",
        );
      }

      let range = self.expr_node(datum.datum).range.clone();
      if i == data.len() - 1 {
        self.code_emitter.emit_write_end(range, to_file);
      } else if datum.comma {
        self.code_emitter.emit_write(range, to_file);
      } else {
        self.code_emitter.emit_pop();
      }
    }
  }

  fn compile_for(
    &mut self,
    range: Range,
    var: &Option<Range>,
    start: ExprId,
    end: ExprId,
    step: Option<ExprId>,
  ) {
    let var = if let Some(var_range) = var {
      let (var, ty) = self.compile_sym(var_range.clone());
      if !ty.matches(Type::Real) {
        self.add_error(
              var_range.clone(),
              format!(
                "变量类型错误。FOR 语句的计数器变量必须是{}类型，而这个变量是{}类型",
                Type::Real,
                ty));
      }
      Some(var)
    } else {
      None
    };

    let ty = self.compile_expr(start);
    if !ty.matches(Type::Real) {
      let range = &self.expr_node(start).range;
      self.add_error(
        range.clone(),
        format!(
          "表达式类型错误。FOR 语句的计数器初始值必须是{}类型，而这个表达式是{}类型",
          Type::Real,
          ty));
    }

    let ty = self.compile_expr(end);
    if !ty.matches(Type::Real) {
      let range = &self.expr_node(end).range;
      self.add_error(
              range.clone(),
              format!(
                "表达式类型错误。FOR 语句的计数器终止值必须是{}类型，而这个表达式是{}类型",
                Type::Real,
                ty));
    }

    if let Some(step) = step {
      let ty = self.compile_expr(step);
      if !ty.matches(Type::Real) {
        let range = &self.expr_node(step).range;
        self.add_error(
              range.clone(),
              format!(
                "表达式类型错误。FOR 语句的计数器步长必须是{}类型，而这个表达式是{}类型",
                Type::Real,
                ty));
      }
    }

    if let Some(var) = var {
      self.code_emitter.emit_for(range, var, step.is_some());
    }
  }

  fn compile_go(
    &mut self,
    range: Range,
    label: &Option<(Range, Label)>,
    is_sub: bool,
  ) {
    let addr = if is_sub {
      self.code_emitter.emit_gosub(range.clone())
    } else {
      self.code_emitter.emit_goto(range.clone())
    };

    if let Some((range, label)) = label {
      self
        .pending_jump_labels
        .push((addr, range.clone(), Some(*label)));
    } else {
      self.pending_jump_labels.push((addr, range, None));
    }
  }

  fn compile_if(
    &mut self,
    range: Range,
    cond: ExprId,
    conseq: &SmallVec<[StmtId; 1]>,
    alt: &Option<SmallVec<[StmtId; 1]>>,
  ) {
    let ty = self.compile_expr(cond);
    if !ty.matches(Type::Real) {
      let range = &self.expr_node(cond).range;
      self.add_error(
        range.clone(),
        format!(
          "表达式类型错误。IF 语句的条件必须是{}类型，而这个表达式是{}类型",
          Type::Real,
          ty
        ),
      );
    }

    let je_addr = self.code_emitter.emit_je(range.clone());

    for &stmt in conseq.iter() {
      self.compile_stmt(stmt);
    }

    let conseq_end_addr = self.code_emitter.emit_goto(range);
    let alt_addr = self.code_emitter.current_addr();
    self.code_emitter.patch_jump_addr(je_addr, alt_addr);

    if let Some(alt) = alt {
      for &stmt in alt.iter() {
        self.compile_stmt(stmt);
      }
    }

    let end_addr = self.code_emitter.current_addr();
    self.code_emitter.patch_jump_addr(conseq_end_addr, end_addr);
  }

  fn compile_input(
    &mut self,
    range: Range,
    source: &InputSource,
    vars: &NonEmptyVec<[ExprId; 1]>,
  ) {
    let is_file = matches!(source, InputSource::File(_));
    for &var in vars.iter() {
      if let ExprKind::UserFuncCall { func, arg } = &self.expr_node(var).kind {
        let range = self.expr_node(var).range.clone();
        if is_file {
          self.add_error(
            range,
            "INPUT 语句只允许从键盘输入自定义函数，不能从文件输入自定义函数",
          );
        } else {
          let func = func.map(|func_range| {
            let (func, ty) = self.compile_sym(func_range.clone());
            if !ty.matches(Type::Real) {
              self.add_error(
                func_range.clone(),
                format!(
                  "变量类型错误。自定义函数的参数必须是{}类型",
                  Type::Real
                ),
              );
            }
            func
          });
          let arg_range = self.expr_node(*arg).range.clone();
          let arg = if let ExprKind::Ident = &self.expr_node(*arg).kind {
            let (arg, ty) = self.compile_sym(arg_range.clone());
            if !ty.matches(Type::Real) {
              self.add_error(
                arg_range,
                format!(
                  "变量类型错误。自定义函数的参数必须是{}类型",
                  Type::Real
                ),
              );
            }
            Some(arg)
          } else {
            self.add_error(
              arg_range,
              "INPUT 语句中的自定义函数的参数只允许是变量",
            );
            None
          };
          if let (Some(func), Some(arg)) = (func, arg) {
            self.code_emitter.emit_fn_lvalue(range, func, arg);
          }
        }
      } else {
        self.compile_lvalue(var);
      }
    }
    match source {
      InputSource::Keyboard(prompt) => {
        self.code_emitter.emit_keyboard_input(
          range,
          prompt.map(|p| {
            let mut text = &self.text[p.start + 1..p.end];
            if text.ends_with('"') {
              text = &text[..text.len() - 1];
            }
            text.to_owned()
          }),
          unsafe { NonZeroUsize::new_unchecked(vars.len()) },
        );
      }
      InputSource::File(filenum) => {
        let ty = self.compile_expr(*filenum);
        if !ty.matches(Type::Real) {
          let range = &self.expr_node(*filenum).range;
          self.add_error(
            range.clone(),
            format!(
              "表达式类型错误。INPUT 语句的文件号必须是{}类型，而这个表达式是{}类型",
              Type::Real,
              ty
            ),
          );
        }
      }
      InputSource::Error => {
        // do nothing
      }
    }
  }

  fn compile_locate(&mut self, row: Option<ExprId>, column: Option<ExprId>) {
    if let Some(row) = row {
      let ty = self.compile_expr(row);
      let range = &self.expr_node(row).range;
      if !ty.matches(Type::Real) {
        self.add_error(
          range.clone(),
          format!(
            "表达式类型错误。LOCATE 语句的行参数必须是{}类型，而这个表达式是{}类型",
            Type::Real,
            ty
          ),
        );
      }
      self.code_emitter.emit_set_row(range.clone());
    }

    if let Some(column) = column {
      let ty = self.compile_expr(column);
      let range = &self.expr_node(column).range;
      if !ty.matches(Type::Real) {
        self.add_error(
          range.clone(),
          format!(
            "表达式类型错误。LOCATE 语句的列参数必须是{}类型，而这个表达式是{}类型",
            Type::Real,
            ty
          ),
        );
      }
      self.code_emitter.emit_set_column(range.clone());
    }
  }

  fn compile_on(
    &mut self,
    range: Range,
    cond: ExprId,
    labels: &NonEmptyVec<[(Range, Option<Label>); 2]>,
    is_sub: bool,
  ) {
    let ty = self.compile_expr(cond);
    if !ty.matches(Type::Real) {
      let range = &self.expr_node(cond).range;
      self.add_error(
        range.clone(),
        format!(
          "表达式类型错误。ON ... {} 语句的条件必须是{}类型，而这个表达式是{}类型",
          if is_sub { "GOSUB" } else { "GOTO" },
          Type::Real,
          ty
        ),
      );
    }

    for (range, label) in labels.iter() {
      let addr = if is_sub {
        self.code_emitter.emit_gosub(range.clone())
      } else {
        self.code_emitter.emit_goto(range.clone())
      };
      self.pending_jump_labels.push((addr, range.clone(), *label));
    }

    self
      .code_emitter
      .emit_on(range, unsafe { NonZeroUsize::new_unchecked(labels.len()) });
  }

  fn compile_open(
    &mut self,
    range: Range,
    filename: ExprId,
    mode: FileMode,
    filenum: ExprId,
    len: Option<ExprId>,
  ) {
    let ty = self.compile_expr(filename);
    if !ty.matches(Type::String) {
      let range = &self.expr_node(filename).range;
      self.add_error(
        range.clone(),
        format!(
          "表达式类型错误。OPEN 语句的文件名必须是{}类型，而这个表达式是{}类型",
          Type::String,
          ty
        ),
      );
    }

    let ty = self.compile_expr(filenum);
    if !ty.matches(Type::Real) {
      let range = &self.expr_node(filenum).range;
      self.add_error(
        range.clone(),
        format!(
          "表达式类型错误。OPEN 语句的文件号必须是{}类型，而这个表达式是{}类型",
          Type::Real,
          ty
        ),
      );
    }

    if let Some(len) = len {
      let range = &self.expr_node(len).range;
      if !matches!(mode, FileMode::Random | FileMode::Error) {
        self
          .add_error(range.clone(), "LEN 参数只能用于以 RANDOM 模式打开的文件")
      }
      let ty = self.compile_expr(len);
      if !ty.matches(Type::Real) {
        self.add_error(
          range.clone(),
          format!(
            "表达式类型错误。OPEN 语句的记录长度必须是{}类型，而这个表达式是{}类型",
            Type::Real,
            ty
          ),
        );
      }
    }

    if !matches!(mode, FileMode::Error) {
      self.code_emitter.emit_open(mode, len.is_some());
    }
  }

  fn compile_next(
    &mut self,
    range: Range,
    vars: &SmallVec<[Option<Range>; 1]>,
  ) {
    if vars.is_empty() {
      self.code_emitter.emit_next(range, None);
    } else {
      for var in vars.iter() {
        if let Some(var_range) = var {
          let (var, ty) = self.compile_sym(var_range.clone());
          if !ty.matches(Type::Real) {
            self.add_error(
                  var_range.clone(),
                  format!(
                    "变量类型错误。NEXT 语句的计数器变量必须是{}类型，而这个变量是{}类型",
                    Type::Real,
                    ty));
          }
          self.code_emitter.emit_next(var_range.clone(), Some(var));
        }
      }
    }
  }

  fn compile_poke(
    &mut self,
    range: Range,
    kind: &StmtKind,
    addr: ExprId,
    value: ExprId,
  ) {
    let ty = self.compile_expr(addr);
    if !ty.matches(Type::Real) {
      let range = &self.expr_node(addr).range;
      self.add_error(
        range.clone(),
        format!(
          "表达式类型错误。POKE 语句的地址参数是{}类型，而这个表达式是{}类型",
          Type::Real,
          ty
        ),
      );
    }

    let ty = self.compile_expr(value);
    if !ty.matches(Type::Real) {
      let range = &self.expr_node(value).range;
      self.add_error(
        range.clone(),
        format!(
          "表达式类型错误。POKE 语句的值参数是{}类型，而这个表达式是{}类型",
          Type::Real,
          ty
        ),
      );
    }

    self.code_emitter.emit_op(range, kind, 2);
  }

  fn compile_print(
    &mut self,
    range: Range,
    elems: &SmallVec<[PrintElement; 2]>,
  ) {
    for (i, elem) in elems.iter().enumerate() {
      match elem {
        PrintElement::Semicolon(_) => {
          // do nothing
        }
        PrintElement::Comma(elem_range) => {
          self.code_emitter.emit_print_newline(elem_range.clone());
        }
        PrintElement::Expr(expr) => match &self.expr_node(*expr).kind {
          ExprKind::SysFuncCall {
            func: (_, kind @ (SysFuncKind::Spc | SysFuncKind::Tab)),
            args,
          } => {
            if args.len() > 1 {
              for &arg in &args[1..] {
                let arg = &self.expr_node(arg);
                self.add_error(
                  arg.range.clone(),
                  format!("多余的参数。{:?} 函数只接受 1 个参数", kind),
                );
              }
            }

            for (i, &arg) in args.iter().enumerate() {
              let ty = self.compile_expr(arg);
              if i == 0 {
                if !ty.matches(Type::Real) {
                  let range = &self.expr_node(arg).range;
                  self.add_error(
                    range.clone(),
                    format!(
                      "表达式类型错误。{:?} 函数的参数必须是{}类型，而这个表达式是{}类型",
                      kind,
                      Type::Real,
                      ty
                    ),
                  );
                }
              }
            }

            let expr_range = self.expr_node(*expr).range.clone();
            match kind {
              SysFuncKind::Spc => self.code_emitter.emit_print_spc(expr_range),
              SysFuncKind::Tab => self.code_emitter.emit_print_tab(expr_range),
              _ => unreachable!(),
            }

            if i == elems.len() - 1 {
              self.code_emitter.emit_print_newline(range.clone());
            }
          }
          _ => {
            self.compile_expr(*expr);
            let elem_range = &self.expr_node(*expr).range;
            self.code_emitter.emit_print_value(elem_range.clone());
            if i == elems.len() - 1 {
              self.code_emitter.emit_print_newline(range.clone());
            } else if matches!(&elems[i + 1], PrintElement::Expr(_)) {
              self.code_emitter.emit_number(
                elem_range.clone(),
                Mbf5::try_from(Mbf5Accum::try_from(0.0).unwrap()).unwrap(),
              );
              self.code_emitter.emit_print_spc(elem_range.clone());
            }
          }
        },
      }
    }
  }

  fn compile_expr(&mut self, expr: ExprId) -> Type {
    let expr = self.expr_node(expr);
    let range = expr.range.clone();
    match &expr.kind {
      ExprKind::Ident => {
        let (sym, ty) = self.compile_sym(range.clone());
        self.code_emitter.emit_var(range, sym);
        ty
      }
      ExprKind::StringLit => {
        let mut text = &self.text[range.start + 1..range.end];
        if text.ends_with('"') {
          text = &text[..text.len() - 1];
        }
        self.code_emitter.emit_string(range, text.to_owned());
        Type::String
      }
      ExprKind::NumberLit => {
        let mut text = self.text[range.start + 1..range.end].to_owned();
        text.retain(|c| c != ' ');
        match text.parse::<Mbf5>() {
          Ok(num) => self.code_emitter.emit_number(range, num),
          Err(FloatError::Infinite) => self.add_error(range, "数值溢出"),
          Err(_) => unreachable!(),
        }
        Type::String
      }
      ExprKind::SysFuncCall {
        func: (func_range, kind),
        args,
      } => {}
      ExprKind::UserFuncCall { func, arg } => {}
      ExprKind::Binary { lhs, op, rhs } => {
        let lhs_ty = self.compile_expr(*lhs);
        let rhs_ty = self.compile_expr(*rhs);

        match op.1 {
          BinaryOpKind::Eq
          | BinaryOpKind::Ne
          | BinaryOpKind::Gt
          | BinaryOpKind::Lt
          | BinaryOpKind::Ge
          | BinaryOpKind::Le
          | BinaryOpKind::Add => {
          }
          BinaryOpKind::Sub
          | BinaryOpKind::Mul
          | BinaryOpKind::Div
          | BinaryOpKind::Pow
          | BinaryOpKind::And
          | BinaryOpKind::Or => {}
        }
      }
      ExprKind::Unary { op, arg } => {
        let ty = self.compile_expr(*arg);
        if !ty.matches(Type::Real) {
          let range = &self.expr_node(*arg).range;
          self.add_error(
            range.clone(),
            format!("表达式类型错误。必须是{}类型", Type::Real),
          );
        }

        self.code_emitter.emit_unary_expr(range, op.1);
        Type::Real
      }
      ExprKind::Index { name, indices } => {
        for &index in indices.iter() {
          let ty = self.compile_expr(index);
          if !ty.matches(Type::Real) {
            let range = &self.expr_node(index).range;
            self.add_error(
              range.clone(),
              format!(
                "表达式类型错误。数组下标必须是{}类型，而这个表达式是{}类型",
                Type::Real,
                ty
              ),
            );
          }
        }

        if let Some(name) = name {
          let (name, ty) = self.compile_sym(name.clone());
          self.code_emitter.emit_index(range, name, unsafe {
            NonZeroUsize::new_unchecked(indices.len())
          });
          ty
        } else {
          Type::Error
        }
      }
      ExprKind::Inkey => {
        self.code_emitter.emit_inkey(range);
        Type::String
      }
      ExprKind::Error => Type::Error,
    }
  }

  fn compile_lvalue(&mut self, lvalue: ExprId) -> (bool, Type) {
    let lvalue = &self.expr_node(lvalue);
    match &lvalue.kind {
      ExprKind::Ident => {
        let (var, ty) = self.compile_sym(lvalue.range.clone());
        self.code_emitter.emit_lvalue(lvalue.range.clone(), var, 0);
        (false, ty)
      }
      ExprKind::Index { name, indices } => {
        for &index in indices.iter() {
          let ty = self.compile_expr(index);
          if !ty.matches(Type::Real) {
            let range = &self.expr_node(index).range;
            self.add_error(
              range.clone(),
              format!(
                "表达式类型错误。数组下标必须是{}类型，而这个表达式是{}类型",
                Type::Real,
                ty
              ),
            );
          }
        }

        if let Some(name_range) = name {
          let (name, ty) = self.compile_sym(name_range.clone());
          self.code_emitter.emit_lvalue(
            name_range.clone(),
            name,
            indices.len(),
          );
          (true, ty)
        } else {
          (true, Type::Error)
        }
      }
      _ => unreachable!(),
    }
  }

  fn compile_sym(&mut self, range: Range) -> (E::Symbol, Type) {
    let name = self.text[range.start..range.end].to_ascii_uppercase();
    let ty = match name.as_bytes().last() {
      Some(b'%') => Type::Integer,
      Some(b'$') => Type::String,
      _ => Type::Real,
    };

    if let Some(i) = name.find(' ') {
      name.split_off(i);
      if !ty.exact_matches(Type::Real) {
        name.push(ty.sigil().unwrap());
      }
      self.add_warning(
        range.clone(),
        format!(
          "该变量包含空格，空格之后的部分会被省略。该变量等价于 {}",
          name
        ),
      );
    }

    let sym = self.code_emitter.make_symbol(name);
    (sym, ty)
  }
}

impl Type {
  fn matches(self, other: Type) -> bool {
    match (self, other) {
      (Self::Error, _) => true,
      (_, Self::Error) => true,
      (Self::Real | Self::Integer, Self::Real | Self::Integer) => true,
      (Self::String, Self::String) => true,
      _ => false,
    }
  }

  fn exact_matches(self, other: Type) -> bool {
    self == other
  }

  fn sigil(self) -> Option<char> {
    match self {
      Self::Integer => Some('%'),
      Self::Real => None,
      Self::String => Some('$'),
      Self::Error => None,
    }
  }
}

impl Display for Type {
  fn fmt(&self, f: &mut Formatter) -> fmt::Result {
    match self {
      Self::Integer => write!(f, "整数"),
      Self::Real => write!(f, "数值"),
      Self::String => write!(f, "字符串"),
      Self::Error => unreachable!(),
    }
  }
}
