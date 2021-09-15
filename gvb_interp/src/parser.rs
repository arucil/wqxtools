use crate::ast::{
  BinaryOpKind, Datum, Eol, Expr, ExprId, ExprKind, FieldSpec, FileMode,
  InputSource, Keyword, Label, NodeBuilder, NonEmptyVec, ParseLabelError,
  PrintElement, Program, ProgramLine, Punc, Range, Stmt, StmtId, StmtKind,
  SysFuncKind, TokenKind, UnaryOpKind, WriteElement,
};
use crate::diagnostic::Diagnostic;
use id_arena::Arena;
use smallvec::{smallvec, Array, SmallVec};

pub fn parse(input: &str) -> Program {
  let mut line_start = 0;
  let mut lines = vec![];
  while let Some(eol) = input[line_start..].find('\n') {
    lines.push(parse_line(&input[line_start..eol]));
    line_start = eol;
  }
  if line_start < input.len() {
    lines.push(parse_line(&input[line_start..]));
  }
  Program { lines }
}

/// `line_with_eol` may contain newline.
pub fn parse_line(line_with_eol: &str) -> ProgramLine {
  let bytes = line_with_eol.as_bytes();
  let line;
  let eol;
  if let Some(b'\n') = bytes.last().copied() {
    if bytes.len() > 1 && bytes[bytes.len() - 2] == b'\r' {
      eol = Eol::CrLf;
      line = &line_with_eol[..line_with_eol.len() - 2];
    } else {
      eol = Eol::Lf;
      line = &line_with_eol[..line_with_eol.len() - 1];
    }
  } else {
    eol = Eol::None;
    line = line_with_eol;
  }

  let node_builder = ArenaNodeBuilder {
    stmt_arena: Arena::new(),
    expr_arena: Arena::new(),
  };
  let mut parser = LineParser::new(line, node_builder);

  let mut label = None;
  if matches!(line.as_bytes().first(), Some(b' ')) {
    parser.read_token(true);
    if parser.token.1 == TokenKind::Label {
      parser.read_token(false);
      match parser.label_value.take().unwrap() {
        Ok(l) => label = Some(l),
        Err(err) => parser.report_label_error(err, parser.token.0.clone()),
      }
    } else {
      parser.report_label_error(
        ParseLabelError::NotALabel,
        Range::new(0, line_with_eol.len()),
      );
    }
  }

  let stmts = parser.parse_stmts(false);
  if stmts.is_empty() {
    parser.add_error(Range::new(0, line_with_eol.len()), "缺少语句");
  }

  parser.into_line(line_with_eol, eol, label, stmts)
}

struct ArenaNodeBuilder {
  stmt_arena: Arena<Stmt>,
  expr_arena: Arena<Expr>,
}

impl NodeBuilder for ArenaNodeBuilder {
  fn new_stmt(&mut self, stmt: Stmt) -> StmtId {
    self.stmt_arena.alloc(stmt)
  }

  fn new_expr(&mut self, expr: Expr) -> ExprId {
    self.expr_arena.alloc(expr)
  }

  fn stmt_range(&self, stmt: StmtId) -> Range {
    self.stmt_arena[stmt].range.clone()
  }

  fn expr_range(&self, expr: ExprId) -> Range {
    self.expr_arena[expr].range.clone()
  }
}

struct LineParser<'a, T: NodeBuilder> {
  offset: usize,
  input: &'a str,
  token: (Range, TokenKind),
  label_value: Option<Result<Label, ParseLabelError>>,
  node_builder: T,
  diagnostics: Vec<Diagnostic>,
}

impl<'a, T: NodeBuilder> LineParser<'a, T> {
  fn new(input: &'a str, node_builder: T) -> Self {
    Self {
      offset: 0,
      input,
      token: (Range::new(0, 0), TokenKind::Eof),
      label_value: None,
      node_builder,
      diagnostics: vec![],
    }
  }

  fn add_error(&mut self, range: Range, message: impl ToString) {
    self.diagnostics.push(Diagnostic::new_error(range, message));
  }

  fn add_warning(&mut self, range: Range, message: impl ToString) {
    self
      .diagnostics
      .push(Diagnostic::new_warning(range, message));
  }

  fn advance(&mut self, count: usize) {
    self.offset += count;
    self.input = &self.input[count..];
  }

  fn report_label_error(&mut self, err: ParseLabelError, range: Range) {
    match err {
      ParseLabelError::NotALabel => {
        self.add_error(range, "缺少行号");
      }
      ParseLabelError::OutOfBound => {
        self.add_error(range, "行号必须在0~9999之间");
      }
    }
  }

  fn skip_space(&mut self) {
    self.advance(count_space(self.input.as_bytes(), 0));
  }

  fn skip_line(&mut self) {
    self.offset += self.input.len();
    self.input = "";
  }

  fn set_token(&mut self, start: usize, kind: TokenKind) {
    self.token = (Range::new(start, self.offset), kind);
  }

  fn put_back_token(&mut self) {
    let len = self.token.0.len();
    self.offset -= len;
    self.input = unsafe {
      std::str::from_utf8_unchecked(std::slice::from_raw_parts(
        self.input.as_ptr().sub(len),
        self.input.len() + len,
      ))
    };
  }

  fn read_token(&mut self, read_label: bool) {
    self.label_value = None;

    self.skip_space();
    let start = self.offset;
    let c;
    if let Some(&c1) = self.input.as_bytes().first() {
      c = c1;
    } else {
      return self.set_token(start, TokenKind::Eof);
    }

    match c {
      b'=' | b'<' | b'>' | b'+' | b'-' | b'*' | b'/' | b'^' | b':' | b'('
      | b')' | b';' | b',' | b'#' => {
        self.advance(1);
        self.set_token(start, TokenKind::Punc(Punc::from(c)));
      }
      b'"' => {
        let len = self.read_quoted_string();
        let start = self.offset;
        self.advance(len);
        self.set_token(start, TokenKind::String);
      }
      b'0'..=b'9' | b'.' => {
        let (len, is_nat) = read_number(self.input.as_bytes(), true);
        let start = self.offset;
        if is_nat && read_label {
          let label = self.input[..len].parse::<Label>();
          self.advance(len);
          self.label_value = Some(label);
          self.set_token(start, TokenKind::Label);
        } else {
          self.advance(len);
          self.set_token(start, TokenKind::Float);
        }
      }
      b'a'..=b'z' | b'A'..=b'Z' => {
        let start = self.offset;
        let mut i = 0;
        while matches!(
          self.input.as_bytes().get(i),
          Some(c) if c.is_ascii_alphanumeric()
        ) {
          i += 1;
        }
        let mut sigil = false;
        if let Some(b'%' | b'$') = self.input.as_bytes().get(i) {
          i += 1;
          sigil = true;
        }

        let str = self.input[..i].to_ascii_lowercase();
        self.advance(i);
        if let Ok(kw) = str.parse::<Keyword>() {
          return self.set_token(start, TokenKind::Keyword(kw));
        } else if let Ok(f) = str.parse::<SysFuncKind>() {
          return self.set_token(start, TokenKind::SysFunc(f));
        } else if sigil {
          return self.set_token(start, TokenKind::Ident);
        }

        let mut i = count_space(self.input.as_bytes(), 0);

        loop {
          match self.input.as_bytes().get(i) {
            Some(c) if c.is_ascii_alphanumeric() => i += 1,
            Some(b' ') => i += 1,
            Some(b'%' | b'$') => {
              i += 1;
              break;
            }
            _ => break,
          }
        }

        while i > 0 && self.input.as_bytes()[i - 1] == b' ' {
          i -= 1;
        }

        self.advance(i);
        self.set_token(start, TokenKind::Ident);
      }
      _ => {
        let start = self.offset;
        let c = self.input.chars().next().unwrap();
        self.advance(c.len_utf8());
        self.add_error(
          Range::new(start, self.offset),
          if (c as u32) < 0x10000 {
            format!("非法字符：U+{:04X}", c as u32)
          } else {
            format!("非法字符：U+{:06X}", c as u32)
          },
        );
        self.read_token(read_label);
      }
    }
  }

