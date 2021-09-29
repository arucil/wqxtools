use std::convert::TryFrom;
use std::num::NonZeroUsize;

use super::{
  Addr, Alignment, ByteString, DatumIndex, Instr, InstrKind, PrintMode,
  ScreenMode, Symbol, DUMMY_ADDR, FISRT_DATUM_INDEX,
};
use crate::ast::{BinaryOpKind, FileMode, Range, StmtKind, UnaryOpKind};
use crate::diagnostic::Diagnostic;
use crate::util::mbf5::Mbf5;
use crate::{compiler::CodeEmitter, machine::EmojiStyle};
use string_interner::StringInterner;

use super::Datum;

pub struct CodeGen {
  emoji_style: EmojiStyle,
  pub(super) interner: StringInterner,
  pub(super) data: Vec<Datum>,
  pub(super) code: Vec<Instr>,
}

impl CodeGen {
  pub fn new(emoji_style: EmojiStyle) -> Self {
    Self {
      emoji_style,
      interner: StringInterner::new(),
      data: vec![],
      code: vec![],
    }
  }

  fn push_instr(&mut self, range: Range, kind: InstrKind) {
    self.code.push(Instr { range, kind })
  }
}

impl CodeEmitter for CodeGen {
  type Symbol = Symbol;
  type Addr = Addr;
  type DatumIndex = DatumIndex;

  fn emit_no_op(&mut self, _range: Range) {
    // do nothing
  }

  fn emit_op(&mut self, range: Range, kind: &StmtKind, arity: usize) {
    match kind {
      StmtKind::Beep => self.push_instr(range, InstrKind::Beep),
      StmtKind::Box(_) => self.push_instr(
        range,
        InstrKind::DrawBox {
          has_fill: arity >= 5,
          has_mode: arity >= 6,
        },
      ),
      StmtKind::Call(_) => self.push_instr(range, InstrKind::Call),
      StmtKind::Circle(_) => self.push_instr(
        range,
        InstrKind::DrawCircle {
          has_fill: arity >= 4,
          has_mode: arity >= 5,
        },
      ),
      StmtKind::Clear => self.push_instr(range, InstrKind::Clear),
      StmtKind::Close { .. } => self.push_instr(range, InstrKind::CloseFile),
      StmtKind::Cls => self.push_instr(range, InstrKind::Cls),
      StmtKind::Cont => self.push_instr(range, InstrKind::NoOp),
      StmtKind::Draw(_) => self.push_instr(
        range,
        InstrKind::DrawPoint {
          has_mode: arity >= 3,
        },
      ),
      StmtKind::Ellipse(_) => self.push_instr(
        range,
        InstrKind::DrawEllipse {
          has_fill: arity >= 5,
          has_mode: arity >= 6,
        },
      ),
      StmtKind::End => self.push_instr(range, InstrKind::End),
      StmtKind::Flash => {
        self.push_instr(range, InstrKind::SetPrintMode(PrintMode::Flash))
      }
      StmtKind::Get { .. } => self.push_instr(range, InstrKind::ReadRecord),
      StmtKind::Graph => {
        self.push_instr(range, InstrKind::SetScreenMode(ScreenMode::Graph))
      }
      StmtKind::InKey => {
        self.push_instr(range.clone(), InstrKind::PushInKey);
        self.push_instr(range, InstrKind::PopValue);
      }
      StmtKind::Inverse => {
        self.push_instr(range, InstrKind::SetPrintMode(PrintMode::Inverse))
      }
      StmtKind::Let { .. } => self.push_instr(range, InstrKind::Assign),
      StmtKind::Line(_) => self.push_instr(
        range,
        InstrKind::DrawLine {
          has_mode: arity >= 5,
        },
      ),
      StmtKind::LSet { .. } => {
        self.push_instr(range, InstrKind::AlignedAssign(Alignment::Left))
      }
      StmtKind::Normal => {
        self.push_instr(range, InstrKind::SetPrintMode(PrintMode::Normal))
      }
      StmtKind::NoTrace => self.push_instr(range, InstrKind::SetTrace(false)),
      StmtKind::Play(_) => self.push_instr(range, InstrKind::PlayNotes),
      StmtKind::Poke { .. } => self.push_instr(range, InstrKind::Poke),
      StmtKind::Pop => self.push_instr(range, InstrKind::Pop),
      StmtKind::Put { .. } => self.push_instr(range, InstrKind::WriteRecord),
      StmtKind::Return => self.push_instr(range, InstrKind::Return),
      StmtKind::RSet { .. } => {
        self.push_instr(range, InstrKind::AlignedAssign(Alignment::Right))
      }
      StmtKind::Run => self.push_instr(range, InstrKind::Restart),
      StmtKind::Swap { .. } => self.push_instr(range, InstrKind::Swap),
      StmtKind::Text => {
        self.push_instr(range, InstrKind::SetScreenMode(ScreenMode::Text))
      }
      StmtKind::Trace => self.push_instr(range, InstrKind::SetTrace(true)),
      StmtKind::Wend => self.push_instr(range, InstrKind::Wend),
      _ => unreachable!(),
    }
  }

