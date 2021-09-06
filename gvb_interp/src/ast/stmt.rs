use super::{Range, NodeId, NonEmptyVec};
use smallvec::SmallVec;

#[derive(Debug, Clone)]
pub enum Stmt {
  /// identical to REM
  Auto(Range),
  Beep,
  Box {
    x1: NodeId,
    y1: NodeId,
    x2: NodeId,
    y2: NodeId,
    fill_mode: Option<NodeId>,
    draw_mode: Option<NodeId>,
  },
  Call {
    addr: NodeId,
  },
  Circle {
    x: NodeId,
    y: NodeId,
    r: NodeId,
    fill_mode: Option<NodeId>,
    draw_mode: Option<NodeId>,
  },
  Clear,
  Close {
    /// expr
    filenum: NodeId,
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
    /// expr
    body: NodeId,
  },
  /// identical to REM
  Del(Range),
  /// lvalue list
  Dim(NonEmptyVec<[NodeId; 1]>),
  Draw {
    x: NodeId,
    y: NodeId,
    draw_mode: Option<NodeId>,
  },
  /// identical to REM
  Edit(Range),
  Ellipse {
    x: NodeId,
    y: NodeId,
    rx: NodeId,
    ry: NodeId,
    fill_mode: Option<NodeId>,
    draw_mode: Option<NodeId>,
  },
  End,
  Field {
    /// expr
    filenum: NodeId,
    fields: NonEmptyVec<[FieldSpec; 1]>
  },
  /// identical to REM
  Files(Range),
  Flash,
  For {
    /// ident
    var: Range,
    start: NodeId,
    end: NodeId,
    step: Option<NodeId>,
  },
  Get {
    /// expr
    filenum: NodeId,
    /// expr
    record: NodeId,
  },
  GoSub(Option<Range>),
  GoTo {
    has_keyword: bool,
    label: Option<Range>
  },
  Graph,
  If {
    /// expr
    cond: NodeId,
    /// stmt list
    conseq: NonEmptyVec<[NodeId; 1]>,
    /// stmt list
    alt: NonEmptyVec<[NodeId; 1]>,
  },
  InKey,
  Input {
    source: InputSource,
    /// lvalue list
    fields: NonEmptyVec<[NodeId; 1]>,
  },
  Inverse,
  /// identical to REM
  Kill(Range),
  Let {
    /// lvalue
    field: NodeId,
    value: NodeId,
  },
  Line {
    x1: NodeId,
    y1: NodeId,
    x2: NodeId,
    y2: NodeId,
    draw_mode: Option<NodeId>,
  },
  /// identical to REM
  List(Range),
  /// identical to REM
  Load(Range),
  Locate {
    row: Option<NodeId>,
    column: Option<NodeId>,
  },
  LSet {
    /// lvalue
    field: NodeId,
    value: NodeId,
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
    cond: NodeId,
    labels: NonEmptyVec<[Range; 2]>,
    is_sub: bool,
  },
  Open {
    /// expr
    filename: NodeId,
    mode: FileMode,
    /// integer
    filenum: Range,
    /// expr
    len: Option<NodeId>
  },
  /// expr
  Play(NodeId),
  Poke {
    addr: NodeId,
    value: NodeId,
  },
  Pop,
  Print(SmallVec<[PrintElement; 2]>),
  Put {
    /// expr
    filenum: NodeId,
    /// expr
    record: NodeId,
  },
  Read(NonEmptyVec<[NodeId; 1]>),
  Rem(Range),
  /// identical to REM
  Rename(Range),
  Restore(Option<Range>),
  Return,
  RSet {
    /// lvalue
    field: NodeId,
    value: NodeId,
  },
  Run,
  /// identical to REM
  Save(Range),
  /// identical to REM
  Stop(Range),
  Swap {
    left: NodeId,
    right: NodeId,
  },
  System,
  Text,
  Trace,
  Wend,
  /// expr
  While(NodeId),
  Write {
    /// expr
    filenum: Option<NodeId>,
    fields: NonEmptyVec<[WriteElement; 1]>,
  }
}

#[derive(Debug, Clone)]
pub struct FieldSpec {
  pub len: NodeId,
  /// lvalue
  pub name: NodeId,
}

#[derive(Debug, Clone)]
pub enum InputSource {
  /// file num expr
  File(NodeId),
  /// prompt string literal
  Keyboard(NodeId),
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
  Expr(NodeId),
  Comma,
  Semicolon,
  Spc(NodeId),
  Tab(NodeId),
}

#[derive(Debug, Clone)]
pub struct WriteElement {
  value: NodeId,
  comma: bool,
}