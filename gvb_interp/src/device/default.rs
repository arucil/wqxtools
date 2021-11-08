use super::*;
use crate::machine::MachineProps;

pub struct DefaultDevice {
  props: MachineProps,
  memory: [u8; 65536],
  row: u8,
  column: u8,
}

pub struct DefaultFileHandle {
}

impl DefaultDevice {
  pub fn new(props: MachineProps) -> Self {
  }
}

impl Device for DefaultDevice {
  type File: DefaultFileHandle;

  fn get_row(&self) -> u8 {
    self.row
  }

  fn get_column(&self) -> u8 {
    self.column
  }

  fn set_row(&mut self, row: u8) {
    self.row = row;
  }

  fn set_column(&mut self, column: u8) {
    self.column = column;
  }

  fn print(&mut self, str: &[u8]);

  fn newline(&mut self);

  fn flush(&mut self);

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