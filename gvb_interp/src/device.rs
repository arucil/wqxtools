use std::io;

use super::{PrintMode, ScreenMode};

pub mod default;

pub trait Device {
  type File: FileHandle;

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

  fn get_byte(&self, addr: u16) -> u8;

  /// Returns true if POKE 199, 155 is executed (ESC).
  fn set_byte(&mut self, addr: u16, value: u8) -> bool;

  fn open_file(
    &mut self,
    name: &[u8],
    read: bool,
    write: bool,
    truncate: bool,
  ) -> io::Result<Self::File>;

  fn cls(&mut self);

  /// Returns true if execution is finished, otherwise false is returned.
  ///
  /// `steps` will be the steps left the when exec_asm() is returned.
  ///
  /// If `start_addr` is None, continue previous unfinished execution.
  fn exec_asm(&mut self, steps: &mut usize, start_addr: Option<u16>) -> bool;

  fn set_screen_mode(&mut self, mode: ScreenMode);

  fn set_print_mode(&mut self, mode: PrintMode);

  fn sleep_unit(&self) -> std::time::Duration;

  fn beep(&mut self);

  fn play_notes(&mut self, notes: &[u8]);

  fn clear_cursor(&mut self);
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DrawMode {
  Erase,
  Copy,
  Not,
}

pub trait FileHandle {
  fn len(&self) -> io::Result<u64>;

  fn seek(&mut self, pos: u64) -> io::Result<()>;

  fn pos(&self) -> io::Result<u64>;

  fn write(&mut self, data: &[u8]) -> io::Result<()>;

  fn read(&mut self, data: &mut [u8]) -> io::Result<usize>;

  fn close(self) -> io::Result<()>;
}