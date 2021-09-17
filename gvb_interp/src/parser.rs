use self::symbol::{Nonterminal, Symbol, SymbolSet};
use crate::ast::{
  BinaryOpKind, Datum, Eol, Expr, ExprId, ExprKind, FieldSpec, FileMode,
  InputSource, Keyword, Label, NodeBuilder, NonEmptyVec, ParseLabelError,
  PrintElement, Program, ProgramLine, Punc, Range, Stmt, StmtId, StmtKind,
  SysFuncKind, TokenKind, UnaryOpKind, WriteElement,
};
use crate::diagnostic::Diagnostic;
use id_arena::Arena;
use smallvec::{smallvec, Array, SmallVec};
use std::fmt::Write;

pub mod symbol;

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
  if !matches!(line.as_bytes().first(), Some(b' ')) {
    parser.read_token(true);
    if parser.token.1 == TokenKind::Label {
      match parser.label_value.take().unwrap() {
        Ok(l) => label = Some(l),
        Err(err) => parser.report_label_error(err, parser.token.0.clone()),
      }
      parser.read_token(true);
    } else {
      parser.report_label_error(
        ParseLabelError::NotALabel,
        Range::new(0, line_with_eol.len()),
      );
    }
  } else {
    parser.report_label_error(
      ParseLabelError::NotALabel,
      Range::new(0, line_with_eol.len()),
    );
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

  fn stmt_node(&self, stmt: StmtId) -> &Stmt {
    &self.stmt_arena[stmt]
  }

  fn expr_node(&self, expr: ExprId) -> &Expr {
    &self.expr_arena[expr]
  }
}

struct LineParser<'a, T: NodeBuilder> {
  offset: usize,
  input: &'a str,
  token: (Range, TokenKind),
  label_value: Option<Result<Label, ParseLabelError>>,
  last_token_end: usize,
  node_builder: T,
  diagnostics: Vec<Diagnostic>,
  expected_symbols_at_eof: Option<SymbolSet>,
  first_symbols: SymbolSet,
  /// Only contains terminals.
  follow_symbols: SymbolSet,
}

macro_rules! extend_symbol {
  ($set:expr, (kw $($kw:ident)+)) => {{
    $(
      $set.extend_one(Symbol::Term(TokenKind::Keyword(Keyword::$kw)))
    );*
  }};
  ($set:expr, (punc $($p:ident)+)) => {{
    $(
      $set.extend_one(Symbol::Term(TokenKind::Punc(Punc::$p)))
    );*
  }};
  ($set:expr, (nt $($nt:ident)*)) => {{
    $(
      $set.extend_one(Symbol::Nonterm(Nonterminal::$nt))
    );*
  }};
  ($set:expr, (id)) => {
    $set.extend_one(Symbol::Term(TokenKind::Ident))
  };
  ($set:expr, (t $($nt:ident)*)) => {{
    $(
      $set.extend(Nonterminal::$nt.first_symbols())
    );*
  }};
}

macro_rules! setup_first {
  ($self:ident, $old_first:ident : $($elem:tt)*) => {
    {
      $self.first_symbols = $old_first.clone();
      $(extend_symbol!($self.first_symbols, $elem));*
    }
  };
}

macro_rules! setup_follow {
  ($self:ident, $old_follow:ident : $($elem:tt)*) => {
    {
      $self.follow_symbols = $old_follow.clone();
      $(extend_symbol!($self.follow_symbols, $elem));*
    }
  };
}