  fn read_quoted_string(&self) -> usize {
    let mut i = 1;
    loop {
      match self.input.as_bytes().get(i) {
        Some(&c) if c != b'"' => i += 1,
        Some(b'"') => {
          i += 1;
          break;
        }
        _ => break,
      }
    }
    i
  }

  fn match_token(&mut self, token: TokenKind, read_label: bool) -> Range {
    if self.token.1 == token {
      let range = self.token.0.clone();
      self.read_token(read_label);
      range
    } else {
      todo!("skip tokens")
    }
  }

  fn parse_stmts(&mut self, in_if_branch: bool) -> SmallVec<[StmtId; 1]> {
    let mut stmts = smallvec![];
    loop {
      match self.token.1 {
        TokenKind::Punc(Punc::Colon) => {
          if in_if_branch {
            self.add_error(
              Range::new(self.offset, self.offset + 1),
              "IF 语句的分支中不能出现多余的冒号",
            );
          } else {
            stmts.push(self.node_builder.new_stmt(Stmt {
              kind: StmtKind::NoOp,
              range: self.token.0.clone(),
              is_recovered: false,
            }));
          }
          self.read_token(in_if_branch);
        }
        TokenKind::Eof => return stmts,
        TokenKind::Keyword(Keyword::Else) => {}
        _ => {
          let stmt = self.parse_stmt(in_if_branch);
          stmts.push(stmt);
          let mut stmt_end = false;
          if let TokenKind::Punc(Punc::Colon) = self.token.1 {
            self.read_token(in_if_branch);
            stmt_end = true;
          } else if self.token.1 == TokenKind::Eof {
            stmt_end = true;
          }
          if matches!(self.token.1, TokenKind::Keyword(Keyword::Else)) {
            if in_if_branch {
              return stmts;
            } else {
              self
                .add_error(self.token.0.clone(), "ELSE 不能出现在 IF 语句之外");
              self.read_token(in_if_branch);
            }
          } else if !stmt_end {
            self.add_error(
              self.node_builder.stmt_range(stmt),
              "语句之后必须是行尾或跟上冒号",
            );
          }
        }
      }
    }
  }

  fn parse_stmt(&mut self, in_if_branch: bool) -> StmtId {
    use self::Keyword as Kw;
    use TokenKind::*;

    match self.token.1 {
      Keyword(Kw::Auto) => self.parse_rem_stmt(StmtKind::Auto, in_if_branch),
      Keyword(Kw::Beep) => self.parse_nullary_cmd(StmtKind::Beep),
      Keyword(Kw::Box) => self.parse_cmd(StmtKind::Box),
      Keyword(Kw::Call) => self.parse_unary_cmd(StmtKind::Call),
      Keyword(Kw::Circle) => self.parse_cmd(StmtKind::Circle),
      Keyword(Kw::Clear) => self.parse_nullary_cmd(StmtKind::Clear),
      Keyword(Kw::Close) => self.parse_close_stmt(),
      Keyword(Kw::Cls) => self.parse_nullary_cmd(StmtKind::Cls),
      Keyword(Kw::Cont) => self.parse_nullary_cmd(StmtKind::Cont),
      Keyword(Kw::Copy) => self.parse_rem_stmt(StmtKind::Copy, in_if_branch),
      Keyword(Kw::Data) => self.parse_data_stmt(),
      Keyword(Kw::Def) => self.parse_def_stmt(),
      Keyword(Kw::Del) => self.parse_rem_stmt(StmtKind::Del, in_if_branch),
      Keyword(Kw::Dim) => self.parse_dim_stmt(),
      Keyword(Kw::Draw) => self.parse_cmd(StmtKind::Draw),
      Keyword(Kw::Edit) => self.parse_rem_stmt(StmtKind::Edit, in_if_branch),
      Keyword(Kw::Ellipse) => self.parse_cmd(StmtKind::Ellipse),
      Keyword(Kw::End) => self.parse_nullary_cmd(StmtKind::End),
      Keyword(Kw::Field) => self.parse_field_stmt(),
      Keyword(Kw::Files) => self.parse_rem_stmt(StmtKind::Files, in_if_branch),
      Keyword(Kw::Flash) => self.parse_nullary_cmd(StmtKind::Flash),
      Keyword(Kw::For) => self.parse_for_stmt(),
      Keyword(Kw::Get) => self.parse_get_put_stmt(false),
      Keyword(Kw::Gosub) => self.parse_go_stmt(StmtKind::GoSub),
      Keyword(Kw::Goto) => self.parse_go_stmt(|label| StmtKind::GoTo {
        has_goto_keyword: true,
        label,
      }),
      Keyword(Kw::Graph) => self.parse_nullary_cmd(StmtKind::Graph),
      Keyword(Kw::If) => self.parse_if_stmt(),
      Keyword(Kw::Inkey) => self.parse_nullary_cmd(StmtKind::InKey),
      Keyword(Kw::Input) => self.parse_input_stmt(),
      Keyword(Kw::Inverse) => self.parse_nullary_cmd(StmtKind::Inverse),
      Keyword(Kw::Kill) => self.parse_rem_stmt(StmtKind::Kill, in_if_branch),
      Keyword(Kw::Let) => self.parse_assign_stmt(true),
      Ident => self.parse_assign_stmt(false),
      Keyword(Kw::Line) => self.parse_cmd(StmtKind::Line),
      Keyword(Kw::List) => self.parse_rem_stmt(StmtKind::List, in_if_branch),
      Keyword(Kw::Load) => self.parse_rem_stmt(StmtKind::Load, in_if_branch),
      Keyword(Kw::Locate) => self.parse_locate_stmt(),
      Keyword(Kw::Lset) => {
        self.parse_set_stmt(|var, value| StmtKind::LSet { var, value })
      }
      Keyword(Kw::New) => self.parse_rem_stmt(StmtKind::New, in_if_branch),
      Keyword(Kw::Next) => self.parse_next_stmt(),
      Keyword(Kw::Normal) => self.parse_nullary_cmd(StmtKind::Normal),
      Keyword(Kw::Notrace) => self.parse_nullary_cmd(StmtKind::NoTrace),
      Keyword(Kw::On) => self.parse_on_stmt(),
      Keyword(Kw::Open) => self.parse_open_stmt(),
      Keyword(Kw::Play) => self.parse_unary_cmd(StmtKind::Play),
      Keyword(Kw::Poke) => self.parse_poke_stmt(),
      Keyword(Kw::Pop) => self.parse_nullary_cmd(StmtKind::Pop),
      Keyword(Kw::Print) => self.parse_print_stmt(),
      Keyword(Kw::Put) => self.parse_get_put_stmt(true),
      Keyword(Kw::Read) => self.parse_read_stmt(),
      Keyword(Kw::Rem) => self.parse_rem_stmt(StmtKind::Rem, in_if_branch),
      Keyword(Kw::Rename) => {
        self.parse_rem_stmt(StmtKind::Rename, in_if_branch)
      }
      Keyword(Kw::Restore) => self.parse_go_stmt(StmtKind::Restore),
      Keyword(Kw::Return) => self.parse_nullary_cmd(StmtKind::Return),
      Keyword(Kw::Rset) => {
        self.parse_set_stmt(|var, value| StmtKind::RSet { var, value })
      }
      Keyword(Kw::Run) => self.parse_nullary_cmd(StmtKind::Run),
      Keyword(Kw::Save) => self.parse_rem_stmt(StmtKind::Save, in_if_branch),
      Keyword(Kw::Stop) => self.parse_rem_stmt(StmtKind::Stop, in_if_branch),
      Keyword(Kw::Swap) => self.parse_swap_stmt(),
      Keyword(Kw::System) => self.parse_nullary_cmd(StmtKind::System),
      Keyword(Kw::Text) => self.parse_nullary_cmd(StmtKind::Text),
      Keyword(Kw::Trace) => self.parse_nullary_cmd(StmtKind::Trace),
      Keyword(Kw::Wend) => self.parse_nullary_cmd(StmtKind::Wend),
      Keyword(Kw::While) => self.parse_unary_cmd(StmtKind::While),
      Keyword(Kw::Write) => self.parse_write_stmt(),
      Eof => unreachable!(),
      _ => {
        todo!("expect stmt")
      }
    }
  }