  fn emit_datum(
    &mut self,
    range: Range,
    value: String,
    is_quoted: bool,
  ) -> Self::DatumIndex {
    let index = DatumIndex(self.data.len());
    self.data.push(Datum {
      range,
      value: ByteString::from_str(value, self.emoji_style).unwrap(),
      is_quoted,
    });
    index
  }

  fn begin_def_fn(
    &mut self,
    range: Range,
    name: Self::Symbol,
    param: Self::Symbol,
  ) -> Self::Addr {
    let addr = Addr(self.code.len());
    self.code.push(Instr {
      range,
      kind: InstrKind::DefFn {
        name,
        param,
        end: DUMMY_ADDR,
      },
    });
    addr
  }

  fn end_def_fn(&mut self, def_addr: Self::Addr) {
    let range = self.code[def_addr.0].range.clone();
    self.push_instr(range, InstrKind::ReturnFn);
    let cur_addr = Addr(self.code.len());
    match &mut self.code[def_addr.0].kind {
      InstrKind::DefFn { end, .. } => {
        *end = cur_addr;
      }
      _ => unreachable!(),
    }
  }

  fn emit_dim(
    &mut self,
    range: Range,
    name: Self::Symbol,
    dimensions: NonZeroUsize,
  ) {
    self.push_instr(range, InstrKind::DimArray { name, dimensions });
  }

  fn emit_lvalue(
    &mut self,
    range: Range,
    name: Self::Symbol,
    dimensions: usize,
  ) {
    self.push_instr(range, InstrKind::PushLValue { name, dimensions });
  }

  fn emit_fn_lvalue(
    &mut self,
    range: Range,
    name: Self::Symbol,
    param: Self::Symbol,
  ) {
    self.push_instr(range, InstrKind::PushFnLValue { name, param });
  }

  fn emit_field(&mut self, range: Range, fields: NonZeroUsize) {
    self.push_instr(range, InstrKind::SetRecordFields { fields });
  }

  fn emit_for(&mut self, range: Range, var: Self::Symbol, has_step: bool) {
    self.push_instr(
      range,
      InstrKind::ForLoop {
        name: var,
        has_step,
      },
    );
  }

  fn emit_next(&mut self, range: Range, var: Option<Self::Symbol>) {
    self.push_instr(range, InstrKind::NextFor { name: var });
  }

  fn make_symbol(&mut self, name: String) -> Self::Symbol {
    self.interner.get_or_intern(name)
  }

  fn emit_gosub(&mut self, range: Range) -> Self::Addr {
    let addr = Addr(self.code.len());
    self.push_instr(range, InstrKind::GoSub(DUMMY_ADDR));
    addr
  }

  fn emit_goto(&mut self, range: Range) -> Self::Addr {
    let addr = Addr(self.code.len());
    self.push_instr(range, InstrKind::GoTo(DUMMY_ADDR));
    addr
  }

  fn patch_jump_addr(&mut self, addr: Self::Addr, label_addr: Self::Addr) {
    match &mut self.code[addr.0].kind {
      InstrKind::GoSub(addr)
      | InstrKind::GoTo(addr)
      | InstrKind::JumpIfZero(addr) => {
        *addr = label_addr;
      }
      _ => unreachable!(),
    }
  }