impl<'a, T: NodeBuilder> LineParser<'a, T> {
  fn new(input: &'a str, node_builder: T) -> Self {
    Self {
      offset: 0,
      input,
      token: (Range::new(0, 0), TokenKind::Eof),
      label_value: None,
      last_token_end: 0,
      node_builder,
      diagnostics: vec![],
      expected_symbols_at_eof: None,
      first_symbols: SymbolSet::new(),
      follow_symbols: SymbolSet::new(),
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
    self.last_token_end = self.token.0.end;

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

        let mut i = 0;
        let mut seg_start = 0;
        let mut in_seg = false;

        loop {
          match self.input.as_bytes().get(i) {
            Some(c) if c.is_ascii_alphanumeric() => {
              if !in_seg {
                seg_start = i;
                in_seg = true;
              }
              i += 1;
            }
            Some(b'%' | b'$') => {
              i += 1;
              if in_seg {
                let str = self.input[seg_start..i].to_ascii_lowercase();
                if str.parse::<Keyword>().is_ok()
                  || str.parse::<SysFuncKind>().is_ok()
                {
                  i = seg_start;
                }
              }
              break;
            }
            c => {
              if in_seg {
                in_seg = false;
                let str = self.input[seg_start..i].to_ascii_lowercase();
                if str.parse::<Keyword>().is_ok()
                  || str.parse::<SysFuncKind>().is_ok()
                {
                  i = seg_start;
                  break;
                }
              }
              if c == Some(&b' ') {
                i += 1;
              } else {
                break;
              }
            }
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
        Some(b'"') => {
          i += 1;
          break;
        }
        Some(_) => i += 1,
        _ => break,
      }
    }
    i
  }

  #[must_use]
  fn match_token(
    &mut self,
    token: TokenKind,
    read_label: bool,
    show_error: bool,
  ) -> Result<Range, ()> {
    if self.token.1 == token {
      let range = self.token.0.clone();
      self.read_token(read_label);
      Ok(range)
    } else {
      if self.token.1 == TokenKind::Eof
        && self.expected_symbols_at_eof.is_none()
      {
        self.expected_symbols_at_eof = Some(self.first_symbols.clone());
      }
      if show_error {
        self.report_mismatch_token_error();
      }
      self.recover(read_label);

      Err(())
    }
  }

  fn report_mismatch_token_error(&mut self) {
    let mut msg = "语法错误。期望是：".to_owned();
    let mut comma = false;
    for sym in &self.first_symbols {
      if comma {
        msg += ", ";
      }
      comma = true;
      match sym {
        Symbol::Term(token) => match token {
          TokenKind::Ident => msg += "标识符",
          TokenKind::Label => msg += "行号",
          TokenKind::Float => msg += "实数",
          TokenKind::String => msg += "字符串",
          TokenKind::Punc(p) => write!(&mut msg, "\"{:?}\"", p).unwrap(),
          TokenKind::Keyword(p) => write!(&mut msg, "{:?}", p).unwrap(),
          TokenKind::SysFunc(p) => write!(&mut msg, "{:?}", p).unwrap(),
          TokenKind::Eof => msg += "行尾",
        },
        Symbol::Nonterm(n) => match n {
          Nonterminal::Expr => msg += "表达式",
          Nonterminal::Stmt => msg += "语句",
        },
      }
    }
    self.add_error(self.token.0.clone(), msg);
  }

  fn recover(&mut self, read_label: bool) {
    while self.token.1 != TokenKind::Eof
      && !self.follow_symbols.contains_token(self.token.1)
    {
      self.read_token(read_label);
    }
  }

  fn parse_stmts(&mut self, in_if_branch: bool) -> SmallVec<[StmtId; 1]> {
    let mut stmts = smallvec![];
    loop {
      match self.token.1 {
        TokenKind::Punc(Punc::Colon) => {
          if in_if_branch {
            self.add_error(
              self.token.0.clone(),
              "IF 语句的分支中不能出现多余的冒号",
            );
          } else {
            stmts.push(self.node_builder.new_stmt(Stmt {
              kind: StmtKind::NoOp,
              range: self.token.0.clone(),
            }));
          }
          self.read_token(in_if_branch);
        }
        TokenKind::Eof => return stmts,
        TokenKind::Keyword(Keyword::Else) => {
          if in_if_branch {
            return stmts;
          } else {
            self.add_error(self.token.0.clone(), "ELSE 不能出现在 IF 语句之外");
            self.read_token(in_if_branch);
          }
        }
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
              self.node_builder.stmt_node(stmt).range.clone(),
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
      Label => match self.label_value.take().unwrap() {
        Ok(label) => {
          let range = self.token.0.clone();
          self.read_token(true);
          self.node_builder.new_stmt(Stmt {
            kind: StmtKind::GoTo {
              label: Some((range.clone(), label)),
              has_goto_keyword: false,
            },
            range,
          })
        }
        Err(err) => {
          let range = self.token.0.clone();
          self.report_label_error(err, range.clone());
          self.read_token(true);
          self.node_builder.new_stmt(Stmt {
            kind: StmtKind::NoOp,
            range,
          })
        }
      },
      Eof => unreachable!(),
      _ => {
        todo!("expect stmt")
      }
    }
  }

  fn parse_unary_cmd(&mut self, ctor: fn(ExprId) -> StmtKind) -> StmtId {
    let start = self.token.0.start;
    self.read_token(false);
    let arg = self.parse_expr();
    self.node_builder.new_stmt(Stmt {
      kind: ctor(arg),
      range: Range::new(start, self.last_token_end),
    })
  }

  fn parse_close_stmt(&mut self) -> StmtId {
    let start = self.token.0.start;
    self.read_token(false);
    if self.token.1 == TokenKind::Punc(Punc::Hash) {
      self.read_token(false);
    }
    let filenum = self.parse_expr();
    self.node_builder.new_stmt(Stmt {
      kind: StmtKind::Close { filenum },
      range: Range::new(start, self.last_token_end),
    })
  }

  fn parse_data_stmt(&mut self) -> StmtId {
    let start = self.token.0.start;
    let mut data = NonEmptyVec::<[Datum; 1]>::new();
    loop {
      self.skip_space();
      let datum_start = self.offset;
      if let Some(b'"') = self.input.as_bytes().first() {
        self.advance(self.read_quoted_string());
        data.push(Datum {
          range: Range::new(datum_start, self.offset),
          is_quoted: true,
        });
        self.skip_space();
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
        });
      }

      match self.input.as_bytes().first() {
        Some(b':') | None => break,
        Some(b',') => self.advance(1),
        _ => {
          self.add_error(Range::new(self.offset, self.offset + 1), "缺少逗号");
        }
      }
    }
    self.read_token(true);
    self.node_builder.new_stmt(Stmt {
      kind: StmtKind::Data(data),
      range: Range::new(start, self.offset),
    })
  }

