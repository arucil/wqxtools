use super::{ExprId, Label, NonEmptyVec, Range, StmtId};
use smallvec::SmallVec;

#[derive(Debug, Clone)]
pub struct Stmt {
  pub kind: StmtKind,
  pub range: Range,
  pub is_recovered: bool,
}

#[derive(Debug, Clone)]
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
    name: Range,
    /// ident
    param: Range,
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
    var: Range,
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
    vars: SmallVec<[Range; 1]>,
  },
  Normal,
  NoTrace,
  On {
    cond: ExprId,
    labels: NonEmptyVec<[(Range, Label); 2]>,
    is_sub: bool,
  },
  Open {
    filename: ExprId,
    mode: FileMode,
    filenum: ExprId,
    len: Option<ExprId>,
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
  Run,
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
  NoOp,
}

#[derive(Debug, Clone)]
pub struct Datum {
  /// Does not include quotes.
  pub range: Range,
  pub is_quoted: bool,
}

#[derive(Debug, Clone)]
pub struct FieldSpec {
  pub range: Range,
  pub len: ExprId,
  /// lvalue
  pub var: ExprId,
}

#[derive(Debug, Clone)]
pub enum InputSource {
  /// file num expr
  File(ExprId),
  /// prompt string literal
  Keyboard(ExprId),
  Error,
}

#[derive(Debug, Clone)]
pub enum FileMode {
  Input,
  Output,
  Append,
  Random,
  Error,
}

#[derive(Debug, Clone)]
pub enum PrintElement {
  Expr(ExprId),
  Comma,
  Semicolon,
}

#[derive(Debug, Clone)]
pub struct WriteElement {
  pub datum: ExprId,
  pub comma: bool,
}