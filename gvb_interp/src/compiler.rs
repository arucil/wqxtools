use crate::parser::ParseResult;
use crate::util::mbf5::{Mbf5, ParseRealError};
use crate::{ast::*, diagnostic::*, HashMap};
use smallvec::SmallVec;
use std::fmt::{self, Display, Formatter};
use std::num::NonZeroUsize;

pub trait CodeEmitter {
  type Symbol: Copy;
  type Addr: Copy;
  type DatumIndex: Copy;

  fn begin_line(&mut self, linenum: usize);

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
  ) -> Self::Addr;
  fn end_def_fn(&mut self, def_addr: Self::Addr);

  fn emit_dim(
    &mut self,
    range: Range,
    name: Self::Symbol,
    dimensions: NonZeroUsize,
  );

  fn emit_var_lvalue(&mut self, range: Range, name: Self::Symbol);

  fn emit_index_lvalue(
    &mut self,
    range: Range,
    name: Self::Symbol,
    dimensions: NonZeroUsize,
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

  fn emit_assign_num(&mut self, range: Range);
  fn emit_assign_str(&mut self, range: Range);

  fn make_symbol(&mut self, name: String) -> Self::Symbol;

  fn emit_gosub(&mut self, range: Range) -> Self::Addr;

  fn emit_goto(&mut self, range: Range) -> Self::Addr;

  fn patch_jump_addr(&mut self, addr: Self::Addr, label_addr: Self::Addr);

  fn emit_jz(&mut self, range: Range) -> Self::Addr;

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

  fn emit_file_input(&mut self, range: Range, fields: NonZeroUsize);

  fn emit_open(&mut self, range: Range, mode: FileMode, has_len: bool);

  fn emit_read(&mut self, range: Range);

  fn emit_newline(&mut self, range: Range);
  fn emit_print_spc(&mut self, range: Range);
  fn emit_print_tab(&mut self, range: Range);
  fn emit_print_num(&mut self, range: Range);
  fn emit_print_str(&mut self, range: Range);
  fn emit_flush(&mut self, range: Range);

  fn emit_pop_num(&mut self, range: Range);
  fn emit_pop_str(&mut self, range: Range);

  fn emit_write_num(&mut self, range: Range, to_file: bool, end: bool);
  fn emit_write_str(&mut self, range: Range, to_file: bool, end: bool);

  fn emit_while(&mut self, range: Range, cond_start: Self::Addr);

  fn emit_number(&mut self, range: Range, num: Mbf5);
  fn emit_var(&mut self, range: Range, sym: Self::Symbol);
  /// Returns if string is too long.
  #[must_use]
  fn emit_string(&mut self, range: Range, str: String) -> bool;
  fn emit_inkey(&mut self, range: Range);
  fn emit_index(
    &mut self,
    range: Range,
    name: Self::Symbol,
    dimensions: NonZeroUsize,
  );
  fn emit_unary_expr(&mut self, range: Range, kind: UnaryOpKind);
  fn emit_num_binary_expr(&mut self, range: Range, kind: BinaryOpKind);
  fn emit_str_binary_expr(&mut self, range: Range, kind: BinaryOpKind);
  fn emit_user_func_call(&mut self, range: Range, name: Self::Symbol);
  fn emit_sys_func_call(
    &mut self,
    range: Range,
    kind: SysFuncKind,
    arity: NonZeroUsize,
  );

  fn clean_up(&mut self) -> Vec<(usize, Diagnostic)>;
}

pub fn compile_prog<E: CodeEmitter>(
  text: impl AsRef<str>,
  prog: &mut Program,
  code_emitter: &mut E,
) {
  let text = text.as_ref();
  let mut state = CompileState {
    text: "",
    code_emitter,
    pending_jump_labels: vec![],
    pending_datum_indices: vec![],
    data_start: HashMap::default(),
    label_addrs: HashMap::default(),
    parsed: std::ptr::null_mut(),
    linenum: 0,
  };

  state.compile_prog(text, prog);
}

pub(crate) fn compile_fn_body<E: CodeEmitter>(
  text: impl AsRef<str>,
  expr: &mut ParseResult<ExprId>,
  code_emitter: &mut E,
) {
  let text = text.as_ref();
  let mut state = CompileState {
    text,
    code_emitter,
    pending_jump_labels: vec![],
    pending_datum_indices: vec![],
    data_start: HashMap::default(),
    label_addrs: HashMap::default(),
    parsed: expr as *mut _,
    linenum: 0,
  };

  let ty = state.compile_expr(expr.content);
  if !ty.matches(Type::Real) {
    let range = &state.expr_node(expr.content).range;
    state.add_error(
      range.clone(),
      format!(
        "表达式类型错误。自定义函数的函数体表达式必须是{}类型，而这个表达式是{}类型",
        Type::Real, ty
      ),
    );
  }
}