  fn parse_def_stmt(&mut self) -> StmtId {
    let def_range = self.token.0.clone();
    self.read_token(false);

    let old_first = self.first_symbols.clone();
    let old_follow = self.follow_symbols.clone();
    setup_first! { self, old_first : (kw Fn) }
    setup_follow! { self, old_follow : (id) (punc LParen Eq) }
    if self
      .match_token(TokenKind::Keyword(Keyword::Fn), false, false)
      .is_err()
    {
      self.add_error(def_range.clone(), "DEF 之后缺少 FN 关键字");
    }

    setup_first! { self, old_first : (id) }
    setup_follow! { self, old_follow : (punc LParen Eq) }
    let name_range;
    match self.match_token(TokenKind::Ident, false, false) {
      Ok(range) => name_range = Some(range),
      Err(()) => {
        self.add_error(def_range.clone(), "DEF 语句缺少函数名称");
        name_range = None;
      }
    }

    setup_first! { self, old_first : (punc LParen) }
    setup_follow! { self, old_follow : (id) (punc RParen Eq) }
    if self
      .match_token(TokenKind::Punc(Punc::LParen), false, false)
      .is_err()
    {
      if let Some(name_range) = &name_range {
        self.add_error(name_range.clone(), "函数名称之后缺少左括号");
      }
    }

    setup_first! { self, old_first : (id) }
    setup_follow! { self, old_follow : (punc RParen Eq) }
    let param_range;
    match self.match_token(TokenKind::Ident, false, false) {
      Ok(range) => param_range = Some(range),
      Err(()) => {
        if let Some(name_range) = &name_range {
          self.add_error(name_range.clone(), "缺少函数参数变量");
        }
        param_range = None;
      }
    }

    setup_first! { self, old_first : (punc RParen) }
    setup_follow! { self, old_follow : (punc Eq) }
    if self
      .match_token(TokenKind::Punc(Punc::RParen), false, false)
      .is_err()
    {
      if let Some(param_range) = &param_range {
        self.add_error(param_range.clone(), "函数参数之后缺少右括号");
      }
    }

    setup_first! { self, old_first : (punc Eq) }
    setup_follow! { self, old_follow : (t Expr) }
    if self
      .match_token(TokenKind::Punc(Punc::Eq), false, false)
      .is_err()
    {
      self.add_error(def_range.clone(), "DEF 语句缺少等号");
    }

    setup_first! { self, old_first : }
    setup_follow! { self, old_follow : }
    let body = self.parse_expr();

    self.follow_symbols = old_follow;
    self.first_symbols = old_first;

    self.node_builder.new_stmt(Stmt {
      kind: StmtKind::Def {
        name: name_range,
        param: param_range,
        body,
      },
      range: Range::new(def_range.start, self.last_token_end),
    })
  }

  fn parse_dim_stmt(&mut self) -> StmtId {
    let start = self.token.0.start;
    self.read_token(false);
    let vars = self.parse_lvalue_list();
    self.node_builder.new_stmt(Stmt {
      kind: StmtKind::Dim(vars),
      range: Range::new(start, self.last_token_end),
    })
  }

  fn parse_field_stmt(&mut self) -> StmtId {
    let start = self.token.0.start;
    self.read_token(false);
    if let TokenKind::Punc(Punc::Hash) = self.token.1 {
      self.read_token(false);
    }
    let filenum = self.parse_expr();
    let mut fields = NonEmptyVec::<[FieldSpec; 1]>::new();
    self.match_token(TokenKind::Punc(Punc::Comma), false);
    let field = self.parse_field_spec();
    fields.push(field);
    while self.token.1 == TokenKind::Punc(Punc::Comma) {
      self.read_token(false);
      let field = self.parse_field_spec();
      fields.push(field);
    }
    self.node_builder.new_stmt(Stmt {
      kind: StmtKind::Field { filenum, fields },
      range: Range::new(start, self.last_token_end),
    })
  }

  fn parse_field_spec(&mut self) -> FieldSpec {
    let start = self.token.0.start;
    let len = self.parse_expr();
    self.put_back_token();
    self.read_as();
    self.read_token(false);
    let var = self.parse_lvalue();
    FieldSpec {
      range: Range::new(start, self.last_token_end),
      len,
      var,
    }
  }

  fn read_as(&mut self) {
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
  }

  fn parse_for_stmt(&mut self) -> StmtId {
    let start = self.token.0.start;
    self.read_token(false);
    let id_range = self.match_token(TokenKind::Ident, false);
    self.match_token(TokenKind::Punc(Punc::Eq), false);
    let from = self.parse_expr();
    self.match_token(TokenKind::Keyword(Keyword::To), false);
    let to = self.parse_expr();
    let mut step = None;
    if let TokenKind::Keyword(Keyword::Step) = self.token.1 {
      self.read_token(false);
      let s = self.parse_expr();
      step = Some(s);
    }
    self.node_builder.new_stmt(Stmt {
      kind: StmtKind::For {
        var: id_range,
        start: from,
        end: to,
        step,
      },
      range: Range::new(start, self.last_token_end),
    })
  }

  fn parse_go_stmt(
    &mut self,
    ctor: fn(Option<(Range, Label)>) -> StmtKind,
  ) -> StmtId {
    let start = self.token.0.start;
    self.read_token(true);
    let mut label = None;
    if self.token.1 == TokenKind::Label {
      match self.label_value.take().unwrap() {
        Ok(l) => {
          label = Some((self.token.0.clone(), l));
        }
        Err(err) => self.report_label_error(err, self.token.0.clone()),
      }
      self.read_token(false);
    }
    self.node_builder.new_stmt(Stmt {
      kind: ctor(label),
      range: Range::new(start, self.last_token_end),
    })
  }