  fn parse_unary_cmd(&mut self, ctor: fn(ExprId) -> StmtKind) -> StmtId {
    let start = self.token.0.start;
    self.read_token(false);
    let arg = self.parse_expr(Prec::None);
    let end = self.node_builder.expr_range(arg).end;
    self.node_builder.new_stmt(Stmt {
      kind: ctor(arg),
      range: Range::new(start, end),
      is_recovered: false,
    })
  }

  fn parse_close_stmt(&mut self) -> StmtId {
    let start = self.token.0.start;
    self.read_token(false);
    if self.token.1 == TokenKind::Punc(Punc::Hash) {
      self.read_token(false);
    }
    let filenum = self.parse_expr(Prec::None);
    let end = self.node_builder.expr_range(filenum).end;
    self.node_builder.new_stmt(Stmt {
      kind: StmtKind::Close { filenum },
      range: Range::new(start, end),
      is_recovered: false,
    })
  }

  fn parse_data_stmt(&mut self) -> StmtId {
    let start = self.token.0.start;
    let mut data = NonEmptyVec::<[Datum; 1]>::new();
    loop {
      self.skip_space();
      let datum_start = self.offset;
      if let Some(b'"') = self.input.as_bytes().first() {
      } else {
        let mut i = 0;
        while !matches!(self.input.as_bytes().get(i), Some(b',' | b':') | None)
        {
          i += 1;
        }
        self.advance(i);
        data.push(Datum {
          range: Range::new(datum_start, self.offset),
          is_quoted: false,
        })
      }

      match self.input.as_bytes().first() {
        Some(b':') | None => break,
        Some(b',') => self.advance(1),
        _ => {
          self.add_error(Range::new(self.offset, self.offset + 1), "缺少逗号");
        }
      }
    }
    self.node_builder.new_stmt(Stmt {
      kind: StmtKind::Data(data),
      range: Range::new(start, self.offset),
      is_recovered: false,
    })
  }

  fn parse_def_stmt(&mut self) -> StmtId {
    let start = self.token.0.start;
    self.read_token(false);
    self.match_token(TokenKind::Keyword(Keyword::Fn), false);
    let name_range = self.match_token(TokenKind::Ident, false);
    self.match_token(TokenKind::Punc(Punc::LParen), false);
    let param_range = self.match_token(TokenKind::Ident, false);
    self.match_token(TokenKind::Punc(Punc::RParen), false);
    self.match_token(TokenKind::Punc(Punc::Eq), false);
    let body = self.parse_expr(Prec::None);
    let end = self.node_builder.expr_range(body).end;
    self.node_builder.new_stmt(Stmt {
      kind: StmtKind::Def {
        name: name_range,
        param: param_range,
        body,
      },
      range: Range::new(start, end),
      is_recovered: false,
    })
  }

  fn parse_dim_stmt(&mut self) -> StmtId {
    let start = self.token.0.start;
    self.read_token(false);
    let (end, vars) = self.parse_lvalue_list();
    self.node_builder.new_stmt(Stmt {
      kind: StmtKind::Dim(vars),
      range: Range::new(start, end),
      is_recovered: false,
    })
  }

  fn parse_field_stmt(&mut self) -> StmtId {
    let start = self.token.0.start;
    self.read_token(false);
    if let TokenKind::Punc(Punc::Hash) = self.token.1 {
      self.read_token(false);
    }
    let filenum = self.parse_expr(Prec::None);
    let mut fields = NonEmptyVec::<[FieldSpec; 1]>::new();
    self.match_token(TokenKind::Punc(Punc::Comma), false);
    let field = self.parse_field_spec();
    let mut end = field.range.end;
    fields.push(field);
    while self.token.1 == TokenKind::Punc(Punc::Comma) {
      self.read_token(false);
      let field = self.parse_field_spec();
      end = field.range.end;
      fields.push(field);
    }
    self.node_builder.new_stmt(Stmt {
      kind: StmtKind::Field { filenum, fields },
      range: Range::new(start, end),
      is_recovered: false,
    })
  }

