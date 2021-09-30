use std::fmt::{self, Debug, Display, Formatter};

use super::ScreenMode;

pub trait Device {
  /// Range: [0, 4]
  fn get_row(&self) -> u8;

  /// Range: [0, 19]
  fn get_column(&self) -> u8;

  /// Range: [0, 4]
  fn set_row(&mut self, row: u8);

  /// Range: [0, 19]
  fn set_column(&mut self, column: u8);

  fn print(&mut self, str: &[u8]);

  fn print_newline(&mut self);

  fn draw_point(&mut self, x: u8, y: u8, mode: DrawMode);

  fn draw_line(&mut self, x1: u8, y1: u8, x2: u8, y2: u8, mode: DrawMode);

  fn draw_box(
    &mut self,
    x1: u8,
    y1: u8,
    x2: u8,
    y2: u8,
    fill: bool,
    mode: DrawMode,
  );

  fn draw_circle(&mut self, x: u8, y: u8, r: u8, fill: bool, mode: DrawMode);

  fn draw_ellipse(
    &mut self,
    x: u8,
    y: u8,
    rx: u8,
    ry: u8,
    fill: bool,
    mode: DrawMode,
  );

  fn clear(&mut self);

  fn get_byte(&self, addr: u16) -> u8;

  fn set_byte(&mut self, addr: u16, value: u8);

  /// Range of `filenum`: [0, 2]
  ///
  /// Returns None if the file is not open.
  fn file_status(&self, filenum: u8) -> Option<FileStatus>;

  /// Range of `filenum`: [0, 2]
  ///
  /// Returns false if the file is not open.
  fn close_file(&mut self, filenum: u8) -> bool;

  fn close_all_files(&mut self);

  fn cls(&mut self);

  /// Returns true if execution is finished, otherwise false is returned.
  /// 
  /// `steps` will be the steps left the when exec_asm() is returned.
  /// 
  /// If `start_addr` is None, continue previous unfinished execution.
  fn exec_asm(
    &mut self,
    steps: &mut usize,
    start_addr: Option<u16>,
  ) -> bool;

  fn set_screen_mode(&mut self, mode: ScreenMode);
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DrawMode {
  Erase,
  Copy,
  Not,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FileMode {
  Input,
  Output,
  Append,
  Random,
}

#[derive(Debug, Clone)]
pub struct FileStatus {
  pub pos: u32,
  pub len: u32,
  pub mode: FileMode,
}

impl Display for FileMode {
  fn fmt(&self, f: &mut Formatter) -> fmt::Result {
    match self {
      Self::Input => write!(f, "INPUT"),
      Self::Output => write!(f, "OUTPUT"),
      Self::Append => write!(f, "APPEND"),
      Self::Random => write!(f, "RANDOM"),
    }
  }
}