struct CompileState<'a, 'b, E: CodeEmitter, T> {
  text: &'b str,
  code_emitter: &'a mut E,
  pending_jump_labels: Vec<(E::Addr, (usize, Range), Option<Label>)>,
  pending_datum_indices: Vec<(E::Addr, (usize, Range), Label)>,
  data_start: HashMap<Label, E::DatumIndex>,
  label_addrs: HashMap<Label, E::Addr>,
  parsed: *mut ParseResult<T>,
  linenum: usize,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Type {
  Integer,
  Real,
  String,
  Error,
}

impl<'a, 'b, E: CodeEmitter, T> CompileState<'a, 'b, E, T> {
  fn add_error(&mut self, range: Range, message: impl ToString) {
    unsafe { &mut *self.parsed }
      .diagnostics
      .push(Diagnostic::new_error(range, message));
  }

  fn add_warning(&mut self, range: Range, message: impl ToString) {
    unsafe { &mut *self.parsed }
      .diagnostics
      .push(Diagnostic::new_warning(range, message));
  }

  fn expr_node(&self, expr: ExprId) -> &'a Expr {
    unsafe { &(*self.parsed).expr_arena[expr] }
  }

  fn stmt_node(&self, stmt: StmtId) -> &'a Stmt {
    unsafe { &(*self.parsed).stmt_arena[stmt] }
  }
}

impl<'a, 'b, E: CodeEmitter> CompileState<'a, 'b, E, ProgramLine> {
  fn label(&self) -> Option<Label> {
    unsafe { (*self.parsed).content.label.as_ref().map(|x| x.1) }
  }

  fn compile_prog(&mut self, text: &'b str, prog: &mut Program) {
    let mut last_label = -1;
    let mut text_offset = 0;

    for (i, line) in prog.lines.iter_mut().enumerate() {
      self.text = &text[text_offset..text_offset + line.content.source_len];
      self.linenum = i;
      self.parsed = line as *mut _;
      if let Some((range, l)) = &line.content.label {
        if l.0 as i32 <= last_label {
          self.add_error(range.clone(), "行号必须递增");
        }
        self
          .label_addrs
          .insert(*l, self.code_emitter.current_addr());
        last_label = l.0 as i32;
      }

      self.code_emitter.begin_line(i);

      for &stmt in &line.content.stmts {
        self.compile_stmt(stmt);
      }

      text_offset += line.content.source_len;
    }

    self.resolve_labels(prog);
    self.resolve_datum_indices(prog);
    for (line, diag) in self.code_emitter.clean_up() {
      prog.lines[line].diagnostics.push(diag);
    }
  }

  fn resolve_labels(&mut self, prog: &mut Program) {
    for (addr, (line, range), label) in
      std::mem::replace(&mut self.pending_jump_labels, vec![])
    {
      let l = label.unwrap_or(Label(0));
      if let Some(&label_addr) = self.label_addrs.get(&l) {
        self.code_emitter.patch_jump_addr(addr, label_addr);
      } else {
        prog.lines[line].diagnostics.push(Diagnostic::new_error(
          range,
          if label.is_some() {
            format!("行号不存在")
          } else {
            format!("行号 0 不存在（省略行号则默认行号是 0）")
          },
        ));
      }
    }
  }