  fn parse_get_put_stmt(&mut self, is_put: bool) -> StmtId {
    let start = self.token.0.start;
    self.read_token(false);
    if self.token.1 == TokenKind::Punc(Punc::Hash) {
      self.read_token(false);
    }
    let filenum = self.parse_expr();
    self.match_token(TokenKind::Punc(Punc::Comma), false);
    let record = self.parse_expr();
    let kind = if is_put {
      StmtKind::Put { filenum, record }
    } else {
      StmtKind::Get { filenum, record }
    };
    self.node_builder.new_stmt(Stmt {
      kind,
      range: Range::new(start, self.last_token_end),
    })
  }

  fn parse_if_stmt(&mut self) -> StmtId {
    let start = self.token.0.start;
    self.read_token(false);
    let cond = self.parse_expr();
    let conseq;
    if let TokenKind::Keyword(kw @ (Keyword::Then | Keyword::Goto)) =
      self.token.1
    {
      let then_range = self.token.0.clone();
      self.read_token(true);
      conseq = self.parse_stmts(true);
      if conseq.is_empty() {
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
      if stmts.is_empty() {
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
      range: Range::new(start, self.last_token_end),
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
        source = InputSource::Keyboard(Some(prompt_range));
      }
      TokenKind::Punc(Punc::Hash) => {
        self.read_token(false);
        let filenum = self.parse_expr();
        self.match_token(TokenKind::Punc(Punc::Comma), false);
        source = InputSource::File(filenum);
      }
      _ => {
        source = InputSource::Keyboard(None);
      }
    }

    let vars = self.parse_lvalue_list();
    self.node_builder.new_stmt(Stmt {
      kind: StmtKind::Input { source, vars },
      range: Range::new(start, self.last_token_end),
    })
  }

  fn parse_lvalue_list(&mut self) -> NonEmptyVec<[ExprId; 1]> {
    let mut vars = NonEmptyVec::<[ExprId; 1]>::new();
    let var = self.parse_lvalue();
    vars.push(var);
    while let TokenKind::Punc(Punc::Comma) = self.token.1 {
      self.read_token(false);
      let var = self.parse_lvalue();
      vars.push(var);
    }
    vars
  }

  fn parse_assign_stmt(&mut self, has_let: bool) -> StmtId {
    let start = self.token.0.start;
    if has_let {
      self.read_token(false);
    }
    let var = self.parse_lvalue();
    self.match_token(TokenKind::Punc(Punc::Eq), false);
    let value = self.parse_expr();
    self.node_builder.new_stmt(Stmt {
      kind: StmtKind::Let { var, value },
      range: Range::new(start, self.last_token_end),
    })
  }

  fn parse_locate_stmt(&mut self) -> StmtId {
    let start = self.token.0.start;
    self.read_token(false);
    let row;
    if let TokenKind::Punc(Punc::Comma) = self.token.1 {
      row = None;
    } else {
      let r = self.parse_expr();
      row = Some(r);
    }
    let column;
    if let TokenKind::Punc(Punc::Comma) = self.token.1 {
      self.read_token(false);
      let r = self.parse_expr();
      column = Some(r);
    } else {
      column = None;
    }
    self.node_builder.new_stmt(Stmt {
      kind: StmtKind::Locate { row, column },
      range: Range::new(start, self.last_token_end),
    })
  }

  fn parse_set_stmt(&mut self, ctor: fn(ExprId, ExprId) -> StmtKind) -> StmtId {
    let start = self.token.0.start;
    self.read_token(false);
    let var = self.parse_lvalue();
    self.match_token(TokenKind::Punc(Punc::Eq), false);
    let value = self.parse_expr();
    self.node_builder.new_stmt(Stmt {
      kind: ctor(var, value),
      range: Range::new(start, self.last_token_end),
    })
  }

  fn parse_next_stmt(&mut self) -> StmtId {
    let start = self.token.0.start;
    self.read_token(false);
    let mut vars = SmallVec::<[Range; 1]>::new();
    if let TokenKind::Ident = self.token.1 {
      loop {
        let var_range = self.match_token(TokenKind::Ident, false);
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
      range: Range::new(start, self.last_token_end),
    })
  }

  fn parse_on_stmt(&mut self) -> StmtId {
    let start = self.token.0.start;
    self.read_token(false);
    let cond = self.parse_expr();
    let is_sub;
    match self.token.1 {
      TokenKind::Keyword(Keyword::Gosub) => {
        self.read_token(true);
        is_sub = true;
      }
      TokenKind::Keyword(Keyword::Goto) => {
        self.read_token(true);
        is_sub = false;
      }
      _ => {
        todo!("expect GOSUB or GOTO, skip to digit, read_token(true)")
      }
    }

    let mut labels = NonEmptyVec::<[(Range, Option<Label>); 2]>::new();
    if self.token.1 == TokenKind::Label {
      match self.label_value.take().unwrap() {
        Ok(label) => {
          labels.push((self.token.0.clone(), Some(label)));
          self.read_token(true);
        }
        Err(err) => {
          self.report_label_error(err, self.token.0.clone());
        }
      }
    } else {
      labels.push((Range::new(self.token.0.start, self.token.0.start), None));
    };

    while let TokenKind::Punc(Punc::Comma) = self.token.1 {
      self.read_token(true);

      if self.token.1 == TokenKind::Label {
        match self.label_value.take().unwrap() {
          Ok(label) => {
            labels.push((self.token.0.clone(), Some(label)));
            self.read_token(true);
          }
          Err(err) => {
            self.report_label_error(err, self.token.0.clone());
          }
        }
      } else {
        labels.push((Range::new(self.token.0.start, self.token.0.start), None));
      };
    }

    self.node_builder.new_stmt(Stmt {
      kind: StmtKind::On {
        cond,
        labels,
        is_sub,
      },
      range: Range::new(start, self.last_token_end),
    })
  }

  fn parse_open_stmt(&mut self) -> StmtId {
    let start = self.token.0.start;
    self.read_token(false);
    let filename = self.parse_expr();
    if let TokenKind::Keyword(Keyword::For) = self.token.1 {
      self.skip_space();
    } else {
      self.put_back_token();
    }

    let mode;
    if self.input.len() >= 5 {
      if self.input.as_bytes()[..5].eq_ignore_ascii_case(b"input") {
        mode = FileMode::Input;
        self.advance(5);
      } else {
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
      }
    } else {
      todo!("expect file mode, skip to A or #")
    }

    self.skip_space();

    self.read_as();

    self.read_token(false);
    if let TokenKind::Punc(Punc::Hash) = self.token.1 {
      self.read_token(false);
    }

    let filenum = self.parse_expr();

    let mut len = None;
    if let TokenKind::SysFunc(SysFuncKind::Len) = self.token.1 {
      self.read_token(false);
      self.match_token(TokenKind::Punc(Punc::Eq), false);
      let l = self.parse_expr();
      len = Some(l);
    }

    self.node_builder.new_stmt(Stmt {
      kind: StmtKind::Open {
        filename,
        mode,
        filenum,
        len,
      },
      range: Range::new(start, self.last_token_end),
    })
  }

  fn parse_poke_stmt(&mut self) -> StmtId {
    let start = self.token.0.start;
    self.read_token(false);
    let addr = self.parse_expr();
    self.match_token(TokenKind::Punc(Punc::Comma), false);
    let value = self.parse_expr();
    self.node_builder.new_stmt(Stmt {
      kind: StmtKind::Poke { addr, value },
      range: Range::new(start, self.last_token_end),
    })
  }

  fn parse_print_stmt(&mut self) -> StmtId {
    let start = self.token.0.start;
    self.read_token(false);

    let mut elems = SmallVec::<[PrintElement; 2]>::new();

    loop {
      match self.token.1 {
        TokenKind::Punc(Punc::Comma) => {
          elems.push(PrintElement::Comma);
          self.read_token(false);
        }
        TokenKind::Punc(Punc::Semicolon) => {
          elems.push(PrintElement::Semicolon);
          self.read_token(false);
        }
        TokenKind::Punc(Punc::Colon)
        | TokenKind::Keyword(Keyword::Else)
        | TokenKind::Eof => break,
        _ => {
          let expr = self.parse_expr();
          elems.push(PrintElement::Expr(expr));
        }
      }
    }

    self.node_builder.new_stmt(Stmt {
      kind: StmtKind::Print(elems),
      range: Range::new(start, self.last_token_end),
    })
  }

  fn parse_read_stmt(&mut self) -> StmtId {
    let start = self.token.0.start;
    self.read_token(false);
    let vars = self.parse_lvalue_list();
    self.node_builder.new_stmt(Stmt {
      kind: StmtKind::Read(vars),
      range: Range::new(start, self.last_token_end),
    })
  }

  fn parse_swap_stmt(&mut self) -> StmtId {
    let start = self.token.0.start;
    self.read_token(false);
    let left = self.parse_lvalue();
    self.match_token(TokenKind::Punc(Punc::Comma), false);
    let right = self.parse_lvalue();
    self.node_builder.new_stmt(Stmt {
      kind: StmtKind::Swap { left, right },
      range: Range::new(start, self.last_token_end),
    })
  }

  fn parse_write_stmt(&mut self) -> StmtId {
    let start = self.token.0.start;
    self.read_token(false);
    let filenum;
    if let TokenKind::Punc(Punc::Hash) = self.token.1 {
      self.read_token(false);
      filenum = Some(self.parse_expr());
      self.match_token(TokenKind::Punc(Punc::Comma), false);
    } else {
      filenum = None;
    }

    let mut data = NonEmptyVec::<[WriteElement; 1]>::new();
    let mut datum = self.parse_expr();

    loop {
      if let TokenKind::Punc(Punc::Comma) = self.token.1 {
        data.push(WriteElement { datum, comma: true });
        self.read_token(false);
      } else {
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
      datum = self.parse_expr();
    }

    self.node_builder.new_stmt(Stmt {
      kind: StmtKind::Write { filenum, data },
      range: Range::new(start, self.last_token_end),
    })
  }

  fn parse_cmd<A: Array<Item = ExprId>>(
    &mut self,
    ctor: fn(NonEmptyVec<A>) -> StmtKind,
  ) -> StmtId {
    let start = self.token.0.start;
    self.read_token(false);
    let mut args = NonEmptyVec::<A>::new();
    self.parse_cmd_args(&mut args.0);
    self.node_builder.new_stmt(Stmt {
      kind: ctor(args),
      range: Range::new(start, self.last_token_end),
    })
  }

  fn parse_nullary_cmd(&mut self, kind: StmtKind) -> StmtId {
    let range = self.token.0.clone();
    self.read_token(false);
    self.node_builder.new_stmt(Stmt { kind, range })
  }

  fn parse_rem_stmt(
    &mut self,
    ctor: fn(Range) -> StmtKind,
    in_if_branch: bool,
  ) -> StmtId {
    self.skip_space();
    let start = self.offset;
    self.skip_line();
    let id = self.node_builder.new_stmt(Stmt {
      kind: ctor(Range::new(start, self.offset)),
      range: Range::new(self.token.0.start, self.offset),
    });
    self.read_token(in_if_branch);
    id
  }

  fn parse_expr(&mut self) -> ExprId {
    let old_first = self.first_symbols.clone();
    let old_follow = self.follow_symbols.clone();
    setup_first! { self, old_first : (nt Expr) }
    setup_follow! { self, old_follow :
      (punc Eq Gt Lt Plus Minus Times Slash Caret)
      (kw And Or)
    }

    let expr = self.parse_expr_prec(Prec::None);

    self.follow_symbols = old_follow;
    self.first_symbols = old_first;

    expr
  }

  fn parse_expr_prec(&mut self, prec: Prec) -> ExprId {
    let start = self.token.0.start;
    let mut lhs = self.parse_atom();

    while token_prec(self.token.1) > prec {
      let tok = self.token.1;
      let op = self.read_binary_op();
      let rhs = self.parse_expr_prec(token_prec(tok));
      lhs = self.node_builder.new_expr(Expr::new(
        ExprKind::Binary { lhs, op, rhs },
        Range::new(start, self.last_token_end),
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
    self.read_token(false);
    (Range::new(range.start, self.last_token_end), op)
  }

  fn parse_atom(&mut self) -> ExprId {
    match self.token.1 {
      TokenKind::Float | TokenKind::Label => {
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
        let op = match op {
          Punc::Plus => UnaryOpKind::Pos,
          Punc::Minus => UnaryOpKind::Neg,
          _ => unreachable!(),
        };
        self.read_token(false);
        let arg = self.parse_expr_prec(Prec::Neg);
        let kind = ExprKind::Unary {
          op: (op_range, op),
          arg,
        };
        self
          .node_builder
          .new_expr(Expr::new(kind, Range::new(start, self.last_token_end)))
      }
      TokenKind::Keyword(Keyword::Not) => {
        let start = self.token.0.start;
        let op_range = self.token.0.clone();
        self.read_token(false);
        let arg = self.parse_expr_prec(Prec::Not);
        let kind = ExprKind::Unary {
          op: (op_range, UnaryOpKind::Not),
          arg,
        };
        self
          .node_builder
          .new_expr(Expr::new(kind, Range::new(start, self.last_token_end)))
      }
      TokenKind::Punc(Punc::LParen) => {
        let paren_range = self.token.0.clone();
        let old_follow = self.follow_symbols.clone();
        self.read_token(false);

        setup_follow! { self, old_follow : (punc RParen) }
        let expr = self.parse_expr();

        setup_follow! { self, old_follow : }
        if self
          .match_token(TokenKind::Punc(Punc::RParen), false, false)
          .is_err()
        {
          self.add_error(paren_range, "缺少匹配的右括号");
        }
        self.follow_symbols = old_follow;
        expr
      }
      TokenKind::Keyword(Keyword::Fn) => {
        let fn_range = self.token.0.clone();
        let old_first = self.first_symbols.clone();
        let old_follow = self.follow_symbols.clone();
        self.read_token(false);

        setup_first! { self, old_first : (id) }
        setup_follow! { self, old_follow : (punc LParen) (id) }
        let id_range;
        match self.match_token(TokenKind::Ident, false, false) {
          Ok(range) => id_range = Some(range),
          Err(()) => {
            self.add_error(fn_range.clone(), "FN 之后缺少函数名称");
            id_range = None;
          }
        }

        setup_first! { self, old_first : (punc LParen) }
        setup_follow! { self, old_follow : (punc RParen) (id) }
        if self
          .match_token(TokenKind::Punc(Punc::LParen), false, false)
          .is_err()
        {
          if let Some(id_range) = &id_range {
            self.add_error(id_range.clone(), "函数名称之后缺少左括号");
          }
        }

        setup_first! { self, old_first : }
        setup_follow! { self, old_follow : }
        let arg = self.parse_expr();

        setup_first! { self, old_first : (punc RParen) }
        setup_follow! { self, old_follow : }
        if self
          .match_token(TokenKind::Punc(Punc::RParen), false, false)
          .is_err()
        {
          let arg = self.node_builder.expr_node(arg);
          if !matches!(&arg.kind, ExprKind::Error) {
            self.add_error(arg.range.clone(), "缺少右括号");
          }
        }

        self.follow_symbols = old_follow;
        self.first_symbols = old_first;

        let kind = ExprKind::UserFuncCall {
          func: id_range,
          arg,
        };
        self.node_builder.new_expr(Expr::new(
          kind,
          Range::new(fn_range.start, self.last_token_end),
        ))
      }
      TokenKind::SysFunc(kind) => {
        let name_range = self.token.0.clone();
        self.read_token(false);
        let mut args = NonEmptyVec::<[ExprId; 1]>::new();
        self.parse_paren_args(&mut args.0);
        let range = Range::new(name_range.start, self.last_token_end);
        let kind = ExprKind::SysFuncCall {
          func: (name_range, kind),
          args,
        };
        self.node_builder.new_expr(Expr::new(kind, range))
      }
      TokenKind::Ident => self.parse_lvalue(),
      _ => {
        let start = self.token.0.start;
        self.report_mismatch_token_error();
        self.recover(false);
        self.node_builder.new_expr(Expr::new(
          ExprKind::Error,
          Range::new(start, self.token.0.start),
        ))
      }
    }
  }

  fn parse_lvalue(&mut self) -> ExprId {
    let id_range = self.match_token(TokenKind::Ident, false);
    if self.token.1 == TokenKind::Punc(Punc::LParen) {
      let mut args = NonEmptyVec::<[ExprId; 1]>::new();
      self.parse_paren_args(&mut args.0);
      let range = Range::new(id_range.start, self.last_token_end);
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

  fn parse_paren_args<U>(&mut self, args: &mut U)
  where
    U: Extend<ExprId>,
  {
    self.match_token(TokenKind::Punc(Punc::LParen), false);
    let arg = self.parse_expr();
    args.extend_one(arg);
    while self.token.1 == TokenKind::Punc(Punc::Comma) {
      self.read_token(false);

      let arg = self.parse_expr();
      args.extend_one(arg);
    }

    if self.token.1 == TokenKind::Punc(Punc::RParen) {
      self.read_token(false);
    } else {
      todo!("expect ) or ,")
    }
  }

  fn parse_cmd_args<U>(&mut self, args: &mut U)
  where
    U: Extend<ExprId>,
  {
    let arg = self.parse_expr();
    args.extend_one(arg);
    while self.token.1 == TokenKind::Punc(Punc::Comma) {
      self.read_token(false);

      let arg = self.parse_expr();
      args.extend_one(arg);
    }

    if !matches!(
      self.token.1,
      TokenKind::Punc(Punc::Colon)
        | TokenKind::Keyword(Keyword::Else)
        | TokenKind::Eof
    ) {
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

  fn stmt_node(&self, stmt: StmtId) -> &Stmt {
    unimplemented!()
  }

  fn expr_node(&self, expr: ExprId) -> &Expr {
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
      r#"LoCaTe 3,2:PrInT "啊A":lOcAtE 3,18-LeN(STr$(Et)):PriNT ET:DRaW 100,38"#,
    );
    assert_debug_snapshot!(tokens);
  }
}

#[cfg(test)]
mod parser_tests {
  use super::*;
  use insta::assert_snapshot;

  #[test]
  fn assign() {
    let line = r#"10 ab$ = 3"#;
    assert_snapshot!(parse_line(line).to_string(line));
  }

  #[test]
  fn two_assign() {
    let line = r#"10 ab$=3: foo %=2+2"#;
    assert_snapshot!(parse_line(line).to_string(line));
  }

  #[test]
  fn no_label() {
    let line = r#"ab$=3 :fO3 %=2-3*1  "#;
    assert_snapshot!(parse_line(line).to_string(line));
  }

  #[test]
  fn no_stmts() {
    let line = r#"10"#;
    assert_snapshot!(parse_line(line).to_string(line));
  }

  #[test]
  fn blank_line() {
    let line = r#"    "#;
    assert_snapshot!(parse_line(line).to_string(line));
  }

  #[test]
  fn empty_line() {
    let line = r#""#;
    assert_snapshot!(parse_line(line).to_string(line));
  }

  #[test]
  fn colon_line() {
    let line = r#"10 :"#;
    assert_snapshot!(parse_line(line).to_string(line));
  }

  #[test]
  fn nullary_cmd() {
    let line = r#"10 ::::Beep::enD:::fLAsh:::inkEy$::"#;
    assert_snapshot!(parse_line(line).to_string(line));
  }

  #[test]
  fn real_world_example() {
    let line = r#"17 LOCaTe 3,2:PRinT "A";,3:locaTE 3,18-LeN(StR$(ET)):PRINT ET:DRAW 100,38"#;
    assert_snapshot!(parse_line(line).to_string(line));
  }

  #[test]
  fn r#box() {
    let line = r#"10 boX 2*3,A/2,INT(INKEY$),1 : BOX 1,2,3,4,-0"#;
    assert_snapshot!(parse_line(line).to_string(line));
  }

  #[test]
  fn unary_cmd() {
    let line =
      r#"10 calL  1340+A*10: CALl T%: play A b $+"DE#" :while not a(i)"#;
    assert_snapshot!(parse_line(line).to_string(line));
  }

  #[test]
  fn close() {
    let line = r#"10 close # 2+1:clOSe 2+1"#;
    assert_snapshot!(parse_line(line).to_string(line));
  }

  #[test]
  fn rem() {
    let line = r#"10 cls:rem machine: tc808"#;
    assert_snapshot!(parse_line(line).to_string(line));
  }

  #[test]
  fn data1() {
    let line = r#"10 daTA   ,," : A,",12,3  : Data A  ,A B  C,  , "#;
    assert_snapshot!(parse_line(line).to_string(line));
  }

  #[test]
  fn data2() {
    let line = r#"10 daTA   1,  2  ,  : data "aA","bB"#;
    assert_snapshot!(parse_line(line).to_string(line));
  }

  #[test]
  fn def() {
    let line =
      r#"10   def Fn a b c%(x Y 3  1) = sin(X / 2) : DEF   fN  f (X)=fn F(x)"#;
    assert_snapshot!(parse_line(line).to_string(line));
  }

  #[test]
  fn dim() {
    let line = r#"10 DIm  A:dIm  B$(k+1,tan(x,y)) , C, O(3*2)"#;
    assert_snapshot!(parse_line(line).to_string(line));
  }

  #[test]
  fn field() {
    let line = r#"10 field # 1*2 , 3*5-1ASA b $  : fiEld 1*2 , 1  A Sx%(3) , 7.5ASA$(A,B)"#;
    assert_snapshot!(parse_line(line).to_string(line));
  }

  #[test]
  fn r#for() {
    let line = r#"10 for I%=K*2 to I%*2: FoR i=0 To 1e3 sTep k"#;
    assert_snapshot!(parse_line(line).to_string(line));
  }

  #[test]
  fn get_put() {
    let line = r#"10 Get # 7/2 , 2*5 : PuT 3*2,k"#;
    assert_snapshot!(parse_line(line).to_string(line));
  }

  #[test]
  fn gosub_goto() {
    let line = r#"10 gosub : goto: gosub 771 : goto 21742: goto"#;
    assert_snapshot!(parse_line(line).to_string(line));
  }

  #[test]
  fn r#if1() {
    let line = r#"10 If A>2 then:if not 1 goto print "a":else"#;
    assert_snapshot!(parse_line(line).to_string(line));
  }

  #[test]
  fn r#if2() {
    let line = r#"10 IF 1 THEN 10:ELSE S=S+1:NEXT:IF S> =10 GOTO"#;
    assert_snapshot!(parse_line(line).to_string(line));
  }

  #[test]
  fn r#if3() {
    let line = r#"10 IF K GOTO ELSE 2:13:7:"#;
    assert_snapshot!(parse_line(line).to_string(line));
  }

  #[test]
  fn input() {
    let line = r#"10 inPUT # s, A$, a$(3,i) : InPuT "ENTER:";A,B:INPUT A%"#;
    assert_snapshot!(parse_line(line).to_string(line));
  }

  #[test]
  fn locate() {
    let line = r#"10 locAte 1,A+2:locate A+1: locate , 2:"#;
    assert_snapshot!(parse_line(line).to_string(line));
  }

  #[test]
  fn lset_rset() {
    let line = r#"10 lset A$=MID$(B$,2):rSEt  a b$(2,k*3+m) =CHr$(x)"#;
    assert_snapshot!(parse_line(line).to_string(line));
  }

  #[test]
  fn next() {
    let line = r#"10 next:Next I  : NExT A,B b% , c , i"#;
    assert_snapshot!(parse_line(line).to_string(line));
  }

  #[test]
  fn on() {
    let line = r#"10 ON (k+2)*b goto:on x goSub ,:on x+1 goto 10,,30,,,"#;
    assert_snapshot!(parse_line(line).to_string(line));
  }

  #[test]
  fn open1() {
    let line = r#"10 opeN A$+".dat" appendA sa+1 : open b$for input as#2"#;
    assert_snapshot!(parse_line(line).to_string(line));
  }

  #[test]
  fn open2() {
    let line = r#"10 OPEN file$ randomas3:OPen f$output as 1"#;
    assert_snapshot!(parse_line(line).to_string(line));
  }

  #[test]
  fn open3() {
    let line = r#"10 OPEN P$FOR inputas1 len=k*2"#;
    assert_snapshot!(parse_line(line).to_string(line));
  }

  #[test]
  fn poke() {
    let line = r#"10 poKE a(i),30+I*2"#;
    assert_snapshot!(parse_line(line).to_string(line));
  }

  #[test]
  fn print() {
    let line =
      r#"10 print 10 ; , "k"+2; spc(3+k) tab(i) 3 +2fn f(4) ,:print,:print"#;
    assert_snapshot!(parse_line(line).to_string(line));
  }

  #[test]
  fn read() {
    let line = r#"10 reaD a$ : READ b$(i,j),c,d%"#;
    assert_snapshot!(parse_line(line).to_string(line));
  }

  #[test]
  fn swap() {
    let line = r#"10 SwaP a$(i),b c"#;
    assert_snapshot!(parse_line(line).to_string(line));
  }

  #[test]
  fn write() {
    let line = r#"10 write a$(i+j*10),b fn f(x):wriTE #3-a,asc(a)x+2 x*6 , o$"#;
    assert_snapshot!(parse_line(line).to_string(line));
  }

  mod expr {
    use super::*;

    #[test]
    fn precedence() {
      let line = r#"10 a(b+1,2)=-70.1++2+fn foo$(k*3-2*(k-3>=2))-5/ab 3 * 2^t  $
"#;
      assert_snapshot!(parse_line(line).to_string(line));
    }

    #[test]
    fn relation() {
      let line = r#"10 A b$=b*3>5 < > (1 2 . 3 e - 5 6 < = not chr$ ( "1" = inkey$ ))  "#;
      assert_snapshot!(parse_line(line).to_string(line));
    }

    #[test]
    fn string() {
      let line = r#"10 A b$=""+"ab cd E"#;
      assert_snapshot!(parse_line(line).to_string(line));
    }

    #[test]
    fn logical() {
      let line = r#"10 A b % =a and not 4 + 2 or -asc(left$(f$,k))"#;
      assert_snapshot!(parse_line(line).to_string(line));
    }
  }
}