  fn parse_field_spec(&mut self) -> FieldSpec {
    let start = self.token.0.start;
    let len = self.parse_expr(Prec::None);
    self.put_back_token();
    if let Some(b'A' | b'a') = self.input.as_bytes().first() {
      self.advance(1);
      self.skip_space();
      if let Some(b'S' | b's') = self.input.as_bytes().first() {
        self.advance(1);
      } else {
        todo!("expect AS")
      }
    } else {
      todo!("expect AS")
    }
    self.read_token(false);
    let var = self.parse_lvalue();
    let end = self.node_builder.expr_range(var).end;
    FieldSpec {
      range: Range::new(start, end),
      len,
      var,
    }
  }

  fn parse_for_stmt(&mut self) -> StmtId {
    let start = self.token.0.start;
    self.read_token(false);
    let id_range = self.match_token(TokenKind::Ident, false);
    self.match_token(TokenKind::Punc(Punc::Eq), false);
    let from = self.parse_expr(Prec::None);
    self.match_token(TokenKind::Keyword(Keyword::To), false);
    let to = self.parse_expr(Prec::None);
    let mut end = self.node_builder.expr_range(to).end;
    let mut step = None;
    if let TokenKind::Keyword(Keyword::Step) = self.token.1 {
      self.read_token(false);
      let s = self.parse_expr(Prec::None);
      step = Some(s);
      end = self.node_builder.expr_range(s).end;
    }
    self.node_builder.new_stmt(Stmt {
      kind: StmtKind::For {
        var: id_range,
        start: from,
        end: from,
        step,
      },
      range: Range::new(start, end),
      is_recovered: false,
    })
  }

  fn parse_go_stmt(
    &mut self,
    ctor: fn(Option<(Range, Label)>) -> StmtKind,
  ) -> StmtId {
    let mut range = self.token.0.clone();
    self.read_token(true);
    let mut label = None;
    if self.token.1 == TokenKind::Label {
      match self.label_value.take().unwrap() {
        Ok(l) => {
          label = Some((self.token.0.clone(), l));
          range.end = self.token.0.end;
        }
        Err(err) => self.report_label_error(err, self.token.0.clone()),
      }
      self.read_token(false);
    }
    self.node_builder.new_stmt(Stmt {
      kind: ctor(label),
      range,
      is_recovered: false,
    })
  }

  fn parse_get_put_stmt(&mut self, is_put: bool) -> StmtId {
    let start = self.token.0.start;
    self.read_token(false);
    if self.token.1 == TokenKind::Punc(Punc::Hash) {
      self.read_token(false);
    }
    let filenum = self.parse_expr(Prec::None);
    self.match_token(TokenKind::Punc(Punc::Comma), false);
    let record = self.parse_expr(Prec::None);
    let end = self.node_builder.expr_range(record).end;
    let kind = if is_put {
      StmtKind::Put { filenum, record }
    } else {
      StmtKind::Get { filenum, record }
    };
    self.node_builder.new_stmt(Stmt {
      kind,
      range: Range::new(start, end),
      is_recovered: false,
    })
  }

  fn parse_if_stmt(&mut self) -> StmtId {
    let start = self.token.0.start;
    self.read_token(false);
    let cond = self.parse_expr(Prec::None);
    let mut end = start;
    let conseq;
    if let TokenKind::Keyword(kw @ (Keyword::Then | Keyword::Goto)) =
      self.token.1
    {
      let then_range = self.token.0.clone();
      self.read_token(true);
      conseq = self.parse_stmts(true);
      if let Some(&stmt) = conseq.last() {
        end = self.node_builder.stmt_range(stmt).end;
      } else {
        end = then_range.end;
        self.add_error(then_range, format!("{:?} 之后缺少语句", kw))
      }
    } else {
      todo!("expect THEN or GOTO")
    }
    let mut alt = None;
    if let TokenKind::Keyword(Keyword::Else) = self.token.1 {
      let else_range = self.token.0.clone();
      self.read_token(true);
      let stmts = self.parse_stmts(true);
      if let Some(&stmt) = stmts.last() {
        end = self.node_builder.stmt_range(stmt).end;
      } else {
        end = else_range.end;
        self.add_error(else_range, "ELSE 之后缺少语句")
      }
      alt = Some(stmts);
    }
    self.node_builder.new_stmt(Stmt {
      kind: StmtKind::If {
        cond,
        conseq: conseq.into(),
        alt,
      },
      range: Range::new(start, end),
      is_recovered: false,
    })
  }

  fn parse_input_stmt(&mut self) -> StmtId {
    let start = self.token.0.start;
    self.read_token(false);
    let source;
    match self.token.1 {
      TokenKind::String => {
        let prompt_range = self.match_token(TokenKind::String, false);
        self.match_token(TokenKind::Punc(Punc::Semicolon), false);
        let prompt = self
          .node_builder
          .new_expr(Expr::new(ExprKind::StringLit, prompt_range));
        source = InputSource::Keyboard(prompt);
      }
      TokenKind::Punc(Punc::Hash) => {
        self.read_token(false);
        let filenum = self.parse_expr(Prec::None);
        self.match_token(TokenKind::Punc(Punc::Comma), false);
        source = InputSource::File(filenum);
      }
      _ => {
        source = InputSource::Error;
        todo!("expect # or string")
      }
    }

    let (end, vars) = self.parse_lvalue_list();
    self.node_builder.new_stmt(Stmt {
      kind: StmtKind::Input { source, vars },
      range: Range::new(start, end),
      is_recovered: false,
    })
  }

  fn parse_lvalue_list(&mut self) -> (usize, NonEmptyVec<[ExprId; 1]>) {
    let mut vars = NonEmptyVec::<[ExprId; 1]>::new();
    let var = self.parse_lvalue();
    let mut end = self.node_builder.expr_range(var).end;
    vars.push(var);
    while let TokenKind::Punc(Punc::Comma) = self.token.1 {
      self.read_token(false);
      let var = self.parse_lvalue();
      end = self.node_builder.expr_range(var).end;
      vars.push(var);
    }
    (end, vars)
  }

  fn parse_assign_stmt(&mut self, has_let: bool) -> StmtId {
    let start = self.token.0.start;
    if has_let {
      self.read_token(false);
    }
    let var = self.parse_lvalue();
    self.match_token(TokenKind::Punc(Punc::Eq), false);
    let value = self.parse_expr(Prec::None);
    let end = self.node_builder.expr_range(value).end;
    self.node_builder.new_stmt(Stmt {
      kind: StmtKind::Let { var, value },
      range: Range::new(start, end),
      is_recovered: false,
    })
  }

  fn parse_locate_stmt(&mut self) -> StmtId {
    let Range { start, mut end } = self.token.0.clone();
    self.read_token(false);
    let row;
    if let TokenKind::Punc(Punc::Comma) = self.token.1 {
      row = None;
    } else {
      let r = self.parse_expr(Prec::None);
      end = self.node_builder.expr_range(r).end;
      row = Some(r);
    }
    let column;
    if let TokenKind::Punc(Punc::Comma) = self.token.1 {
      self.read_token(false);
      let r = self.parse_expr(Prec::None);
      end = self.node_builder.expr_range(r).end;
      column = Some(r);
    } else {
      column = None;
    }
    self.node_builder.new_stmt(Stmt {
      kind: StmtKind::Locate { row, column },
      range: Range::new(start, end),
      is_recovered: false,
    })
  }

