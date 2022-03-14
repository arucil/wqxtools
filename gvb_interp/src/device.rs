use std::io;

use super::{PrintMode, ScreenMode};
use crate::machine::EofBehavior;

pub mod default;

pub enum KeyCode {
  Enter = 13,
  Esc = 27,
}

pub trait Device {
  type File: FileHandle + Default;
  type AsmState;
  type AsmError;

  /// Range: [0, 4]
  fn get_row(&self) -> u8;

  /// Range: [0, 19]
  fn get_column(&self) -> u8;

  /// Range: [0, 4]
  fn set_row(&mut self, row: u8);

  /// Range: [0, 19]
  fn set_column(&mut self, column: u8);

  fn print(&mut self, str: &[u8]);

  fn newline(&mut self);

  fn flush(&mut self);

  fn draw_point(&mut self, coord: (u8, u8), mode: DrawMode);

  fn draw_line(&mut self, coord1: (u8, u8), coord2: (u8, u8), mode: DrawMode);

  fn draw_box(
    &mut self,
    coord1: (u8, u8),
    coord2: (u8, u8),
    fill: bool,
    mode: DrawMode,
  );

  fn draw_circle(&mut self, coord: (u8, u8), r: u8, fill: bool, mode: DrawMode);

  fn draw_ellipse(
    &mut self,
    coord: (u8, u8),
    radius: (u8, u8),
    fill: bool,
    mode: DrawMode,
  );

  fn check_point(&self, coord: (i32, i32)) -> bool;

  fn check_key(&self, key: u8) -> bool;

  fn key(&mut self) -> Option<u8>;

  fn read_byte(&self, addr: u16) -> u8;

  fn write_byte(&mut self, addr: u16, byte: u8);

  /// Returns true if user is pressing ESC.
  fn user_quit(&self) -> bool;

  fn open_file(
    &mut self,
    file: &mut Self::File,
    name: &[u8],
    read: bool,
    write: bool,
    truncate: bool,
  ) -> io::Result<()>;

  fn cls(&mut self);

  /// Returns Ok(Some(state)) if execution is not finished,
  /// Ok(None) if execution is finished.
  ///
  /// `steps` will be the steps left the when exec_asm() is returned.
  fn exec_asm(
    &mut self,
    steps: &mut usize,
    state: AsmExecState<Self::AsmState>,
  ) -> Result<Option<Self::AsmState>, Self::AsmError>;

  fn set_screen_mode(&mut self, mode: ScreenMode);

  fn set_print_mode(&mut self, mode: PrintMode);

  fn sleep_unit(&self) -> std::time::Duration;

  fn beep(&mut self);

  fn play_notes(&mut self, notes: &[u8]);

  fn clear_cursor(&mut self);

  fn eof_behavior(&self) -> EofBehavior;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DrawMode {
  Clear,
  Or,
  Xor,
  Unknown,
}

pub trait FileHandle {
  fn len(&self) -> io::Result<u64>;

  fn seek(&mut self, pos: u64) -> io::Result<()>;

  fn pos(&self) -> io::Result<u64>;

  fn write(&mut self, data: &[u8]) -> io::Result<()>;

  fn read(&mut self, data: &mut [u8]) -> io::Result<usize>;

  fn close(&mut self) -> io::Result<()>;

  fn is_open(&self) -> bool;
}

pub enum AsmExecState<S> {
  Start(u16),
  Cont(S),
}
