use std::convert::TryFrom;
#[cfg(test)]
use std::fmt::{self, Debug, Formatter};
use std::num::NonZeroUsize;

use super::{
  Addr, Alignment, ByteString, CmpKind, DatumIndex, Instr, InstrKind, Location,
  PrintMode, ScreenMode, StringError, Symbol, DUMMY_ADDR, FISRT_DATUM_INDEX,
};
use crate::ast::{
  BinaryOpKind, FileMode, Range, StmtKind, SysFuncKind, UnaryOpKind,
};
use crate::diagnostic::Diagnostic;
use crate::util::mbf5::Mbf5;
use crate::{
  compiler::{CharError, CodeEmitter},
  machine::EmojiVersion,
};
use string_interner::StringInterner;

use super::Datum;

#[derive(Clone)]
pub struct CodeGen {
  pub(super) emoji_version: EmojiVersion,
  pub(super) interner: StringInterner,
  pub(super) data: Vec<Datum>,
  pub(super) code: Vec<Instr>,
  cur_line: usize,
}

impl CodeGen {
  pub fn new(emoji_version: EmojiVersion) -> Self {
    Self {
      emoji_version,
      interner: StringInterner::new(),
      data: vec![],
      code: vec![],
      cur_line: 0,
    }
  }

  fn push_instr(&mut self, range: Range, kind: InstrKind) {
    self.code.push(Instr {
      loc: Location {
        line: self.cur_line,
        range,
      },
      kind,
    })
  }
}

impl CodeEmitter for CodeGen {
  type Symbol = Symbol;
  type Addr = Addr;
  type DatumIndex = DatumIndex;