  fn parse_set_stmt(&mut self, ctor: fn(ExprId, ExprId) -> StmtKind) -> StmtId {
    let start = self.token.0.start;
    self.read_token(false);
    let var = self.parse_lvalue();
    self.match_token(TokenKind::Punc(Punc::Eq), false);
    let value = self.parse_expr(Prec::None);
    let end = self.node_builder.expr_range(value).end;
    self.node_builder.new_stmt(Stmt {
      kind: ctor(var, value),
      range: Range::new(start, end),
      is_recovered: false,
    })
  }

  fn parse_next_stmt(&mut self) -> StmtId {
    let Range { start, mut end } = self.token.0.clone();
    self.read_token(false);
    let mut vars = SmallVec::<[Range; 1]>::new();
    if let TokenKind::Ident = self.token.1 {
      loop {
        let var_range = self.match_token(TokenKind::Ident, false);
        end = var_range.end;
        vars.push(var_range);
        if let TokenKind::Punc(Punc::Comma) = self.token.1 {
          self.read_token(false);
        } else {
          break;
        }
      }
    }
    self.node_builder.new_stmt(Stmt {
      kind: StmtKind::Next { vars },
      range: Range::new(start, end),
      is_recovered: false,
    })
  }

  fn parse_on_stmt(&mut self) -> StmtId {
    let Range { start, mut end } = self.token.0.clone();
    self.read_token(false);
    let cond = self.parse_expr(Prec::None);
    let is_sub;
    match self.token.1 {
      TokenKind::Keyword(Keyword::Gosub) => {
        end = self.token.0.end;
        self.read_token(true);
        is_sub = true;
      }
      TokenKind::Keyword(Keyword::Goto) => {
        end = self.token.0.end;
        self.read_token(true);
        is_sub = false;
      }
      _ => {
        todo!("expect GOSUB or GOTO, skip to digit, read_token(true)")
      }
    }

    let mut labels = NonEmptyVec::<[(Range, Label); 2]>::new();
    if self.token.1 == TokenKind::Label {
      match self.label_value.take().unwrap() {
        Ok(label) => {
          end = self.token.0.end;
          labels.push((self.token.0.clone(), label));
          self.read_token(true);
        }
        Err(err) => {
          self.report_label_error(err, self.token.0.clone());
        }
      }
    } else {
      todo!("expect label")
    };

    while let TokenKind::Punc(Punc::Comma) = self.token.1 {
      self.read_token(true);

      if self.token.1 == TokenKind::Label {
        match self.label_value.take().unwrap() {
          Ok(label) => {
            end = self.token.0.end;
            labels.push((self.token.0.clone(), label));
            self.read_token(true);
          }
          Err(err) => {
            self.report_label_error(err, self.token.0.clone());
          }
        }
      } else {
        todo!("expect label")
      };
    }

    self.node_builder.new_stmt(Stmt {
      kind: StmtKind::On {
        cond,
        labels,
        is_sub,
      },
      range: Range::new(start, end),
      is_recovered: false,
    })
  }

  fn parse_open_stmt(&mut self) -> StmtId {
    let Range { start, mut end } = self.token.0.clone();
    self.read_token(false);
    let filename = self.parse_expr(Prec::None);
    if let TokenKind::Keyword(Keyword::For) = self.token.1 {
      self.skip_space();
    } else {
      self.put_back_token();
    }

    let mode;
    if self.input.len() >= 6 {
      let m = &self.input.as_bytes()[..6];
      if m.eq_ignore_ascii_case(b"output") {
        mode = FileMode::Output;
      } else if m.eq_ignore_ascii_case(b"append") {
        mode = FileMode::Append;
      } else if m.eq_ignore_ascii_case(b"random") {
        mode = FileMode::Random
      } else {
        todo!("expect file mode, skip to A or #")
      }
      self.advance(6);
    } else if self.input.len() >= 5
      && self.input.as_bytes()[..5].eq_ignore_ascii_case(b"input")
    {
      mode = FileMode::Input;
      self.advance(5);
    } else {
      todo!("expect file mode, skip to A or #")
    }

    self.skip_space();

    if let Some(b'A' | b'a') = self.input.as_bytes().first() {
      self.advance(1);
      self.skip_space();
      if let Some(b'S' | b's') = self.input.as_bytes().first() {
        self.advance(1);
      } else {
        todo!("expect AS")
      }
    } else {
      todo!("expect AS")
    }

    self.read_token(true);
    if let TokenKind::Punc(Punc::Hash) = self.token.1 {
      self.read_token(true);
    }

    let filenum = self.parse_expr(Prec::None);
    end = self.node_builder.expr_range(filenum).end;

    let mut len = None;
    if let TokenKind::SysFunc(SysFuncKind::Len) = self.token.1 {
      self.read_token(false);
      self.match_token(TokenKind::Punc(Punc::Eq), false);
      let l = self.parse_expr(Prec::None);
      end = self.node_builder.expr_range(l).end;
      len = Some(l);
    }

    self.node_builder.new_stmt(Stmt {
      kind: StmtKind::Open {
        filename,
        mode,
        filenum,
        len,
      },
      range: Range::new(start, end),
      is_recovered: false,
    })
  }

  fn parse_poke_stmt(&mut self) -> StmtId {
    let Range { start, mut end } = self.token.0.clone();
    self.read_token(false);
    let addr = self.parse_expr(Prec::None);
    self.match_token(TokenKind::Punc(Punc::Comma), false);
    let value = self.parse_expr(Prec::None);
    end = self.node_builder.expr_range(value).end;
    self.node_builder.new_stmt(Stmt {
      kind: StmtKind::Poke { addr, value },
      range: Range::new(start, end),
      is_recovered: false,
    })
  }

  fn parse_print_stmt(&mut self) -> StmtId {
    let Range { start, mut end } = self.token.0.clone();
    self.read_token(false);

    let mut elems = SmallVec::<[PrintElement; 2]>::new();

    loop {
      match self.token.1 {
        TokenKind::Punc(Punc::Comma) => elems.push(PrintElement::Comma),
        TokenKind::Punc(Punc::Semicolon) => elems.push(PrintElement::Semicolon),
        TokenKind::Punc(Punc::Colon)
        | TokenKind::Keyword(Keyword::Else)
        | TokenKind::Eof => break,
        _ => {
          let expr = self.parse_expr(Prec::None);
          elems.push(PrintElement::Expr(expr));
        }
      }
    }

    self.node_builder.new_stmt(Stmt {
      kind: StmtKind::Print(elems),
      range: Range::new(start, end),
      is_recovered: false,
    })
  }

