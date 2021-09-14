use crate::ast::{
  Datum, Eol, Expr, ExprId, ExprKind, Keyword, Label, NodeBuilder, NonEmptyVec,
  ParseLabelError, Program, ProgramLine, Punc, Range, Stmt, StmtId, StmtKind,
  SysFuncKind, TokenKind, UnaryOpKind,
};
use crate::diagnostic::Diagnostic;
use id_arena::Arena;
use smallvec::Array;

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

        let first_frag_end = self.offset;

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

  fn parse_stmts(&mut self, in_if_branch: bool) -> Vec<StmtId> {
    let mut stmts = vec![];
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
    match self.token.1 {
      TokenKind::Keyword(Keyword::Auto) => {
        self.parse_rem_like(StmtKind::Auto, in_if_branch)
      }
      TokenKind::Keyword(Keyword::Beep) => {
        self.parse_nullary_cmd(StmtKind::Beep)
      }
      TokenKind::Keyword(Keyword::Box) => self.parse_cmd(StmtKind::Box),
      TokenKind::Keyword(Keyword::Call) => {
        let start = self.token.0.start;
        self.read_token(false);
        let addr = self.parse_expr(Prec::None);
        let end = self.node_builder.expr_range(addr).end;
        self.node_builder.new_stmt(Stmt {
          kind: StmtKind::Call { addr },
          range: Range::new(start, end),
          is_recovered: false,
        })
      }
      TokenKind::Keyword(Keyword::Circle) => self.parse_cmd(StmtKind::Circle),
      TokenKind::Keyword(Keyword::Clear) => {
        self.parse_nullary_cmd(StmtKind::Clear)
      }
      TokenKind::Keyword(Keyword::Close) => {
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
      TokenKind::Keyword(Keyword::Cls) => self.parse_nullary_cmd(StmtKind::Cls),
      TokenKind::Keyword(Keyword::Cont) => {
        self.parse_nullary_cmd(StmtKind::Cont)
      }
      TokenKind::Keyword(Keyword::Copy) => {
        self.parse_rem_like(StmtKind::Copy, in_if_branch)
      }
      TokenKind::Keyword(Keyword::Data) => {
        let start = self.token.0.start;
        let mut data = NonEmptyVec::<[Datum; 1]>::new();
        loop {
          self.skip_space();
          let datum_start = self.offset;
          if let Some(b'"') = self.input.as_bytes().first() {
          } else {
            let mut i = 0;
            while !matches!(
              self.input.as_bytes().get(i),
              Some(b',' | b':') | None
            ) {
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
              self.add_error(
                Range::new(self.offset, self.offset + 1),
                "缺少逗号",
              );
            }
          }
        }
        self.node_builder.new_stmt(Stmt {
          kind: StmtKind::Data(data),
          range: Range::new(start, self.offset),
          is_recovered: false,
        })
      }
      TokenKind::Eof => unreachable!(),
    }
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

  fn parse_rem_like(
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

  fn parse_expr(&mut self, prec: Prec) -> ExprId {}

  fn parse_atom(&mut self) -> ExprId {
    match self.token.1 {
      TokenKind::Float => {
        let id = self.node_builder.new_expr(Expr {
          kind: ExprKind::NumberLit,
          range: self.token.0.clone(),
          is_recovered: false,
        });
        self.read_token(false);
        id
      }
      TokenKind::String => {
        let id = self.node_builder.new_expr(Expr {
          kind: ExprKind::StringLit,
          range: self.token.0.clone(),
          is_recovered: false,
        });
        self.read_token(false);
        id
      }
      TokenKind::Keyword(Keyword::Inkey) => {
        let id = self.node_builder.new_expr(Expr {
          kind: ExprKind::Inkey,
          range: self.token.0.clone(),
          is_recovered: false,
        });
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
        self.node_builder.new_expr(Expr {
          kind: ExprKind::Unary {
            op: (op_range, op),
            arg,
          },
          range: Range::new(start, end),
          is_recovered: false,
        })
      }
      TokenKind::Keyword(Keyword::Not) => {
        let start = self.token.0.start;
        let op_range = self.token.0.clone();
        self.read_token(false);
        let arg = self.parse_expr(Prec::Not);
        let end = self.node_builder.expr_range(arg).end;
        self.node_builder.new_expr(Expr {
          kind: ExprKind::Unary {
            op: (op_range, UnaryOpKind::Not),
            arg,
          },
          range: Range::new(start, end),
          is_recovered: false,
        })
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
        self.node_builder.new_expr(Expr {
          kind: ExprKind::UserFuncCall {
            func: id_range,
            arg,
          },
          range: Range::new(start, end),
          is_recovered: false,
        })
      }
      TokenKind::SysFunc(kind) => {
        let name_range = self.token.0.clone();
        self.read_token(false);
        let mut args = NonEmptyVec::<[ExprId; 1]>::new();
        let Range { end, .. } = self.parse_paren_args(&mut args.0);
        let range = Range::new(name_range.start, end);
        self.node_builder.new_expr(Expr {
          kind: ExprKind::SysFuncCall {
            func: (name_range, kind),
            args,
          },
          range,
          is_recovered: false,
        })
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
      self.node_builder.new_expr(Expr {
        kind: ExprKind::Index {
          name: id_range,
          indices: args,
        },
        range,
        is_recovered: false,
      })
    } else {
      self.node_builder.new_expr(Expr {
        kind: ExprKind::Ident,
        range: id_range,
        is_recovered: false,
      })
    }
  }

  fn parse_paren_args<U>(&mut self, args: &mut U) -> Range
  where
    U: Extend<ExprId>,
  {
    let start = self.token.0.start;
    self.match_token(TokenKind::Punc(Punc::LParen), false);
    loop {
      if self.token.1 == TokenKind::Punc(Punc::RParen) {
        // TODO eof
        let end = self.token.0.end;
        let range = Range::new(start, end);
        self.read_token(false);
        break range;
      }

      loop {
        let arg = self.parse_expr(Prec::None);
        args.extend_one(arg);

        if self.token.1 == TokenKind::Punc(Punc::Comma) {
          self.read_token(false);
        } else {
          break;
        }
      }
    }
  }

  fn parse_cmd_args<U>(&mut self, args: &mut U) -> Range
  where
    U: Extend<ExprId>,
  {
    let start = self.token.0.start;
    let mut end = start;
    loop {
      if let TokenKind::Punc(Punc::Colon)
      | TokenKind::Keyword(Keyword::Else)
      | TokenKind::Eof = self.token.1
      {
        // TODO eof
        break Range::new(start, end);
      }

      loop {
        let arg = self.parse_expr(Prec::None);
        end = self.node_builder.expr_range(arg).end;
        args.extend_one(arg);

        if self.token.1 == TokenKind::Punc(Punc::Comma) {
          self.read_token(false);
        } else {
          break;
        }
      }
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

impl<'a> LineParser<'a, ArenaNodeBuilder> {
  fn into_line(
    self,
    line: &str,
    eol: Eol,
    label: Option<Label>,
    stmts: Vec<StmtId>,
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
        (Range::new(10, 11), TokenKind::Punc(Punc::Semi)),
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
        read_tokens(r#"  asc chr$ leti%  "#),
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