  fn begin_line(&mut self, line: usize) {
    self.cur_line = line;
  }

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
        self.push_instr(range, InstrKind::PopStr);
      }
      StmtKind::Inverse => {
        self.push_instr(range, InstrKind::SetPrintMode(PrintMode::Inverse))
      }
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
      StmtKind::Run(..) => self.push_instr(range, InstrKind::Restart),
      StmtKind::Swap { .. } => self.push_instr(range, InstrKind::Swap),
      StmtKind::Text => {
        self.push_instr(range, InstrKind::SetScreenMode(ScreenMode::Text))
      }
      StmtKind::Trace => self.push_instr(range, InstrKind::SetTrace(true)),
      StmtKind::Wend => self.push_instr(range, InstrKind::Wend),
      StmtKind::Sleep(_) => self.push_instr(range, InstrKind::Sleep),
      StmtKind::Fputc { .. } => self.push_instr(range, InstrKind::Fputc),
      StmtKind::Fread { .. } => self.push_instr(range, InstrKind::Fread),
      StmtKind::Fwrite { .. } => self.push_instr(range, InstrKind::Fwrite),
      StmtKind::Fseek { .. } => self.push_instr(range, InstrKind::Fseek),
      _ => unreachable!(),
    }
  }

  fn emit_datum(
    &mut self,
    range: Range,
    value: String,
    is_quoted: bool,
  ) -> Result<(Self::DatumIndex, usize), CharError> {
    let index = DatumIndex(self.data.len());
    let value = match ByteString::from_str(value, self.emoji_version, true) {
      Ok(value) => value,
      Err(StringError::InvalidChar(i, c)) => {
        return Err(CharError {
          range: Range::new(i, i + c.len_utf8())
            .offset(range.start as isize + is_quoted as isize),
          char: c,
        })
      }
    };
    let len = value.len();
    self.data.push(Datum { value, is_quoted });
    Ok((index, len))
  }

  fn begin_def_fn(
    &mut self,
    range: Range,
    name: Self::Symbol,
    param: Self::Symbol,
  ) -> Self::Addr {
    let addr = Addr(self.code.len());
    self.push_instr(
      range,
      InstrKind::DefFn {
        name,
        param,
        end: DUMMY_ADDR,
      },
    );
    addr
  }

  fn end_def_fn(&mut self, def_addr: Self::Addr) {
    let loc = self.code[def_addr.0].loc.clone();
    self.code.push(Instr {
      loc,
      kind: InstrKind::ReturnFn,
    });
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

  fn emit_var_lvalue(&mut self, range: Range, name: Self::Symbol) {
    self.push_instr(range, InstrKind::PushVarLValue { name });
  }

  fn emit_index_lvalue(
    &mut self,
    range: Range,
    name: Self::Symbol,
    dimensions: NonZeroUsize,
  ) {
    self.push_instr(range, InstrKind::PushIndexLValue { name, dimensions });
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

  fn emit_assign_int(&mut self, range: Range) {
    self.push_instr(range, InstrKind::AssignInt);
  }

  fn emit_assign_real(&mut self, range: Range) {
    self.push_instr(range, InstrKind::AssignReal);
  }

  fn emit_assign_str(&mut self, range: Range) {
    self.push_instr(range, InstrKind::AssignStr);
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
    has_prompt: bool,
    fields: NonZeroUsize,
  ) {
    self.push_instr(range, InstrKind::KeyboardInput { has_prompt, fields });
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

  fn emit_newline(&mut self, range: Range) {
    self.push_instr(range, InstrKind::NewLine);
  }

  fn emit_print_spc(&mut self, range: Range) {
    self.push_instr(range, InstrKind::PrintSpc);
  }

  fn emit_print_tab(&mut self, range: Range) {
    self.push_instr(range, InstrKind::PrintTab);
  }

  fn emit_print_num(&mut self, range: Range) {
    self.push_instr(range, InstrKind::PrintNum);
  }

  fn emit_print_str(&mut self, range: Range) {
    self.push_instr(range, InstrKind::PrintStr);
  }

  fn emit_flush(&mut self, range: Range) {
    self.push_instr(range, InstrKind::Flush);
  }

  fn emit_pop_num(&mut self, range: Range) {
    self.push_instr(range, InstrKind::PopNum);
  }

  fn emit_pop_str(&mut self, range: Range) {
    self.push_instr(range, InstrKind::PopStr);
  }

  fn emit_write_num(&mut self, range: Range, to_file: bool, end: bool) {
    self.push_instr(range, InstrKind::WriteNum { to_file, end });
  }

  fn emit_write_str(&mut self, range: Range, to_file: bool, end: bool) {
    self.push_instr(range, InstrKind::WriteStr { to_file, end });
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

  fn emit_string(
    &mut self,
    range: Range,
    str: String,
  ) -> Result<usize, CharError> {
    let str = match ByteString::from_str(str, self.emoji_version, true) {
      Ok(str) => str,
      Err(StringError::InvalidChar(i, c)) => {
        return Err(CharError {
          range: Range::new(i, i + c.len_utf8()).offset((range.start + 1) as _),
          char: c,
        })
      }
    };
    let len = str.len();
    self.push_instr(range, InstrKind::PushStr(str));
    Ok(len)
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

  fn emit_num_binary_expr(&mut self, range: Range, kind: BinaryOpKind) {
    let kind = match kind {
      BinaryOpKind::Eq => InstrKind::CmpNum(CmpKind::Eq),
      BinaryOpKind::Ne => InstrKind::CmpNum(CmpKind::Ne),
      BinaryOpKind::Gt => InstrKind::CmpNum(CmpKind::Gt),
      BinaryOpKind::Lt => InstrKind::CmpNum(CmpKind::Lt),
      BinaryOpKind::Ge => InstrKind::CmpNum(CmpKind::Ge),
      BinaryOpKind::Le => InstrKind::CmpNum(CmpKind::Le),
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

  fn emit_str_binary_expr(&mut self, range: Range, kind: BinaryOpKind) {
    let kind = match kind {
      BinaryOpKind::Eq => InstrKind::CmpStr(CmpKind::Eq),
      BinaryOpKind::Ne => InstrKind::CmpStr(CmpKind::Ne),
      BinaryOpKind::Gt => InstrKind::CmpStr(CmpKind::Gt),
      BinaryOpKind::Lt => InstrKind::CmpStr(CmpKind::Lt),
      BinaryOpKind::Ge => InstrKind::CmpStr(CmpKind::Ge),
      BinaryOpKind::Le => InstrKind::CmpStr(CmpKind::Le),
      BinaryOpKind::Add => InstrKind::Concat,
      _ => unreachable!(),
    };
    self.push_instr(range, kind);
  }

  fn emit_user_func_call(&mut self, range: Range, name: Self::Symbol) {
    self.push_instr(range, InstrKind::CallFn(name));
  }

  fn emit_sys_func_call(
    &mut self,
    range: Range,
    kind: SysFuncKind,
    arity: NonZeroUsize,
  ) {
    self.push_instr(range, InstrKind::SysFuncCall { kind, arity });
  }

  fn clean_up(&mut self) -> Vec<(usize, Diagnostic)> {
    let mut diags = vec![];
    self.patch_while_instr(&mut diags);
    self.convert_for_loop_to_sleep(&mut diags);
    self.push_instr(Range::empty(0), InstrKind::End);
    diags
  }
}

impl CodeGen {
  fn patch_while_instr(&mut self, diagnostics: &mut Vec<(usize, Diagnostic)>) {
    let mut wend_stack: Vec<Addr> = vec![];

    for (i, instr) in self.code.iter_mut().enumerate().rev() {
      match &mut instr.kind {
        InstrKind::Wend => wend_stack.push(Addr(i + 1)),
        InstrKind::WhileLoop { end, .. } => {
          if let Some(i) = wend_stack.pop() {
            *end = i;
          } else {
            diagnostics.push((
              instr.loc.line,
              Diagnostic::new_error(
                instr.loc.range.clone(),
                "WHILE 语句没有对应的 WEND 语句",
              ),
            ));
          }
        }
        _ => {
          // do nothing
        }
      }
    }
  }

  fn convert_for_loop_to_sleep(
    &mut self,
    _diagnostics: &mut Vec<(usize, Diagnostic)>,
  ) {
    for i in 0..self.code.len() {
      match &self.code[i].kind {
        InstrKind::ForLoop { name, has_step } => {
          if i == self.code.len() - 1 || i < 2 {
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

          let mut end_index = i - 1;
          if *has_step {
            if i >= 3
              && matches!(
                &self.code[end_index].kind,
                InstrKind::PushNum(num) if num.is_one()
              )
            {
            } else {
              continue;
            }
            end_index -= 1;
          }
          if let InstrKind::PushNum(start) = &self.code[end_index - 1].kind {
            if start.is_zero() || *start == 1.0 {
              match &self.code[end_index].kind {
                InstrKind::PushNum(end) if end.is_positive() => {
                  let end = f64::from(*end);
                  let steps = end.ceil();
                  if let Ok(steps) = Mbf5::try_from(steps) {
                    self.code[i - 1].kind = InstrKind::NoOp;
                    self.code[end_index - 1].kind = InstrKind::NoOp;
                    self.code[end_index].kind = InstrKind::PushNum(steps);
                    self.code[i].kind = InstrKind::Sleep;
                    self.code[i + 1].kind = InstrKind::NoOp;
                  }
                }
                InstrKind::PushVar { .. } => {
                  if *has_step {
                    self.code[i - 1].kind = InstrKind::NoOp;
                  }
                  self.code[end_index - 1].kind = InstrKind::NoOp;
                  self.code[i].kind = InstrKind::Sleep;
                  self.code[i + 1].kind = InstrKind::NoOp;
                }
                _ => {}
              }
            }
          }
        }
        _ => {
          // do nothing
        }
      }
    }
  }
}

#[cfg(test)]
impl Debug for CodeGen {
  fn fmt(&self, f: &mut Formatter) -> fmt::Result {
    writeln!(f, "emoji_version: {:?}", self.emoji_version)?;
    writeln!(f, "--------- data ----------")?;
    for (i, datum) in self.data.iter().enumerate() {
      let quote = if datum.is_quoted { "\"" } else { "" };
      writeln!(
        f,
        "{:<6}{}{}{}",
        i,
        quote,
        datum.value.to_string_lossy(self.emoji_version),
        quote
      )?;
    }
    writeln!(f, "--------- code ----------")?;
    for (i, instr) in self.code.iter().enumerate() {
      writeln!(
        f,
        "{:<6}{}",
        i,
        instr.print(&self.interner, self.emoji_version)
      )?;
    }
    Ok(())
  }
}