  fn parse_read_stmt(&mut self) -> StmtId {
    let start = self.token.0.start;
    self.read_token(false);
    let (end, vars) = self.parse_lvalue_list();
    self.node_builder.new_stmt(Stmt {
      kind: StmtKind::Read(vars),
      range: Range::new(start, end),
      is_recovered: false,
    })
  }

  fn parse_swap_stmt(&mut self) -> StmtId {
    let Range { start, mut end } = self.token.0.clone();
    self.read_token(false);
    let left = self.parse_lvalue();
    self.match_token(TokenKind::Punc(Punc::Comma), false);
    let right = self.parse_lvalue();
    end = self.node_builder.expr_range(right).end;
    self.node_builder.new_stmt(Stmt {
      kind: StmtKind::Swap { left, right },
      range: Range::new(start, end),
      is_recovered: false,
    })
  }

  fn parse_write_stmt(&mut self) -> StmtId {
    let Range { start, mut end } = self.token.0.clone();
    self.read_token(false);
    let filenum;
    if let TokenKind::Punc(Punc::Hash) = self.token.1 {
      self.read_token(false);
      filenum = Some(self.parse_expr(Prec::None));
      self.match_token(TokenKind::Punc(Punc::Comma), false);
    } else {
      filenum = None;
    }

    let mut data = NonEmptyVec::<[WriteElement; 1]>::new();
    let mut datum = self.parse_expr(Prec::None);

    loop {
      if let TokenKind::Punc(Punc::Comma) = self.token.1 {
        end = self.token.0.end;
        data.push(WriteElement { datum, comma: true });
        self.read_token(false);
      } else {
        end = self.node_builder.expr_range(datum).end;
        data.push(WriteElement {
          datum,
          comma: false,
        });
      }
      if let TokenKind::Punc(Punc::Colon)
      | TokenKind::Keyword(Keyword::Else)
      | TokenKind::Eof = self.token.1
      {
        break;
      }
      datum = self.parse_expr(Prec::None);
    }

    self.node_builder.new_stmt(Stmt {
      kind: StmtKind::Write { filenum, data },
      range: Range::new(start, end),
      is_recovered: false,
    })
  }

  fn parse_cmd<A: Array<Item = ExprId>>(
    &mut self,
    ctor: fn(NonEmptyVec<A>) -> StmtKind,
  ) -> StmtId {
    let start = self.token.0.start;
    self.read_token(false);
    let mut args = NonEmptyVec::<A>::new();
    let Range { end, .. } = self.parse_cmd_args(&mut args.0);
    self.node_builder.new_stmt(Stmt {
      kind: ctor(args),
      range: Range::new(start, end),
      is_recovered: false,
    })
  }

  fn parse_nullary_cmd(&mut self, kind: StmtKind) -> StmtId {
    let range = self.token.0.clone();
    self.read_token(false);
    self.node_builder.new_stmt(Stmt {
      kind,
      range,
      is_recovered: false,
    })
  }

  fn parse_rem_stmt(
    &mut self,
    ctor: fn(Range) -> StmtKind,
    in_if_branch: bool,
  ) -> StmtId {
    let start = self.offset;
    self.skip_line();
    let id = self.node_builder.new_stmt(Stmt {
      kind: ctor(Range::new(start, self.offset)),
      range: Range::new(self.token.0.start, self.offset),
      is_recovered: false,
    });
    self.read_token(in_if_branch);
    id
  }

  fn parse_expr(&mut self, prec: Prec) -> ExprId {
    let mut lhs = self.parse_atom();
    while token_prec(self.token.1) > prec {
      let op = self.read_binary_op();
      let rhs = self.parse_expr(token_prec(self.token.1));
      let start = self.node_builder.expr_range(lhs).start;
      let end = self.node_builder.expr_range(rhs).end;
      lhs = self.node_builder.new_expr(Expr::new(
        ExprKind::Binary { lhs, op, rhs },
        Range::new(start, end),
      ));
    }
    lhs
  }

  fn read_binary_op(&mut self) -> (Range, BinaryOpKind) {
    let range = self.token.0.clone();
    let op = match self.token.1 {
      TokenKind::Punc(Punc::Eq) => BinaryOpKind::Eq,
      TokenKind::Punc(Punc::Lt) => {
        self.read_token(false);
        match self.token.1 {
          TokenKind::Punc(Punc::Eq) => BinaryOpKind::Le,
          TokenKind::Punc(Punc::Gt) => BinaryOpKind::Ne,
          _ => return (range, BinaryOpKind::Lt),
        }
      }
      TokenKind::Punc(Punc::Gt) => {
        self.read_token(false);
        if self.token.1 == TokenKind::Punc(Punc::Eq) {
          BinaryOpKind::Ge
        } else {
          return (range, BinaryOpKind::Gt);
        }
      }
      TokenKind::Punc(Punc::Plus) => BinaryOpKind::Add,
      TokenKind::Punc(Punc::Minus) => BinaryOpKind::Sub,
      TokenKind::Punc(Punc::Times) => BinaryOpKind::Mul,
      TokenKind::Punc(Punc::Slash) => BinaryOpKind::Div,
      TokenKind::Punc(Punc::Caret) => BinaryOpKind::Pow,
      TokenKind::Keyword(Keyword::And) => BinaryOpKind::And,
      TokenKind::Keyword(Keyword::Or) => BinaryOpKind::Or,
      _ => unreachable!(),
    };
    let end = self.token.0.end;
    self.read_token(false);
    (Range::new(range.start, end), op)
  }

