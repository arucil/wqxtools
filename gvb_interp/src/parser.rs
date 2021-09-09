use crate::ast::{
  Eol, Expr, ExprId, Keyword, NodeBuilder, Program, ProgramLine, Punc, Range,
  Stmt, StmtId, StmtKind, SysFuncKind, TokenKind,
};
use crate::diagnostic::Diagnostic;
use id_arena::Arena;

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

  let label = match parser.read_label(false) {
    (_, Ok(label)) => Some(label),
    (mut range, Err(err)) => {
      if range.is_empty() {
        range = Range::new(0, line_with_eol.len());
      }
      parser.report_label_error(err, range);
      None
    }
  };

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
}

struct LineParser<'a, T: NodeBuilder> {
  offset: usize,
  input: &'a str,
  token: (Range, TokenKind),
  node_builder: T,
  diagnostics: Vec<Diagnostic>,
}

enum ReadLabelError {
  OutOfBound,
  NoLabel,
}

impl<'a, T: NodeBuilder> LineParser<'a, T> {
  fn new(input: &'a str, node_builder: T) -> Self {
    Self {
      offset: 0,
      input,
      token: (Range::new(0, 0), TokenKind::Eof),
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

  fn report_label_error(&mut self, err: ReadLabelError, range: Range) {
    match err {
      ReadLabelError::NoLabel => {
        self.add_error(range, "缺少行号");
      }
      ReadLabelError::OutOfBound => {
        self.add_error(range, "行号必须在0~9999之间");
      }
    }
  }

  fn read_label(
    &mut self,
    skip_space: bool,
  ) -> (Range, Result<u16, ReadLabelError>) {
    let start = self.offset;
    if skip_space {
      self.skip_space()
    }
    if self.input.is_empty() {
      return (Range::new(start, start), Err(ReadLabelError::NoLabel));
    }
    let mut i = 0;
    loop {
      match self.input.as_bytes().get(i) {
        Some(c) if c.is_ascii_digit() => i += 1,
        _ => break,
      }
    }
    if i != 0 {
      self.advance(i);
      self.input[..i].parse::<u16>().map_or_else(
        |_| (Range::new(start, start), Err(ReadLabelError::OutOfBound)),
        |label| {
          if label < 10000 {
            (Range::new(start, start + i), Ok(label))
          } else {
            (
              Range::new(start, start + i),
              Err(ReadLabelError::OutOfBound),
            )
          }
        },
      )
    } else {
      (Range::new(start, start), Err(ReadLabelError::NoLabel))
    }
  }

  fn skip_space(&mut self) {
    self.advance(count_space(self.input.as_bytes(), 0));
  }

  fn set_token(&mut self, start: usize, kind: TokenKind) {
    self.token = (Range::new(start, self.offset), kind);
  }

  fn read_token(&mut self) {
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
        let len = read_number(self.input.as_bytes(), true);
        let start = self.offset;
        self.advance(len);
        self.set_token(start, TokenKind::Number);
      }
      b'a'..=b'z' | b'A'..=b'Z' => {
        let start = self.offset;
        let mut i = 0;
        loop {
          match self.input.as_bytes().get(i) {
            Some(c) if c.is_ascii_alphanumeric() => i += 1,
            _ => break,
          }
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
        self.read_token();
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

  fn parse_stmts(&mut self, in_if_branch: bool) -> Vec<StmtId> {
    let mut stmts = vec![];
    loop {
      self.skip_space();

      if let Some(b':') = self.input.as_bytes().first() {
        if in_if_branch {
          self.add_error(
            Range::new(self.offset, self.offset + 1),
            "IF语句的分支中不能出现多余的冒号",
          );
        }
        self.advance(1);
        continue;
      }

      match self.read_label(false) {
        (range, label@(Ok(_) | Err(ReadLabelError::OutOfBound))) => {
          let label = match label {
            Ok(label) => Some((range, label)),
            Err(_) => None
          };
          stmts.push(self.node_builder.new_stmt(Stmt {
            kind: StmtKind::GoTo {
              has_goto_keyword: false,
              label,
            },
            range,
            is_recovered: false,
          }));
          if !in_if_branch {
            self.add_error(
              Range::new(self.offset, self.offset + 1),
              "缺少 GOTO 或 GOSUB 关键字",
            );
          }
        }
        (range, Err(ReadLabelError::NoLabel)) => {
          self.read_token();
          stmts.push(self.parse_stmt());
        }
      }
    }
    stmts
  }

  fn parse_stmt(&mut self) -> StmtId {}
}

impl<'a> LineParser<'a, ArenaNodeBuilder> {
  fn into_line(
    self,
    line: &str,
    eol: Eol,
    label: Option<u16>,
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

/// [-+]?\d*(\.\d*)?(E[-+]?\d*)?
pub fn read_number(input: &[u8], allow_space: bool) -> usize {
  let mut i = 0;

  if let Some(b'+' | b'-') = input.first() {
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

  i
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
      parser.read_token();
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
          (Range::new(2, 5), TokenKind::Number),
          (Range::new(5, 7), TokenKind::Ident),
          (Range::new(8, 9), TokenKind::Number),
          (Range::new(9, 9), TokenKind::Eof),
        ]
      );
    }

    #[test]
    fn fraction() {
      assert_eq!(
        read_tokens(r#"  0.14   -147.  .1"#),
        vec![
          (Range::new(2, 6), TokenKind::Number),
          (Range::new(9, 10), TokenKind::Punc(Punc::Minus)),
          (Range::new(10, 14), TokenKind::Number),
          (Range::new(16, 18), TokenKind::Number),
          (Range::new(18, 18), TokenKind::Eof),
        ]
      );
    }

    #[test]
    fn sci_notation() {
      assert_eq!(
        read_tokens(r#"  5E+17 .E3 , 1.e-14 , 12.3e45 , .E+  "#),
        vec![
          (Range::new(2, 7), TokenKind::Number),
          (Range::new(8, 11), TokenKind::Number),
          (Range::new(12, 13), TokenKind::Punc(Punc::Comma)),
          (Range::new(14, 20), TokenKind::Number),
          (Range::new(21, 22), TokenKind::Punc(Punc::Comma)),
          (Range::new(23, 30), TokenKind::Number),
          (Range::new(31, 32), TokenKind::Punc(Punc::Comma)),
          (Range::new(33, 36), TokenKind::Number),
          (Range::new(38, 38), TokenKind::Eof),
        ]
      );
    }

    #[test]
    fn space_in_number() {
      assert_eq!(
        read_tokens(r#"  123 456  7   8 , 12 . 3 , 12 . 3 E + 4 5, . e  -  "#),
        vec![
          (Range::new(2, 16), TokenKind::Number),
          (Range::new(17, 18), TokenKind::Punc(Punc::Comma)),
          (Range::new(19, 25), TokenKind::Number),
          (Range::new(26, 27), TokenKind::Punc(Punc::Comma)),
          (Range::new(28, 42), TokenKind::Number),
          (Range::new(42, 43), TokenKind::Punc(Punc::Comma)),
          (Range::new(44, 50), TokenKind::Number),
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