  fn resolve_datum_indices(&mut self, prog: &mut Program) {
    for (addr, (line, range), label) in
      std::mem::replace(&mut self.pending_datum_indices, vec![])
    {
      if let Some(&index) = self.data_start.get(&label) {
        self.code_emitter.patch_datum_index(addr, index);
      } else {
        prog.lines[line].diagnostics.push(Diagnostic::new_warning(
          range,
          "行号不存在，RESTORE 语句将会把 DATA 指针重置到程序开头",
        ));
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

        if $args.len().get() < $min_arity {
          self.add_error(
            $stmt.range.clone(),
            concat!(
              stringify!($kind),
              " 语句至少要有 ",
              stringify!($min_arity),
              " 个参数"));
        } else if $args.len().get() > $max_arity {
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
            $args.len().get());
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
        let ty = self.compile_expr(*value);
        if ty.matches(Type::Real) {
          self.code_emitter.emit_assign_num(range);
        } else {
          self.code_emitter.emit_assign_str(range);
        }
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
      } => self.compile_open(
        range,
        *filename,
        *mode,
        *filenum,
        len.as_ref().cloned(),
      ),
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
      StmtKind::Run(_) => self.code_emitter.emit_op(range, &stmt.kind, 0),
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
      StmtKind::While(cond) => self.compile_while(range, *cond),
      StmtKind::Write { filenum, data } => {
        self.compile_write(range, *filenum, data)
      }
      StmtKind::Sleep(arg) => self.compile_unary_stmt(
        range,
        &stmt.kind,
        *arg,
        Type::Real,
        "SLEEP",
        "参数",
      ),
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
    let name = name.as_ref().map(|name_range| {
      let (name, ty) = self.compile_sym(name_range.clone());
      if !ty.exact_matches(Type::Real) {
        self.add_error(
          name_range.clone(),
          format!("变量类型错误。自定义函数必须是{:#}类型", Type::Real),
        );
      }
      name
    });

    let param = param.as_ref().map(|param_range| {
      let (param, ty) = self.compile_sym(param_range.clone());
      if !ty.exact_matches(Type::Real) {
        self.add_error(
          param_range.clone(),
          format!(
            "参数类型错误。自定义函数的参数的类型必须是{:#}类型",
            Type::Real
          ),
        );
      }
      param
    });

    let body_range = self.expr_node(body).range.clone();
    if let (Some(name), Some(param)) = (name, param) {
      let def_addr = self.code_emitter.begin_def_fn(range, name, param);
      let ty = self.compile_expr(body);
      if !ty.matches(Type::Real) {
        self.add_error(
          body_range,
          format!(
            "函数体类型错误。自定义函数的函数体表达式必须是{}类型",
            Type::Real
          ),
        );
      }
      self.code_emitter.end_def_fn(def_addr);
    } else {
      let ty = self.compile_expr(body);
      if !ty.matches(Type::Real) {
        self.add_error(
          body_range,
          format!(
            "函数体类型错误。自定义函数的函数体表达式必须是{}类型",
            Type::Real
          ),
        );
      }
    }
  }

  fn compile_dim(&mut self, vars: &NonEmptyVec<[ExprId; 1]>) {
    for &var in vars.iter() {
      let var = &self.expr_node(var);
      if let ExprKind::Index { name, indices } = &var.kind {
        for &index in indices.iter() {
          let ty = self.compile_expr(index);
          if !ty.matches(Type::Real) {
            let range = &self.expr_node(index).range;
            self.add_error(
              range.clone(),
              format!(
                "表达式类型错误。数组下标必须是{}类型，而这个表达式是{}类型",
                Type::Real,
                ty,
              ),
            );
          }
        }
        if let Some(name_range) = name {
          let (name, _) = self.compile_sym(name_range.clone());
          self
            .code_emitter
            .emit_dim(name_range.clone(), name, indices.len());
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
      if !ty.matches(Type::String) {
        let range = &self.expr_node(field.var).range;
        self.add_error(
          range.clone(),
          format!(
            "字段类型错误。FIELD 语句的字段必须是{}类型，而这个{}是{}类型",
            Type::String,
            if is_array { "数组" } else { "变量" },
            ty
          ),
        );
      }
    }

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

  fn compile_read(&mut self, _range: Range, vars: &NonEmptyVec<[ExprId; 1]>) {
    for &var in vars.iter() {
      let (_, _) = self.compile_lvalue(var);
      let range = self.expr_node(var).range.clone();
      self.code_emitter.emit_read(range);
    }
  }

  fn compile_restore(&mut self, range: Range, label: &Option<(Range, Label)>) {
    let addr = self.code_emitter.emit_restore(range);
    if let Some((range, label)) = label {
      self.pending_datum_indices.push((
        addr,
        (self.linenum, range.clone()),
        *label,
      ));
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

  fn compile_while(&mut self, range: Range, cond: ExprId) {
    let cond_start = self.code_emitter.current_addr();
    let ty = self.compile_expr(cond);
    if !ty.matches(Type::Real) {
      let range = &self.expr_node(cond).range;
      self.add_error(
        range.clone(),
        format!(
          "表达式类型错误。WHILE 语句的条件表达式必须是{}类型，而这个表达式是{}类型",
          Type::Real,
          ty));
    }
    self.code_emitter.emit_while(range, cond_start);
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
    let mut printed = false;

    for (i, datum) in data.iter().enumerate() {
      let ty = self.compile_expr(datum.datum);
      if i < data.len().get() - 1 && !datum.comma {
        let range = self.expr_node(datum.datum).range.clone();
        self.add_warning(
          range,
          "这个表达式的值会被 WRITE 语句忽略，请在表达式末尾加上逗号",
        );
      }

      let range = self.expr_node(datum.datum).range.clone();
      if i == data.len().get() - 1 || datum.comma {
        printed = true;
        let end = i == data.len().get() - 1;
        if ty.matches(Type::Real) {
          self.code_emitter.emit_write_num(range, to_file, end);
        } else {
          self.code_emitter.emit_write_str(range, to_file, end);
        }
      } else if ty.matches(Type::Real) {
        self.code_emitter.emit_pop_num(range);
      } else {
        self.code_emitter.emit_pop_str(range);
      }
    }

    if !to_file && printed {
      self.code_emitter.emit_flush(range);
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
      if !ty.exact_matches(Type::Real) {
        self.add_error(
          var_range.clone(),
          format!(
            "变量类型错误。FOR 语句的计数器变量必须是{:#}类型，而这个变量是{:#}类型",
            Type::Real,
            ty,
          )
        );
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
          ty,
        )
      );
    }

    let ty = self.compile_expr(end);
    if !ty.matches(Type::Real) {
      let range = &self.expr_node(end).range;
      self.add_error(
        range.clone(),
        format!(
          "表达式类型错误。FOR 语句的计数器终止值必须是{}类型，而这个表达式是{}类型",
          Type::Real,
          ty,
        )
      );
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
            ty,
          )
        );
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
      self.pending_jump_labels.push((
        addr,
        (self.linenum, range.clone()),
        Some(*label),
      ));
    } else {
      self
        .pending_jump_labels
        .push((addr, (self.linenum, range), None));
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

    let jz_addr = self.code_emitter.emit_jz(range.clone());

    for &stmt in conseq.iter() {
      self.compile_stmt(stmt);
    }

    if let Some(alt) = alt {
      let conseq_end_addr = self.code_emitter.emit_goto(range);
      let alt_addr = self.code_emitter.current_addr();
      self.code_emitter.patch_jump_addr(jz_addr, alt_addr);

      for &stmt in alt.iter() {
        self.compile_stmt(stmt);
      }

      let end_addr = self.code_emitter.current_addr();
      self.code_emitter.patch_jump_addr(conseq_end_addr, end_addr);
    } else {
      let end_addr = self.code_emitter.current_addr();
      self.code_emitter.patch_jump_addr(jz_addr, end_addr);
    }
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
          let func = func.as_ref().map(|func_range| {
            let (func, ty) = self.compile_sym(func_range.clone());
            if !ty.exact_matches(Type::Real) {
              self.add_error(
                func_range.clone(),
                format!("变量类型错误。自定义函数必须是{:#}类型", Type::Real),
              );
            }
            func
          });
          let arg_range = self.expr_node(*arg).range.clone();
          let arg = if let ExprKind::Ident = &self.expr_node(*arg).kind {
            let (arg, ty) = self.compile_sym(arg_range.clone());
            if !ty.exact_matches(Type::Real) {
              self.add_error(
                arg_range,
                format!(
                  "变量类型错误。自定义函数的参数必须是{:#}类型",
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
        let (_, _) = self.compile_lvalue(var);
      }
    }
    match source {
      InputSource::Keyboard(prompt) => {
        self.code_emitter.emit_keyboard_input(
          range,
          prompt.as_ref().map(|p| {
            let mut text = &self.text[p.start + 1..p.end];
            if text.ends_with('"') {
              text = &text[..text.len() - 1];
            }
            text.to_owned()
          }),
          vars.len(),
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
        self.code_emitter.emit_file_input(range, vars.len());
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

    self.code_emitter.emit_on(range, labels.len());

    for (range, label) in labels.iter() {
      let addr = if is_sub {
        self.code_emitter.emit_gosub(range.clone())
      } else {
        self.code_emitter.emit_goto(range.clone())
      };
      self.pending_jump_labels.push((
        addr,
        (self.linenum, range.clone()),
        *label,
      ));
    }
  }

  fn compile_open(
    &mut self,
    range: Range,
    filename: ExprId,
    mode: FileMode,
    filenum: ExprId,
    len: Option<(Range, ExprId)>,
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

    if let Some((len_range, len)) = len {
      if !matches!(mode, FileMode::Random | FileMode::Error) {
        self.add_error(len_range, "LEN 参数只能用于以 RANDOM 模式打开的文件")
      }
      let ty = self.compile_expr(len);
      if !ty.matches(Type::Real) {
        let range = self.expr_node(len).range.clone();
        self.add_error(
          range,
          format!(
            "表达式类型错误。OPEN 语句的记录长度必须是{}类型，而这个表达式是{}类型",
            Type::Real,
            ty
          ),
        );
      }
      if !matches!(mode, FileMode::Error) {
        self.code_emitter.emit_open(range, mode, true);
      }
    } else {
      self.code_emitter.emit_open(range, mode, false);
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
          self.code_emitter.emit_next(
            if vars.len() == 1 {
              range.clone()
            } else {
              var_range.clone()
            },
            Some(var),
          );
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
    let mut printed = false;

    for (i, elem) in elems.iter().enumerate() {
      match elem {
        PrintElement::Semicolon(_) => {
          // do nothing
        }
        PrintElement::Comma(elem_range) => {
          self.code_emitter.emit_newline(elem_range.clone());
        }
        PrintElement::Expr(expr) => match &self.expr_node(*expr).kind {
          ExprKind::SysFuncCall {
            func: (_, kind @ (SysFuncKind::Spc | SysFuncKind::Tab)),
            args,
          } => {
            if args.len().get() > 1 {
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
              self.code_emitter.emit_newline(range.clone());
            }

            printed = true;
          }
          _ => {
            let ty = self.compile_expr(*expr);
            let elem_range = &self.expr_node(*expr).range;
            if ty.matches(Type::Real) {
              self.code_emitter.emit_print_num(elem_range.clone());
            } else {
              self.code_emitter.emit_print_str(elem_range.clone());
            }
            if i == elems.len() - 1 {
              self.code_emitter.emit_newline(range.clone());
            } else if matches!(&elems[i + 1], PrintElement::Expr(_)) {
              self
                .code_emitter
                .emit_number(elem_range.clone(), Mbf5::one());
              self.code_emitter.emit_print_spc(elem_range.clone());
            }

            printed = true;
          }
        },
      }
    }

    if elems.is_empty() {
      self.code_emitter.emit_newline(range.clone());
    } else if printed {
      self.code_emitter.emit_flush(range);
    }
  }
}

impl<'a, 'b, E: CodeEmitter, T> CompileState<'a, 'b, E, T> {
  #[must_use]
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
        if self
          .code_emitter
          .emit_string(range.clone(), text.to_owned())
        {
          self.add_error(range, "字符串太长，长度超出 255");
        }
        Type::String
      }
      ExprKind::NumberLit => {
        let mut text = self.text[range.start..range.end].to_owned();
        text.retain(|c| c != ' ');
        match text.parse::<Mbf5>() {
          Ok(num) => self.code_emitter.emit_number(range, num),
          Err(ParseRealError::Infinite) => {
            self.add_error(range, "数值过大，超出实数的表示范围")
          }
          Err(_) => unreachable!(),
        }
        Type::Real
      }
      ExprKind::SysFuncCall { func, args } => {
        self.compile_sys_func_call(range, func, args)
      }
      ExprKind::UserFuncCall { func, arg } => {
        let func = func.as_ref().map(|func_range| {
          let (func, ty) = self.compile_sym(func_range.clone());
          if !ty.exact_matches(Type::Real) {
            self.add_error(
              func_range.clone(),
              format!("变量类型错误。自定义函数必须是{:#}类型", Type::Real),
            );
          }
          func
        });
        let ty = self.compile_expr(*arg);
        if !ty.matches(Type::Real) {
          let range = &self.expr_node(*arg).range;
          self.add_error(
            range.clone(),
            format!(
              "表达式类型错误。自定义函数的参数必须是{}类型，而这个表达式是{}类型",
              Type::Real,
              ty),
          );
        }
        if let Some(func) = func {
          self.code_emitter.emit_user_func_call(range, func);
        }
        Type::Real
      }
      ExprKind::Binary { lhs, op, rhs } => {
        self.compile_binary_expr(range, *lhs, op, *rhs)
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
          self.code_emitter.emit_index(range, name, indices.len());
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

  fn compile_sys_func_call(
    &mut self,
    range: Range,
    func: &(Range, SysFuncKind),
    args: &NonEmptyVec<[ExprId; 1]>,
  ) -> Type {
    let (min_arity, max_arity, arg_tys, ret_ty) = match func.1 {
      SysFuncKind::Abs
      | SysFuncKind::Atn
      | SysFuncKind::Cos
      | SysFuncKind::Exp
      | SysFuncKind::Int
      | SysFuncKind::Log
      | SysFuncKind::Peek
      | SysFuncKind::Rnd
      | SysFuncKind::Sgn
      | SysFuncKind::Sin
      | SysFuncKind::Sqr
      | SysFuncKind::Tan
      | SysFuncKind::Eof
      | SysFuncKind::Lof
      | SysFuncKind::Pos => {
        (1, 1, [Type::Real, Type::Error, Type::Error], Type::Real)
      }
      SysFuncKind::Asc
      | SysFuncKind::Cvi
      | SysFuncKind::Cvs
      | SysFuncKind::Len
      | SysFuncKind::Val => {
        (1, 1, [Type::String, Type::Error, Type::Error], Type::Real)
      }
      SysFuncKind::Mki
      | SysFuncKind::Mks
      | SysFuncKind::Chr
      | SysFuncKind::Str => {
        (1, 1, [Type::Real, Type::Error, Type::Error], Type::String)
      }
      SysFuncKind::Left | SysFuncKind::Right => {
        (2, 2, [Type::String, Type::Real, Type::Error], Type::String)
      }
      SysFuncKind::Mid => {
        (2, 3, [Type::String, Type::Real, Type::Real], Type::String)
      }
      SysFuncKind::Tab | SysFuncKind::Spc => {
        self.add_error(
          func.0.clone(),
          format!("{:?} 函数只能作为 PRINT 语句的参数出现", func.1),
        );
        (1, 1, [Type::Real, Type::Error, Type::Error], Type::Real)
      }
    };
    if args.len().get() < min_arity {
      self.add_error(
        range.clone(),
        if min_arity == max_arity {
          format!("{:?} 函数必须有 {} 个参数", func.1, min_arity)
        } else {
          format!("{:?} 函数至少要有 {} 个参数", func.1, min_arity)
        },
      );
    } else if args.len().get() > max_arity {
      self.add_error(
        range.clone(),
        if min_arity == max_arity {
          format!("{:?} 函数必须有 {} 个参数", func.1, max_arity)
        } else {
          format!("{:?} 函数最多接受 {} 个参数", func.1, max_arity)
        },
      );
    }

    for (i, &arg) in args.iter().enumerate() {
      let ty = self.compile_expr(arg);
      if i < max_arity {
        if !ty.matches(arg_tys[i]) {
          let range = &self.expr_node(arg).range;
          self.add_error(
            range.clone(),
            format!(
              "表达式类型错误。{:?} 函数的第 {} 个参数是{}类型，而这个表达式是{}类型",
              func.1,
              i + 1,
              arg_tys[i],
              ty
            ),
          );
        }
      }
    }

    self
      .code_emitter
      .emit_sys_func_call(range, func.1, args.len());

    ret_ty
  }

  fn compile_binary_expr(
    &mut self,
    range: Range,
    lhs: ExprId,
    op: &(Range, BinaryOpKind),
    rhs: ExprId,
  ) -> Type {
    let lhs_ty = self.compile_expr(lhs);
    let rhs_ty = self.compile_expr(rhs);

    match op.1 {
      BinaryOpKind::Eq
      | BinaryOpKind::Ne
      | BinaryOpKind::Gt
      | BinaryOpKind::Lt
      | BinaryOpKind::Ge
      | BinaryOpKind::Le
      | BinaryOpKind::Add => {
        if lhs_ty.matches(rhs_ty) {
          if lhs_ty == Type::String {
            self.code_emitter.emit_str_binary_expr(range, op.1);
          } else {
            self.code_emitter.emit_num_binary_expr(range, op.1);
          }
          if let BinaryOpKind::Add = op.1 {
            lhs_ty.as_rvalue_type()
          } else {
            Type::Real
          }
        } else {
          self.add_error(
            op.0.clone(),
            format!(
              "运算数类型不匹配，左边是{}类型，右边是{}类型",
              lhs_ty, rhs_ty
            ),
          );
          if let BinaryOpKind::Add = op.1 {
            Type::Error
          } else {
            Type::Real
          }
        }
      }
      BinaryOpKind::Sub
      | BinaryOpKind::Mul
      | BinaryOpKind::Div
      | BinaryOpKind::Pow
      | BinaryOpKind::And
      | BinaryOpKind::Or => {
        if !lhs_ty.matches(Type::Real) {
          let lhs_range = self.expr_node(lhs).range.clone();
          self.add_error(
            lhs_range,
            format!(
              "类型不匹配。{}运算左边必须是{}类型，而这个表达式是{}类型",
              op.1,
              Type::Real,
              lhs_ty
            ),
          );
        }
        if !rhs_ty.matches(Type::Real) {
          let rhs_range = self.expr_node(rhs).range.clone();
          self.add_error(
            rhs_range,
            format!(
              "类型不匹配。{}运算右边必须是{}类型，而这个表达式是{}类型",
              op.1,
              Type::Real,
              rhs_ty
            ),
          );
        }
        self.code_emitter.emit_num_binary_expr(range, op.1);
        Type::Real
      }
    }
  }

  #[must_use]
  fn compile_lvalue(&mut self, lvalue: ExprId) -> (bool, Type) {
    let lvalue = &self.expr_node(lvalue);
    match &lvalue.kind {
      ExprKind::Ident => {
        let (var, ty) = self.compile_sym(lvalue.range.clone());
        self.code_emitter.emit_var_lvalue(lvalue.range.clone(), var);
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
          self.code_emitter.emit_index_lvalue(
            name_range.clone(),
            name,
            indices.len(),
          );
          (true, ty)
        } else {
          (true, Type::Error)
        }
      }
      ExprKind::Error => (false, Type::Error),
      _ => unreachable!(),
    }
  }

  #[must_use]
  fn compile_sym(&mut self, range: Range) -> (E::Symbol, Type) {
    let mut name = self.text[range.start..range.end].to_ascii_uppercase();
    let ty = match name.as_bytes().last() {
      Some(b'%') => Type::Integer,
      Some(b'$') => Type::String,
      _ => Type::Real,
    };

    if let Some(i) = name.find(' ') {
      name.truncate(i);
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

  fn as_rvalue_type(self) -> Self {
    match self {
      Self::Integer => Self::Real,
      _ => self,
    }
  }
}

impl Display for Type {
  fn fmt(&self, f: &mut Formatter) -> fmt::Result {
    match self {
      Self::Integer => {
        if f.alternate() {
          write!(f, "整数")
        } else {
          write!(f, "数值")
        }
      }
      Self::Real => {
        if f.alternate() {
          write!(f, "实数")
        } else {
          write!(f, "数值")
        }
      }
      Self::String => write!(f, "字符串"),
      Self::Error => unreachable!(),
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::machine::EmojiStyle;
  use crate::parser::{parse_expr, parse_prog};
  use crate::vm::codegen::CodeGen;
  use insta::assert_debug_snapshot;
  use pretty_assertions::assert_eq;

  fn compile(text: &str) -> CodeGen {
    let mut prog = parse_prog(text);
    let mut codegen = CodeGen::new(EmojiStyle::New);
    compile_prog(text, &mut prog, &mut codegen);
    for (i, line) in prog.lines.iter().enumerate() {
      let diags: Vec<_> = line
        .diagnostics
        .iter()
        .filter(|d| d.severity == Severity::Error)
        .cloned()
        .collect();
      assert_eq!(diags, vec![], "line {}", i);
    }
    codegen
  }

  #[test]
  fn assignment() {
    assert_debug_snapshot!(compile(r#"10 a=a*b+coo%/2^len(a$+"xx和")"#));
  }

  #[test]
  fn nullary_statement() {
    assert_debug_snapshot!(compile(
      r#"
10 beep:clear:end:cls
20 cont : rem ss lssss  jfju14jlgsas975309fakl;gkS&^*$%F951)
30 flash:text:graph:notrace:pop:return:inKey$:trace:run jf:aksdl
    "#
      .trim()
    ));
  }

  #[test]
  fn draw() {
    assert_debug_snapshot!(compile(r#"
10 boX 1+2,3,4,5,7:box a%,val(c$(a,b)+chr$(tan(72))),m%(3,j+k,i),0:box 1,2,3,4,5,6
20 draw 10,20:draw -3,1,t
30 circle x*10,y*10,3:circle x1,y1%+2,11,0:circle x2,y2,r,4,5
40 line x0,y0,x1,y1:line x0,y0,x1,y1,2
50 ellipse x,y,a,b:ellipse x,y,a,b,1:ellipse x,y,a,b,k,6
    "#.trim()));
  }

  #[test]
  fn jump() {
    assert_debug_snapshot!(compile(
      r#"
0 :
10 ::gosub 30:goto 10::
20 on x+1 goto 30,,40:on m(x) gosub:on m gosub 40,50,40,,
30 print 1:end
40 print 2;:end
50 print 3,:end
    "#
      .trim()
    ));
  }

  #[test]
  fn ppc() {
    assert_debug_snapshot!(compile(
      r#"
10 let a = peek(23):poke 237+i,c(i+1)
20 call 31284+k
    "#
      .trim()
    ));
  }

  #[test]
  fn data() {
    assert_debug_snapshot!(compile(
      r#"
10 read a,b$,c(m+1,10): data "cj,:",abc13,  126a  ,
20 data 
30 data "",,,++  !:
40 restore:restore 30:restore 20:restore 25
    "#
      .trim()
    ));
  }

  #[test]
  fn r#fn() {
    assert_debug_snapshot!(compile(
      r#"
10 def fn k(x )=sin(i/2)+3:def fn F (x) = fn f (x)
20 let k=1+fn k(37+fn k(0))
    "#
      .trim()
    ));
  }

  #[test]
  fn dim() {
    assert_debug_snapshot!(compile(
      r#"
10 dim a$,b(3,f(2)),k%(m+1):dim a
    "#
      .trim()
    ));
  }

  #[test]
  fn for_loop() {
    assert_debug_snapshot!(compile(
      r#"
10 for i =10 to n+1:for abc=k(m)*3 to 31 step -n*k
20 next:next i:next i,j,ka
    "#
      .trim()
    ));
  }

  #[test]
  fn r#if() {
    assert_debug_snapshot!(compile(
      r#"
10 if a+b goto print:30:
20 if f(x)>3 then cont:else graph:cls:gosub 10
30 if a< >b goto x=x+1:20:30:if x>10 then play "abc" else 50:t(i)=int(x/2):else 10
50 rem x
    "#
      .trim()
    ));
  }

  #[test]
  fn input() {
    assert_debug_snapshot!(compile(
      r#"
10 input a, b$, c%(m+2,i): input "ABC123俄"; fn v(t ), c%
    "#
      .trim()
    ));
  }

  #[test]
  fn locate() {
    assert_debug_snapshot!(compile(
      r#"
10 locate i+1:locate,2*k(m+1):locate 5,10
    "#
      .trim()
    ));
  }

  #[test]
  fn set() {
    assert_debug_snapshot!(compile(
      r#"
10 lset r$(x)=chr$(0):rset t$="abc"+str$(i)
    "#
      .trim()
    ));
  }

  #[test]
  fn swap() {
    assert_debug_snapshot!(compile(
      r#"
10 swap a$(b*2,3,i*10+j), c$:swap c%,d%
    "#
      .trim()
    ));
  }

  #[test]
  fn while_loop() {
    assert_debug_snapshot!(compile(
      r#"
10 while a$>"AB=号即":print:wend:while a<>b:cls:while a<2:cls:wend
20 cls:goto 20:wend
    "#
      .trim()
    ));
  }

  #[test]
  fn print() {
    assert_debug_snapshot!(compile(
      r#"
10 print:print;:print,:print a$:print a$+b$;: print a$b$:
20 print a$ 3;spc(n+1);7:print spc(2)abc$: print;;t;:print ,T%(K),,;
30 print 3;4,tab(7)6,tab(12);8,:
40 print spc(2):print ,,tab(3)
    "#
      .trim()
    ));
  }

  #[test]
  fn write() {
    assert_debug_snapshot!(compile(
      r#"
10 write 1+2, c$(t+1),:write a$b,mki$(j)
    "#
      .trim()
    ));
  }

  #[test]
  fn sleep() {
    assert_debug_snapshot!(compile(
      r#"
10 sleep -2*x:sleep 0:sleep 300
    "#
      .trim()
    ));
  }

  #[test]
  fn for_loop_to_sleep() {
    assert_debug_snapshot!(compile(
      r#"
10 for i=1 to 30:next i:for j=0 to 2000 step 1:next:
20 for j=1 to 1000:cls:next:for i=1 to n:next i,j:
30 for k=1 to t step 1:next k:for k=1 to t step 1:next t
    "#
      .trim()
    ));
  }

  mod file {
    use super::*;

    #[test]
    fn open() {
      assert_debug_snapshot!(compile(
        r#"
10 open a$ for append as i: open "foo" inpuT as3:
30 open a$+b$ random a sc:open a$(3) output as#k+1
40 open c$(2) for random as #3len=v*2:
    "#
        .trim()
      ));
    }

    #[test]
    fn close() {
      assert_debug_snapshot!(compile(
        r#"
10 close 3: close #2:close#k+1
30 close fi(i):
    "#
        .trim()
      ));
    }

    #[test]
    fn field() {
      assert_debug_snapshot!(compile(
        r#"
10 field f(i)+1, 25aSa$(i), 1 as m$ : field #1,k+3 asa$
    "#
        .trim()
      ));
    }

    #[test]
    fn get_put() {
      assert_debug_snapshot!(compile(
        r#"
10 get f(i)+1, m*2:get #i/20,abc
20 put #f(i)+1,m*3:put i/20,AbC%
    "#
        .trim()
      ));
    }

    #[test]
    fn input() {
      assert_debug_snapshot!(compile(
        r#"
10 input #1, a, b$, c%(m+2,i)
    "#
        .trim()
      ));
    }

    #[test]
    fn write() {
      assert_debug_snapshot!(compile(
        r#"
10 write #k+1, abc$ 12+val(chr$(k)), s%(2),:write #2,a,b(1,2,3),c:
    "#
        .trim()
      ));
    }
  }

  #[test]
  fn fn_body() {
    let text = r#"x + 3 * fn f(7) - 2"#;
    let mut prog = parse_expr(text).0;
    let mut codegen = CodeGen::new(EmojiStyle::New);
    compile_fn_body(text, &mut prog, &mut codegen);
    assert_eq!(prog.diagnostics, vec![]);
    assert_debug_snapshot!(codegen);
  }

  #[test]
  fn fn_body_type_mismatch() {
    let text = r#"x$ + chr$(i)"#;
    let mut prog = parse_expr(text).0;
    let mut codegen = CodeGen::new(EmojiStyle::New);
    compile_fn_body(text, &mut prog, &mut codegen);
    assert_debug_snapshot!(prog.diagnostics);
  }
}