  fn parse_atom(&mut self) -> ExprId {
    match self.token.1 {
      TokenKind::Float => {
        let id = self
          .node_builder
          .new_expr(Expr::new(ExprKind::NumberLit, self.token.0.clone()));
        self.read_token(false);
        id
      }
      TokenKind::String => {
        let id = self
          .node_builder
          .new_expr(Expr::new(ExprKind::StringLit, self.token.0.clone()));
        self.read_token(false);
        id
      }
      TokenKind::Keyword(Keyword::Inkey) => {
        let id = self
          .node_builder
          .new_expr(Expr::new(ExprKind::Inkey, self.token.0.clone()));
        self.read_token(false);
        id
      }
      TokenKind::Punc(op @ (Punc::Plus | Punc::Minus)) => {
        let start = self.token.0.start;
        let op_range = self.token.0.clone();
        self.read_token(false);
        let arg = self.parse_expr(Prec::Neg);
        let op = match op {
          Punc::Plus => UnaryOpKind::Pos,
          Punc::Minus => UnaryOpKind::Neg,
          _ => unreachable!(),
        };
        let end = self.node_builder.expr_range(arg).end;
        let kind = ExprKind::Unary {
          op: (op_range, op),
          arg,
        };
        self
          .node_builder
          .new_expr(Expr::new(kind, Range::new(start, end)))
      }
      TokenKind::Keyword(Keyword::Not) => {
        let start = self.token.0.start;
        let op_range = self.token.0.clone();
        self.read_token(false);
        let arg = self.parse_expr(Prec::Not);
        let end = self.node_builder.expr_range(arg).end;
        let kind = ExprKind::Unary {
          op: (op_range, UnaryOpKind::Not),
          arg,
        };
        self
          .node_builder
          .new_expr(Expr::new(kind, Range::new(start, end)))
      }
      TokenKind::Punc(Punc::LParen) => {
        self.read_token(false);
        let expr = self.parse_expr(Prec::None);
        self.match_token(TokenKind::Punc(Punc::RParen), false);
        expr
      }
      TokenKind::Keyword(Keyword::Fn) => {
        let start = self.token.0.start;
        self.read_token(false);
        let id_range = self.match_token(TokenKind::Ident, false);
        self.match_token(TokenKind::Punc(Punc::LParen), false);
        let arg = self.parse_expr(Prec::None);
        let Range { end, .. } =
          self.match_token(TokenKind::Punc(Punc::RParen), false);

        let kind = ExprKind::UserFuncCall {
          func: id_range,
          arg,
        };
        self
          .node_builder
          .new_expr(Expr::new(kind, Range::new(start, end)))
      }
      TokenKind::SysFunc(kind) => {
        let name_range = self.token.0.clone();
        self.read_token(false);
        let mut args = NonEmptyVec::<[ExprId; 1]>::new();
        let Range { end, .. } = self.parse_paren_args(&mut args.0);
        let range = Range::new(name_range.start, end);
        let kind = ExprKind::SysFuncCall {
          func: (name_range, kind),
          args,
        };
        self.node_builder.new_expr(Expr::new(kind, range))
      }
      TokenKind::Ident => self.parse_lvalue(),
      _ => {
        todo!("expect expr, skip tokens")
      }
    }
  }

  fn parse_lvalue(&mut self) -> ExprId {
    let id_range = self.match_token(TokenKind::Ident, false);
    if self.token.1 == TokenKind::Punc(Punc::LParen) {
      let mut args = NonEmptyVec::<[ExprId; 1]>::new();
      let Range { end, .. } = self.parse_paren_args(&mut args.0);
      let range = Range::new(id_range.start, end);
      let kind = ExprKind::Index {
        name: id_range,
        indices: args,
      };
      self.node_builder.new_expr(Expr::new(kind, range))
    } else {
      self
        .node_builder
        .new_expr(Expr::new(ExprKind::Ident, id_range))
    }
  }

  fn parse_paren_args<U>(&mut self, args: &mut U) -> Range
  where
    U: Extend<ExprId>,
  {
    let start = self.token.0.start;
    self.match_token(TokenKind::Punc(Punc::LParen), false);
    let arg = self.parse_expr(Prec::None);
    args.extend_one(arg);
    while self.token.1 == TokenKind::Punc(Punc::Comma) {
      self.read_token(false);

      let arg = self.parse_expr(Prec::None);
      args.extend_one(arg);
    }

    if self.token.1 == TokenKind::Punc(Punc::RParen) {
      let end = self.token.0.end;
      self.read_token(false);
      Range::new(start, end)
    } else {
      todo!("expect ) or ,")
    }
  }

  fn parse_cmd_args<U>(&mut self, args: &mut U) -> Range
  where
    U: Extend<ExprId>,
  {
    let start = self.token.0.start;
    let arg = self.parse_expr(Prec::None);
    let mut end = self.node_builder.expr_range(arg).end;
    args.extend_one(arg);
    while self.token.1 == TokenKind::Punc(Punc::Comma) {
      self.read_token(false);

      let arg = self.parse_expr(Prec::None);
      end = self.node_builder.expr_range(arg).end;
      args.extend_one(arg);
    }

    if let TokenKind::Punc(Punc::Colon)
    | TokenKind::Keyword(Keyword::Else)
    | TokenKind::Eof = self.token.1
    {
      Range::new(start, end)
    } else {
      todo!("expect : or , or ELSE")
    }
  }
}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum Prec {
  None,
  Log,
  Rel,
  Add,
  Mul,
  Neg,
  Pow,
  Not,
}

fn token_prec(kind: TokenKind) -> Prec {
  match kind {
    TokenKind::Punc(Punc::Eq | Punc::Lt | Punc::Gt) => Prec::Rel,
    TokenKind::Punc(Punc::Plus | Punc::Minus) => Prec::Add,
    TokenKind::Punc(Punc::Times | Punc::Slash) => Prec::Mul,
    TokenKind::Punc(Punc::Caret) => Prec::Pow,
    TokenKind::Keyword(Keyword::And | Keyword::Or) => Prec::Log,
    _ => Prec::None,
  }
}

impl<'a> LineParser<'a, ArenaNodeBuilder> {
  fn into_line(
    self,
    line: &str,
    eol: Eol,
    label: Option<Label>,
    stmts: SmallVec<[StmtId; 1]>,
  ) -> ProgramLine {
    ProgramLine {
      source_len: line.len(),
      label,
      stmt_arena: self.node_builder.stmt_arena,
      expr_arena: self.node_builder.expr_arena,
      stmts,
      eol,
      diagnostics: self.diagnostics,
    }
  }
}

fn count_space(input: &[u8], start: usize) -> usize {
  let mut i = start;
  while let Some(b' ') = input.get(i) {
    i += 1;
  }
  i - start
}

/// ```regexp
/// [-+]?\d*(\.\d*)?(E[-+]?\d*)?
/// ```
pub fn read_number(input: &[u8], allow_space: bool) -> (usize, bool) {
  let mut i = 0;
  let mut is_nat = true;

  if let Some(b'+' | b'-') = input.first() {
    is_nat = false;
    i += 1;
  }
  if allow_space {
    i += count_space(input, i);
  }

  loop {
    match input.get(i) {
      Some(c) if c.is_ascii_digit() => i += 1,
      Some(b' ') if allow_space => i += 1,
      _ => break,
    }
  }

  if let Some(b'.') = input.get(i) {
    is_nat = false;
    i += 1;
    loop {
      match input.get(i) {
        Some(c) if c.is_ascii_digit() => i += 1,
        Some(b' ') if allow_space => i += 1,
        _ => break,
      }
    }
  }

  if let Some(b'e' | b'E') = input.get(i) {
    is_nat = false;
    i += 1;
    if allow_space {
      i += count_space(input, i);
    }
    if let Some(b'+' | b'-') = input.get(i) {
      i += 1;
    }
    loop {
      match input.get(i) {
        Some(c) if c.is_ascii_digit() => i += 1,
        Some(b' ') if allow_space => i += 1,
        _ => break,
      }
    }
  }

  if allow_space {
    while i > 0 && input[i - 1] == b' ' {
      i -= 1;
    }
  }

  (i, is_nat)
}

#[cfg(test)]
struct DummyNodeBuilder;

