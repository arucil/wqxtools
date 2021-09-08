use crate::ast::{Eol, Node, NodeId, Program, ProgramLine, Range, TokenKind};
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

  let mut state = State::new(line);

  let label = match state.read_label() {
    Ok((_, label)) => Some(label),
    Err(err) => {
      state.report_label_error(err, Range::new(0, line_with_eol.len()));
      None
    }
  };

  state.into_line(line_with_eol, eol, label, stmts)
}

struct State<'a> {
  offset: usize,
  input: &'a str,
  token: (Range, TokenKind),
  arena: Arena<Node>,
  diagnostics: Vec<Diagnostic>,
}

enum ReadLabelError {
  Overflow,
  NoLabel,
}

impl<'a> State<'a> {
  fn new(input: &'a str) -> Self {
    Self {
      offset: 0,
      input,
      token: (Range::new(0, 0), TokenKind::Eof),
      arena: Arena::new(),
      diagnostics: vec![],
    }
  }

  fn into_line(
    self,
    line: &str,
    eol: Eol,
    label: Option<u16>,
    stmts: Vec<NodeId>,
  ) -> ProgramLine {
    ProgramLine {
      source_len: line.len(),
      label,
      arena: self.arena,
      stmts,
      eol,
      diagnostics: self.diagnostics,
    }
  }

  fn add_error(&mut self, range: Range, message: impl ToString) {
    self.diagnostics.push(Diagnostic::new_error(range, message));
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
      ReadLabelError::Overflow => {
        self.add_error(range, "行号必须在0~65535之间");
      }
    }
  }

  fn read_label(&mut self) -> Result<(Range, u16), ReadLabelError> {
    if self.input.is_empty() {
      return Err(ReadLabelError::NoLabel);
    }
    let start = self.offset;
    let mut i = 0;
    while let Some(n) = self.input.as_bytes().get(i).copied()
      && n.is_ascii_digit() {
      i += 1;
    }
    if i != 0 {
      self.advance(i);
      self.input[..i].parse::<u16>().map_or_else(
        |_| Err(ReadLabelError::Overflow),
        |label| Ok((Range::new(start, start + i), label)),
      )
    } else {
      Err(ReadLabelError::NoLabel)
    }
  }
}