  fn emit_jz(&mut self, range: Range) -> Self::Addr {
    let addr = Addr(self.code.len());
    self.push_instr(range, InstrKind::JumpIfZero(DUMMY_ADDR));
    addr
  }

  fn current_addr(&self) -> Self::Addr {
    Addr(self.code.len())
  }

  fn emit_on(&mut self, range: Range, labels: NonZeroUsize) {
    self.push_instr(range, InstrKind::Switch(labels));
  }

  fn emit_set_row(&mut self, range: Range) {
    self.push_instr(range, InstrKind::SetRow);
  }

  fn emit_set_column(&mut self, range: Range) {
    self.push_instr(range, InstrKind::SetColumn);
  }

  fn emit_restore(&mut self, range: Range) -> Self::Addr {
    let addr = Addr(self.code.len());
    self.push_instr(range, InstrKind::RestoreDataPtr(FISRT_DATUM_INDEX));
    addr
  }

  fn patch_datum_index(&mut self, addr: Self::Addr, index: Self::DatumIndex) {
    match &mut self.code[addr.0].kind {
      InstrKind::RestoreDataPtr(i) => {
        *i = index;
      }
      _ => unreachable!(),
    }
  }

  fn emit_keyboard_input(
    &mut self,
    range: Range,
    prompt: Option<String>,
    fields: NonZeroUsize,
  ) {
    self.push_instr(range, InstrKind::KeyboardInput { prompt, fields });
  }

  fn emit_file_input(&mut self, range: Range, fields: NonZeroUsize) {
    self.push_instr(range, InstrKind::FileInput { fields });
  }

  fn emit_open(&mut self, range: Range, mode: FileMode, has_len: bool) {
    self.push_instr(range, InstrKind::OpenFile { mode, has_len });
  }

  fn emit_read(&mut self, range: Range) {
    self.push_instr(range, InstrKind::ReadData);
  }

  fn emit_print_newline(&mut self, range: Range) {
    self.push_instr(range, InstrKind::PrintNewLine);
  }

  fn emit_print_spc(&mut self, range: Range) {
    self.push_instr(range, InstrKind::PrintSpc);
  }

  fn emit_print_tab(&mut self, range: Range) {
    self.push_instr(range, InstrKind::PrintTab);
  }

  fn emit_print_value(&mut self, range: Range) {
    self.push_instr(range, InstrKind::PrintValue);
  }

  fn emit_pop(&mut self, range: Range) {
    self.push_instr(range, InstrKind::PopValue);
  }

  fn emit_write(&mut self, range: Range, to_file: bool) {
    self.push_instr(range, InstrKind::Write { to_file });
  }

  fn emit_write_end(&mut self, range: Range, to_file: bool) {
    self.push_instr(range, InstrKind::WriteEnd { to_file });
  }

  fn emit_while(&mut self, range: Range, cond_start: Addr) {
    self.push_instr(
      range,
      InstrKind::WhileLoop {
        start: cond_start,
        end: DUMMY_ADDR,
      },
    );
  }

  fn emit_number(&mut self, range: Range, num: Mbf5) {
    self.push_instr(range, InstrKind::PushNum(num));
  }

  fn emit_var(&mut self, range: Range, sym: Self::Symbol) {
    self.push_instr(range, InstrKind::PushVar(sym));
  }

  fn emit_string(&mut self, range: Range, str: String) {
    self.push_instr(
      range,
      InstrKind::PushStr(ByteString::from_str(str, self.emoji_style).unwrap()),
    );
  }

  fn emit_inkey(&mut self, range: Range) {
    self.push_instr(range, InstrKind::PushInKey);
  }

  fn emit_index(
    &mut self,
    range: Range,
    name: Self::Symbol,
    dimensions: NonZeroUsize,
  ) {
    self.push_instr(range, InstrKind::PushIndex { name, dimensions });
  }
  fn emit_unary_expr(&mut self, range: Range, kind: UnaryOpKind) {
    let kind = match kind {
      UnaryOpKind::Not => InstrKind::Not,
      UnaryOpKind::Neg => InstrKind::Neg,
      UnaryOpKind::Pos => return,
    };
    self.push_instr(range, kind);
  }