#[cfg(test)]
impl NodeBuilder for DummyNodeBuilder {
  fn new_expr(&mut self, _expr: Expr) -> ExprId {
    unimplemented!()
  }

  fn new_stmt(&mut self, _stmt: Stmt) -> StmtId {
    unimplemented!()
  }

  fn stmt_range(&self, _stmt: StmtId) -> Range {
    unimplemented!()
  }

  fn expr_range(&self, _expr: ExprId) -> Range {
    unimplemented!()
  }
}

#[cfg(test)]
mod lex_tests {
  use super::*;
  use insta::assert_debug_snapshot;
  use pretty_assertions::assert_eq;

  fn read_tokens(input: &str) -> Vec<(Range, TokenKind)> {
    let mut parser = LineParser::new(input, DummyNodeBuilder);
    let mut tokens = vec![];
    loop {
      parser.read_token(false);
      tokens.push(parser.token.clone());
      if parser.token.1 == TokenKind::Eof {
        break;
      }
    }
    tokens
  }

  #[test]
  fn punctuations() {
    assert_eq!(
      read_tokens(r#"   ><  =^ ; (   "#),
      vec![
        (Range::new(3, 4), TokenKind::Punc(Punc::Gt)),
        (Range::new(4, 5), TokenKind::Punc(Punc::Lt)),
        (Range::new(7, 8), TokenKind::Punc(Punc::Eq)),
        (Range::new(8, 9), TokenKind::Punc(Punc::Caret)),
        (Range::new(10, 11), TokenKind::Punc(Punc::Semicolon)),
        (Range::new(12, 13), TokenKind::Punc(Punc::LParen)),
        (Range::new(16, 16), TokenKind::Eof),
      ]
    );
  }

  #[test]
  fn string() {
    assert_eq!(
      read_tokens(r#"   "Fo和1" "3   "#),
      vec![
        (Range::new(3, 11), TokenKind::String),
        (Range::new(12, 17), TokenKind::String),
        (Range::new(17, 17), TokenKind::Eof),
      ]
    );
  }

  mod number {
    use super::*;
    use pretty_assertions::assert_eq;

    #[test]
    fn integer() {
      assert_eq!(
        read_tokens(r#"  134A$ 0"#),
        vec![
          (Range::new(2, 5), TokenKind::Float),
          (Range::new(5, 7), TokenKind::Ident),
          (Range::new(8, 9), TokenKind::Float),
          (Range::new(9, 9), TokenKind::Eof),
        ]
      );
    }

    #[test]
    fn fraction() {
      assert_eq!(
        read_tokens(r#"  0.14   -147.  .1"#),
        vec![
          (Range::new(2, 6), TokenKind::Float),
          (Range::new(9, 10), TokenKind::Punc(Punc::Minus)),
          (Range::new(10, 14), TokenKind::Float),
          (Range::new(16, 18), TokenKind::Float),
          (Range::new(18, 18), TokenKind::Eof),
        ]
      );
    }

    #[test]
    fn sci_notation() {
      assert_eq!(
        read_tokens(r#"  5E+17 .E3 , 1.e-14 , 12.3e45 , .E+  "#),
        vec![
          (Range::new(2, 7), TokenKind::Float),
          (Range::new(8, 11), TokenKind::Float),
          (Range::new(12, 13), TokenKind::Punc(Punc::Comma)),
          (Range::new(14, 20), TokenKind::Float),
          (Range::new(21, 22), TokenKind::Punc(Punc::Comma)),
          (Range::new(23, 30), TokenKind::Float),
          (Range::new(31, 32), TokenKind::Punc(Punc::Comma)),
          (Range::new(33, 36), TokenKind::Float),
          (Range::new(38, 38), TokenKind::Eof),
        ]
      );
    }

    #[test]
    fn space_in_number() {
      assert_eq!(
        read_tokens(r#"  123 456  7   8 , 12 . 3 , 12 . 3 E + 4 5, . e  -  "#),
        vec![
          (Range::new(2, 16), TokenKind::Float),
          (Range::new(17, 18), TokenKind::Punc(Punc::Comma)),
          (Range::new(19, 25), TokenKind::Float),
          (Range::new(26, 27), TokenKind::Punc(Punc::Comma)),
          (Range::new(28, 42), TokenKind::Float),
          (Range::new(42, 43), TokenKind::Punc(Punc::Comma)),
          (Range::new(44, 50), TokenKind::Float),
          (Range::new(52, 52), TokenKind::Eof),
        ]
      );
    }
  }

  mod symbol {
    use super::*;
    use pretty_assertions::assert_eq;

    #[test]
    fn identifier() {
      assert_eq!(
        read_tokens(r#"  foo : Ba7 123 "#),
        vec![
          (Range::new(2, 5), TokenKind::Ident),
          (Range::new(6, 7), TokenKind::Punc(Punc::Colon)),
          (Range::new(8, 15), TokenKind::Ident),
          (Range::new(16, 16), TokenKind::Eof),
        ]
      );
    }

    #[test]
    fn sigil_ends_an_identifier() {
      assert_eq!(
        read_tokens(r#"  foo%bar$ foo bar "#),
        vec![
          (Range::new(2, 6), TokenKind::Ident),
          (Range::new(6, 10), TokenKind::Ident),
          (Range::new(11, 18), TokenKind::Ident),
          (Range::new(19, 19), TokenKind::Eof),
        ]
      );
    }

    #[test]
    fn multi_frag_identifier() {
      assert_eq!(
        read_tokens(r#"  foo b7R 123 Z%  a b c $x y z "#),
        vec![
          (Range::new(2, 16), TokenKind::Ident),
          (Range::new(18, 25), TokenKind::Ident),
          (Range::new(25, 30), TokenKind::Ident),
          (Range::new(31, 31), TokenKind::Eof),
        ]
      );
    }

    #[test]
    fn keyword() {
      assert_eq!(
        read_tokens(r#"  lEt foo leti%  "#),
        vec![
          (Range::new(2, 5), TokenKind::Keyword(Keyword::Let)),
          (Range::new(6, 15), TokenKind::Ident),
          (Range::new(17, 17), TokenKind::Eof),
        ]
      );
    }

    #[test]
    fn sysfunc() {
      assert_eq!(
        read_tokens(r#"  AsC cHr$ leti%  "#),
        vec![
          (Range::new(2, 5), TokenKind::SysFunc(SysFuncKind::Asc)),
          (Range::new(6, 10), TokenKind::SysFunc(SysFuncKind::Chr)),
          (Range::new(11, 16), TokenKind::Ident),
          (Range::new(18, 18), TokenKind::Eof),
        ]
      );
    }
  }

  #[test]
  fn real_world_example() {
    let tokens = read_tokens(
      r#"LOCATE 3,2:PRINT "啊A":LOCATE 3,18-LEN(STR$(ET)):PRINT ET:DRAW 100,38"#,
    );
    assert_debug_snapshot!(tokens);
  }
}
