use super::{Range, NonEmptyVec, ExprId, StmtId, Label};
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
  Box {
    x1: ExprId,
    y1: ExprId,
    x2: ExprId,
    y2: ExprId,
    fill_mode: Option<ExprId>,
    draw_mode: Option<ExprId>,
  },
  Call {
    addr: ExprId,
  },
  Circle {
    x: ExprId,
    y: ExprId,
    r: ExprId,
    fill_mode: Option<ExprId>,
    draw_mode: Option<ExprId>,
  },
  Clear,
  Close {
    filenum: ExprId,
  },
  Cls,
  Cont,
  /// identical to REM
  Copy(Range),
  Data(Range),
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
  Draw {
    x: ExprId,
    y: ExprId,
    draw_mode: Option<ExprId>,
  },
  /// identical to REM
  Edit(Range),
  Ellipse {
    x: ExprId,
    y: ExprId,
    rx: ExprId,
    ry: ExprId,
    fill_mode: Option<ExprId>,
    draw_mode: Option<ExprId>,
  },
  End,
  Field {
    filenum: ExprId,
    fields: NonEmptyVec<[FieldSpec; 1]>
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
    label: Option<(Range, Label)>
  },
  Graph,
  If {
    cond: ExprId,
    conseq: SmallVec<[StmtId; 1]>,
    alt: SmallVec<[StmtId; 1]>,
  },
  InKey,
  Input {
    source: InputSource,
    /// lvalue list
    fields: NonEmptyVec<[ExprId; 1]>,
  },
  Inverse,
  /// identical to REM
  Kill(Range),
  Let {
    /// lvalue
    field: ExprId,
    value: ExprId,
  },
  Line {
    x1: ExprId,
    y1: ExprId,
    x2: ExprId,
    y2: ExprId,
    draw_mode: Option<ExprId>,
  },
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
    field: ExprId,
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
    /// integer
    filenum: Range,
    len: Option<ExprId>
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
    field: ExprId,
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
    fields: NonEmptyVec<[WriteElement; 1]>,
  },
  NoOp,
}

#[derive(Debug, Clone)]
pub struct FieldSpec {
  pub len: ExprId,
  /// lvalue
  pub name: ExprId,
}

#[derive(Debug, Clone)]
pub enum InputSource {
  /// file num expr
  File(ExprId),
  /// prompt string literal
  Keyboard(ExprId),
}

#[derive(Debug, Clone)]
pub enum FileMode {
  Input,
  Output,
  Append,
  Random
}

#[derive(Debug, Clone)]
pub enum PrintElement {
  Expr(ExprId),
  Comma,
  Semicolon,
  Spc(ExprId),
  Tab(ExprId),
}

#[derive(Debug, Clone)]
pub struct WriteElement {
  value: ExprId,
  comma: bool,
}