  fn emit_binary_expr(&mut self, range: Range, kind: BinaryOpKind) {
    let kind = match kind {
      BinaryOpKind::Eq => InstrKind::Eq,
      BinaryOpKind::Ne => InstrKind::Ne,
      BinaryOpKind::Gt => InstrKind::Gt,
      BinaryOpKind::Lt => InstrKind::Lt,
      BinaryOpKind::Ge => InstrKind::Ge,
      BinaryOpKind::Le => InstrKind::Le,
      BinaryOpKind::Add => InstrKind::Add,
      BinaryOpKind::Sub => InstrKind::Sub,
      BinaryOpKind::Mul => InstrKind::Mul,
      BinaryOpKind::Div => InstrKind::Div,
      BinaryOpKind::Pow => InstrKind::Pow,
      BinaryOpKind::And => InstrKind::And,
      BinaryOpKind::Or => InstrKind::Or,
    };
    self.push_instr(range, kind);
  }

  fn emit_user_func_call(&mut self, range: Range, name: Self::Symbol) {
    self.push_instr(range, InstrKind::CallFn(name));
  }

  fn clean_up(&mut self) -> Vec<Diagnostic> {
    let mut diags = vec![];
    self.patch_while_instr(&mut diags);
    self.convert_for_loop_to_sleep(&mut diags);
    self.code.push(Instr {
      range: Range::new(0, 0),
      kind: InstrKind::End,
    });
    diags
  }
}

impl CodeGen {
  fn patch_while_instr(&mut self, diagnostics: &mut Vec<Diagnostic>) {
    let mut wend_stack: Vec<Addr> = vec![];

    for (i, instr) in self.code.iter_mut().enumerate().rev() {
      match &mut instr.kind {
        InstrKind::Wend => wend_stack.push(Addr(i)),
        InstrKind::WhileLoop { end, .. } => {
          if let Some(i) = wend_stack.pop() {
            *end = i;
          } else {
            diagnostics.push(Diagnostic::new_error(
              instr.range.clone(),
              "WHILE 语句没有对应的 WEND 语句",
            ));
          }
        }
        _ => {
          // do nothing
        }
      }
    }
  }

  fn convert_for_loop_to_sleep(&mut self, _diagnostics: &mut Vec<Diagnostic>) {
    for i in 0..self.code.len() {
      match &self.code[i].kind {
        InstrKind::ForLoop { name, has_step } => {
          if i == self.code.len() || i < 2 {
            continue;
          }
          match &self.code[i + 1].kind {
            InstrKind::NextFor { name: var } => {
              if !var.map_or(true, |var| var == *name) {
                continue;
              }
            }
            _ => continue,
          }

          let mut step = 1.0;
          let mut j = i - 1;
          if *has_step {
            if i >= 3 {
              match &self.code[j].kind {
                InstrKind::PushNum(num) if num.is_positive() => {
                  step = f64::from(*num);
                }
                _ => continue,
              }
            } else {
              continue;
            }
            j -= 1;
          }
          let steps =
            match (&self.code[j].kind, &self.code[j - 1].kind) {
              (InstrKind::PushNum(end), InstrKind::PushNum(start))
                if end.is_positive() && start.is_zero() || *start == 1.0 =>
              {
                let start = f64::from(*start);
                let end = f64::from(*end);
                let steps = ((end - start) / step).ceil();
                if let Ok(steps) = Mbf5::try_from(steps) {
                  steps
                } else {
                  continue;
                }
              }
              _ => continue,
            };
          self.code[i - 1].kind = InstrKind::NoOp;
          self.code[j - 1].kind = InstrKind::NoOp;
          self.code[j].kind = InstrKind::PushNum(steps);
          self.code[i].kind = InstrKind::Sleep;
          self.code[i + 1].kind = InstrKind::NoOp;
        }
        _ => {
          // do nothing
        }
      }
    }
  }
}
