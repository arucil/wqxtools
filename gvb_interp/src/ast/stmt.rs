#[cfg(test)]
use std::fmt::Write;
use std::fmt::{self, Debug, Formatter};

#[cfg(test)]
use super::Expr;
use super::{ExprId, Label, NonEmptyVec, Range, StmtId};
#[cfg(test)]
use id_arena::Arena;
use smallvec::SmallVec;
#[cfg(test)]
use widestring::utf16str;
#[cfg(test)]
use widestring::Utf16Str;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Stmt {
  pub kind: StmtKind,
  pub range: Range,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StmtKind {
  /// identical to REM
  Auto(Range),
  Beep,
  Box(NonEmptyVec<[ExprId; 4]>),
  Call(ExprId),
  Circle(NonEmptyVec<[ExprId; 3]>),
  Clear,
  Close {
    filenum: ExprId,
  },
  Cls,
  Cont,
  /// identical to REM
  Copy(Range),
  Data(NonEmptyVec<[Datum; 1]>),
  Def {
    /// ident
    name: Option<Range>,
    /// ident
    param: Option<Range>,
    body: ExprId,
  },
  /// identical to REM
  Del(Range),
  /// lvalue list
  Dim(NonEmptyVec<[ExprId; 1]>),
  Draw(NonEmptyVec<[ExprId; 2]>),
  /// identical to REM
  Edit(Range),
  Ellipse(NonEmptyVec<[ExprId; 4]>),
  End,
  Field {
    filenum: ExprId,
    fields: NonEmptyVec<[FieldSpec; 1]>,
  },
  /// identical to REM
  Files(Range),
  Flash,
  For {
    /// ident
    var: Option<Range>,
    start: ExprId,
    end: ExprId,
    step: Option<ExprId>,
  },
  Get {
    filenum: ExprId,
    record: ExprId,
  },
  GoSub(Option<(Range, Label)>),
  GoTo {
    has_goto_keyword: bool,
    label: Option<(Range, Label)>,
  },
  Graph,
  If {
    cond: ExprId,
    conseq: SmallVec<[StmtId; 1]>,
    alt: Option<SmallVec<[StmtId; 1]>>,
  },
  InKey,
  Input {
    source: InputSource,
    /// lvalue list
    vars: NonEmptyVec<[ExprId; 1]>,
  },
  Inverse,
  /// identical to REM
  Kill(Range),
  Let {
    /// lvalue
    var: ExprId,
    value: ExprId,
  },
  Line(NonEmptyVec<[ExprId; 4]>),
  /// identical to REM
  List(Range),
  /// identical to REM
  Load(Range),
  Locate {
    row: Option<ExprId>,
    column: Option<ExprId>,
  },
  LSet {
    /// lvalue
    var: ExprId,
    value: ExprId,
  },
  /// identical to REM
  New(Range),
  Next {
    /// ident list. may be empty
    vars: SmallVec<[Option<Range>; 1]>,
  },
  Normal,
  NoTrace,
  On {
    cond: ExprId,
    labels: NonEmptyVec<[(Range, Option<Label>); 2]>,
    is_sub: bool,
  },
  Open {
    filename: ExprId,
    mode: FileMode,
    filenum: ExprId,
    len: Option<(Range, ExprId)>,
  },
  Play(ExprId),
  Poke {
    addr: ExprId,
    value: ExprId,
  },
  Pop,
  Print(SmallVec<[PrintElement; 2]>),
  Put {
    filenum: ExprId,
    record: ExprId,
  },
  /// lvalue list
  Read(NonEmptyVec<[ExprId; 1]>),
  Rem(Range),
  /// identical to REM
  Rename(Range),
  Restore(Option<(Range, Label)>),
  Return,
  RSet {
    /// lvalue
    var: ExprId,
    value: ExprId,
  },
  Run(Range),
  /// identical to REM
  Save(Range),
  /// identical to REM
  Stop(Range),
  Swap {
    left: ExprId,
    right: ExprId,
  },
  System,
  Text,
  Trace,
  Wend,
  While(ExprId),
  Write {
    filenum: Option<ExprId>,
    data: NonEmptyVec<[WriteElement; 1]>,
  },
  Sleep(ExprId),
  Fputc {
    filenum: ExprId,
    value: ExprId,
  },
  Fread {
    filenum: ExprId,
    addr: ExprId,
    size: ExprId,
  },
  Fwrite {
    filenum: ExprId,
    addr: ExprId,
    size: ExprId,
  },
  Fseek {
    filenum: ExprId,
    offset: ExprId,
  },
  DebugPrint {
    value: ExprId,
  },
  NoOp,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Datum {
  /// Includes quotes.
  pub range: Range,
  pub is_quoted: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FieldSpec {
  pub range: Range,
  pub len: ExprId,
  /// lvalue
  pub var: ExprId,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InputSource {
  /// file num expr
  File(ExprId),
  /// prompt string literal
  Keyboard(Option<Range>),
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum FileMode {
  Input,
  Output,
  Append,
  Random,
  Binary,
  Error,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PrintElement {
  Expr(ExprId),
  Comma(Range),
  Semicolon(Range),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WriteElement {
  pub datum: ExprId,
  pub comma: bool,
}

impl Debug for FileMode {
  fn fmt(&self, f: &mut Formatter) -> fmt::Result {
    let kind = match self {
      Self::Input => "INPUT",
      Self::Output => "OUTPUT",
      Self::Append => "APPEND",
      Self::Random => "RANDOM",
      Self::Binary => "BINARY",
      Self::Error => "ERROR",
    };
    write!(f, "{}", kind)
  }
}

#[cfg(test)]
impl Stmt {
  pub fn print(
    &self,
    stmt_arena: &Arena<Stmt>,
    expr_arena: &Arena<Expr>,
    text: &Utf16Str,
    f: &mut impl Write,
  ) -> fmt::Result {
    print_stmt(self, 0, stmt_arena, expr_arena, text, f)
  }
}

#[cfg(test)]
fn print_stmt(
  stmt: &Stmt,
  indent: usize,
  stmt_arena: &Arena<Stmt>,
  expr_arena: &Arena<Expr>,
  text: &Utf16Str,
  f: &mut impl Write,
) -> fmt::Result {
  write!(f, "{:<1$?}", stmt.range, indent + 10)?;
  match &stmt.kind {
    StmtKind::Auto(range) => {
      writeln!(f, "AUTO [{:?}]", &text[range.range()])
    }
    StmtKind::Beep => writeln!(f, "BEEP"),
    StmtKind::Box(args) => {
      write!(f, "BOX ")?;
      let mut comma = false;
      for &arg in args.iter() {
        if comma {
          write!(f, ", ")?;
        }
        comma = true;
        expr_arena[arg].print(expr_arena, text, f)?;
      }
      writeln!(f)
    }
    StmtKind::Call(arg) => {
      write!(f, "CALL ")?;
      expr_arena[*arg].print(expr_arena, text, f)?;
      writeln!(f)
    }
    StmtKind::Circle(args) => {
      write!(f, "CIRCLE ")?;
      let mut comma = false;
      for &arg in args.iter() {
        if comma {
          write!(f, ", ")?;
        }
        comma = true;
        expr_arena[arg].print(expr_arena, text, f)?;
      }
      writeln!(f)
    }
    StmtKind::Clear => writeln!(f, "CLEAR"),
    StmtKind::Close { filenum } => {
      write!(f, "CLOSE # ")?;
      expr_arena[*filenum].print(expr_arena, text, f)?;
      writeln!(f)
    }
    StmtKind::Cls => writeln!(f, "CLS"),
    StmtKind::Cont => writeln!(f, "CONT"),
    StmtKind::Copy(range) => {
      writeln!(f, "COPY [{:?}]", &text[range.range()])
    }
    StmtKind::Data(data) => {
      write!(f, "DATA ")?;
      let mut comma = false;
      for datum in data.iter() {
        if comma {
          write!(f, ", ")?;
        }
        comma = true;
        if datum.is_quoted {
          write!(f, "\"[{}]\"", &text[datum.range.range()])?;
        } else {
          write!(f, "[{}]", &text[datum.range.range()])?;
        }
      }
      writeln!(f)
    }
    StmtKind::Def { name, param, body } => {
      write!(
        f,
        "DEF FN {}({}) = ",
        if let Some(name) = name {
          &text[name.range()]
        } else {
          utf16str!("???")
        },
        if let Some(param) = param {
          &text[param.range()]
        } else {
          utf16str!("???")
        },
      )?;
      expr_arena[*body].print(expr_arena, text, f)?;
      writeln!(f)
    }
    StmtKind::Del(range) => {
      writeln!(f, "DEL [{:?}]", &text[range.range()])
    }
    StmtKind::Dim(vars) => {
      write!(f, "DIM ")?;
      let mut comma = false;
      for &arg in vars.iter() {
        if comma {
          write!(f, ", ")?;
        }
        comma = true;
        expr_arena[arg].print(expr_arena, text, f)?;
      }
      writeln!(f)
    }
    StmtKind::Draw(args) => {
      write!(f, "DRAW ")?;
      let mut comma = false;
      for &arg in args.iter() {
        if comma {
          write!(f, ", ")?;
        }
        comma = true;
        expr_arena[arg].print(expr_arena, text, f)?;
      }
      writeln!(f)
    }
    StmtKind::Edit(range) => {
      writeln!(f, "EDIT [{:?}]", &text[range.range()])
    }
    StmtKind::Ellipse(args) => {
      write!(f, "ELLIPSE ")?;
      let mut comma = false;
      for &arg in args.iter() {
        if comma {
          write!(f, ", ")?;
        }
        comma = true;
        expr_arena[arg].print(expr_arena, text, f)?;
      }
      writeln!(f)
    }
    StmtKind::End => writeln!(f, "END"),
    StmtKind::Field { filenum, fields } => {
      write!(f, "FIELD # ")?;
      expr_arena[*filenum].print(expr_arena, text, f)?;
      for field in fields.iter() {
        write!(f, ", <{:?}> ", field.range)?;
        expr_arena[field.len].print(expr_arena, text, f)?;
        write!(f, " AS ")?;
        expr_arena[field.var].print(expr_arena, text, f)?;
      }
      writeln!(f)
    }
    StmtKind::Files(range) => {
      writeln!(f, "FILES [{:?}]", &text[range.range()])
    }
    StmtKind::Flash => writeln!(f, "FLASH"),
    StmtKind::For {
      var,
      start,
      end,
      step,
    } => {
      if let Some(var) = var {
        write!(f, "FOR {} = ", &text[var.range()])?;
      } else {
        write!(f, "FOR ??? = ")?;
      }
      expr_arena[*start].print(expr_arena, text, f)?;
      write!(f, " TO ")?;
      expr_arena[*end].print(expr_arena, text, f)?;
      if let Some(step) = step {
        write!(f, " STEP ")?;
        expr_arena[*step].print(expr_arena, text, f)?;
      }
      writeln!(f)
    }
    StmtKind::Get { filenum, record } => {
      write!(f, "GET # ")?;
      expr_arena[*filenum].print(expr_arena, text, f)?;
      write!(f, ", ")?;
      expr_arena[*record].print(expr_arena, text, f)?;
      writeln!(f)
    }
    StmtKind::GoSub(label) => {
      if let Some((range, label)) = label {
        assert_eq!(text[range.range()].to_string().parse::<Label>(), Ok(*label));
        writeln!(f, "GOSUB {}", label.0)
      } else {
        writeln!(f, "GOSUB")
      }
    }
    StmtKind::GoTo {
      has_goto_keyword,
      label,
    } => {
      let goto = if *has_goto_keyword { "GOTO" } else { "[GOTO]" };
      if let Some((range, label)) = label {
        assert_eq!(text[range.range()].to_string().parse::<Label>(), Ok(*label));
        writeln!(f, "{} {}", goto, label.0)
      } else {
        writeln!(f, "{}", goto)
      }
    }
    StmtKind::Graph => writeln!(f, "GRAPH"),
    StmtKind::If { cond, conseq, alt } => {
      write!(f, "IF ")?;
      expr_arena[*cond].print(expr_arena, text, f)?;
      writeln!(f, " THEN")?;
      for &stmt in conseq.iter() {
        print_stmt(
          &stmt_arena[stmt],
          indent + 4,
          stmt_arena,
          expr_arena,
          text,
          f,
        )?;
      }
      if let Some(alt) = alt {
        writeln!(f, "{:1$}{2:<1$}", "", indent + 10, "ELSE")?;
        for &stmt in alt.iter() {
          print_stmt(
            &stmt_arena[stmt],
            indent + 4,
            stmt_arena,
            expr_arena,
            text,
            f,
          )?;
        }
      }
      Ok(())
    }
    StmtKind::InKey => writeln!(f, "INKEY$"),
    StmtKind::Input { source, vars } => {
      write!(f, "INPUT ")?;
      match source {
        InputSource::Keyboard(Some(range)) => {
          write!(f, "<STR: {}>; ", &text[range.range()])?;
        }
        InputSource::Keyboard(None) => {}
        InputSource::File(filenum) => {
          write!(f, "# ")?;
          expr_arena[*filenum].print(expr_arena, text, f)?;
          write!(f, ", ")?;
        }
      }
      let mut comma = false;
      for &arg in vars.iter() {
        if comma {
          write!(f, ", ")?;
        }
        comma = true;
        expr_arena[arg].print(expr_arena, text, f)?;
      }
      writeln!(f)
    }
    StmtKind::Inverse => writeln!(f, "INVERSE"),
    StmtKind::Kill(range) => {
      writeln!(f, "KILL [{:?}]", &text[range.range()])
    }
    StmtKind::Let { var, value } => {
      write!(f, "LET ")?;
      expr_arena[*var].print(expr_arena, text, f)?;
      write!(f, " = ")?;
      expr_arena[*value].print(expr_arena, text, f)?;
      writeln!(f)
    }
    StmtKind::Line(args) => {
      write!(f, "LINE ")?;
      let mut comma = false;
      for &arg in args.iter() {
        if comma {
          write!(f, ", ")?;
        }
        comma = true;
        expr_arena[arg].print(expr_arena, text, f)?;
      }
      writeln!(f)
    }
    StmtKind::List(range) => {
      writeln!(f, "LIST [{:?}]", &text[range.range()])
    }
    StmtKind::Load(range) => {
      writeln!(f, "LOAD [{:?}]", &text[range.range()])
    }
    StmtKind::Locate { row, column } => {
      write!(f, "LOCATE ")?;
      if let Some(row) = row {
        expr_arena[*row].print(expr_arena, text, f)?;
      }
      if let Some(column) = column {
        write!(f, ", ")?;
        expr_arena[*column].print(expr_arena, text, f)?;
      }
      writeln!(f)
    }
    StmtKind::LSet { var, value } => {
      write!(f, "LSET ")?;
      expr_arena[*var].print(expr_arena, text, f)?;
      write!(f, " = ")?;
      expr_arena[*value].print(expr_arena, text, f)?;
      writeln!(f)
    }
    StmtKind::New(range) => {
      writeln!(f, "NEW [{:?}]", &text[range.range()])
    }
    StmtKind::Next { vars } => {
      write!(f, "NEXT ")?;
      let mut comma = false;
      for var in vars.iter() {
        if comma {
          write!(f, ", ")?;
        }
        comma = true;
        if let Some(var) = var {
          write!(f, "{}", &text[var.range()])?;
        } else {
          write!(f, "???")?;
        }
      }
      writeln!(f)
    }
    StmtKind::Normal => writeln!(f, "NORMAL"),
    StmtKind::NoTrace => writeln!(f, "NOTRACE"),
    StmtKind::On {
      cond,
      labels,
      is_sub,
    } => {
      write!(f, "ON ")?;
      expr_arena[*cond].print(expr_arena, text, f)?;
      if *is_sub {
        write!(f, " GOSUB ")?;
      } else {
        write!(f, " GOTO ")?;
      }
      let mut comma = false;
      for (range, label) in labels.iter() {
        if comma {
          write!(f, ", ")?;
        }
        comma = true;
        if let Some(label) = label {
          assert_eq!(text[range.range()].to_string().parse::<Label>(), Ok(*label));
          write!(f, "{}", label.0)?;
        } else {
          write!(f, "<{:?}>", range)?;
        }
      }
      writeln!(f)
    }
    StmtKind::Open {
      filename,
      mode,
      filenum,
      len,
    } => {
      write!(f, "OPEN ")?;
      expr_arena[*filename].print(expr_arena, text, f)?;
      write!(f, " FOR {:?} AS # ", mode)?;
      expr_arena[*filenum].print(expr_arena, text, f)?;
      if let Some((_, len)) = len {
        write!(f, " LEN = ")?;
        expr_arena[*len].print(expr_arena, text, f)?;
      }
      writeln!(f)
    }
    StmtKind::Play(e) => {
      write!(f, "PLAY ")?;
      expr_arena[*e].print(expr_arena, text, f)?;
      writeln!(f)
    }
    StmtKind::Poke { addr, value } => {
      write!(f, "POKE ")?;
      expr_arena[*addr].print(expr_arena, text, f)?;
      write!(f, ", ")?;
      expr_arena[*value].print(expr_arena, text, f)?;
      writeln!(f)
    }
    StmtKind::Pop => writeln!(f, "POP"),
    StmtKind::Print(elems) => {
      write!(f, "PRINT ")?;
      for elem in elems.iter() {
        match elem {
          PrintElement::Comma(_) => write!(f, ", ")?,
          PrintElement::Semicolon(_) => write!(f, "; ")?,
          PrintElement::Expr(e) => {
            expr_arena[*e].print(expr_arena, text, f)?;
            write!(f, " ")?;
          }
        }
      }
      writeln!(f)
    }
    StmtKind::Put { filenum, record } => {
      write!(f, "PUT # ")?;
      expr_arena[*filenum].print(expr_arena, text, f)?;
      write!(f, ", ")?;
      expr_arena[*record].print(expr_arena, text, f)?;
      writeln!(f)
    }
    StmtKind::Read(vars) => {
      write!(f, "READ ")?;
      let mut comma = false;
      for &arg in vars.iter() {
        if comma {
          write!(f, ", ")?;
        }
        comma = true;
        expr_arena[arg].print(expr_arena, text, f)?;
      }
      writeln!(f)
    }
    StmtKind::Rem(range) => {
      writeln!(f, "REM [{:?}]", &text[range.range()])
    }
    StmtKind::Rename(range) => {
      writeln!(f, "RENAME [{:?}]", &text[range.range()])
    }
    StmtKind::Restore(label) => {
      if let Some((range, label)) = label {
        assert_eq!(text[range.range()].to_string().parse::<Label>(), Ok(*label));
        writeln!(f, "RESTORE {}", label.0)
      } else {
        writeln!(f, "RESTORE")
      }
    }
    StmtKind::Return => writeln!(f, "RETURN"),
    StmtKind::RSet { var, value } => {
      write!(f, "RSET ")?;
      expr_arena[*var].print(expr_arena, text, f)?;
      write!(f, " = ")?;
      expr_arena[*value].print(expr_arena, text, f)?;
      writeln!(f)
    }
    StmtKind::Run(range) => {
      writeln!(f, "RUN [{:?}]", &text[range.range()])
    }
    StmtKind::Save(range) => {
      writeln!(f, "SAVE [{:?}]", &text[range.range()])
    }
    StmtKind::Stop(range) => {
      writeln!(f, "STOP [{:?}]", &text[range.range()])
    }
    StmtKind::Swap { left, right } => {
      write!(f, "SWAP ")?;
      expr_arena[*left].print(expr_arena, text, f)?;
      write!(f, ", ")?;
      expr_arena[*right].print(expr_arena, text, f)?;
      writeln!(f)
    }
    StmtKind::System => writeln!(f, "SYSTEM"),
    StmtKind::Text => writeln!(f, "TEXT"),
    StmtKind::Trace => writeln!(f, "TRACE"),
    StmtKind::Wend => writeln!(f, "WEND"),
    StmtKind::While(cond) => {
      write!(f, "WHILE ")?;
      expr_arena[*cond].print(expr_arena, text, f)?;
      writeln!(f)
    }
    StmtKind::Write { filenum, data } => {
      write!(f, "WRITE ")?;
      if let Some(filenum) = filenum {
        write!(f, "# ")?;
        expr_arena[*filenum].print(expr_arena, text, f)?;
        write!(f, ", ")?;
      }
      for datum in data.iter() {
        expr_arena[datum.datum].print(expr_arena, text, f)?;
        if datum.comma {
          write!(f, ", ")?;
        } else {
          write!(f, " ")?;
        }
      }
      writeln!(f)
    }
    StmtKind::Sleep(arg) => {
      write!(f, "SLEEP ")?;
      expr_arena[*arg].print(expr_arena, text, f)?;
      writeln!(f)
    }
    StmtKind::Fputc { filenum, value } => {
      write!(f, "FPUTC # ")?;
      expr_arena[*filenum].print(expr_arena, text, f)?;
      write!(f, ", ")?;
      expr_arena[*value].print(expr_arena, text, f)?;
      writeln!(f)
    }
    StmtKind::Fread {
      filenum,
      addr,
      size,
    } => {
      write!(f, "Fread # ")?;
      expr_arena[*filenum].print(expr_arena, text, f)?;
      write!(f, ", ")?;
      expr_arena[*addr].print(expr_arena, text, f)?;
      write!(f, ", ")?;
      expr_arena[*size].print(expr_arena, text, f)?;
      writeln!(f)
    }
    StmtKind::Fwrite {
      filenum,
      addr,
      size,
    } => {
      write!(f, "Fwrite # ")?;
      expr_arena[*filenum].print(expr_arena, text, f)?;
      write!(f, ", ")?;
      expr_arena[*addr].print(expr_arena, text, f)?;
      write!(f, ", ")?;
      expr_arena[*size].print(expr_arena, text, f)?;
      writeln!(f)
    }
    StmtKind::Fseek { filenum, offset } => {
      write!(f, "Fseek # ")?;
      expr_arena[*filenum].print(expr_arena, text, f)?;
      write!(f, ", ")?;
      expr_arena[*offset].print(expr_arena, text, f)?;
      writeln!(f)
    }
    StmtKind::DebugPrint { value } => {
      write!(f, "DebugPrint # ")?;
      expr_arena[*value].print(expr_arena, text, f)?;
      writeln!(f)
    }
    StmtKind::NoOp => writeln!(f, ":"),
  }
}
