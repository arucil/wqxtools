use super::*;

pub trait Device {
  /// Range: [1, 5]
  fn get_row(&self) -> u32;

  /// Range: [1, 20]
  fn get_column(&self) -> u32;

  /// Range: [1, 5]
  fn set_row(&mut self, row: u32);

  /// Range: [1, 20]
  fn set_column(&mut self, column: u32);

  fn print(&mut self, str: &ByteString);

  fn print_newline(&mut self);

  /// Range of x, y: [0, 255]  
  fn draw_point(&mut self, x: u32, y: u32, mode: DrawMode);

  /// Range of x1, y1, x2, y2: [0, 255]  
  fn draw_line(&mut self, x1: u32, y1: u32, x2: u32, y2: u32, mode: DrawMode);

  /// Range of x1, y1, x2, y2: [0, 255]  
  fn draw_box(
    &mut self,
    x1: u32,
    y1: u32,
    x2: u32,
    y2: u32,
    fill: bool,
    mode: DrawMode,
  );

  /// Range of x, y: [0, 255]  
  fn draw_circle(&mut self, x: u32, y: u32, r: u32, fill: bool, mode: DrawMode);

  /// Range of x, y: [0, 255]  
  fn draw_ellipse(
    &mut self,
    x: u32,
    y: u32,
    rx: u32,
    ry: u32,
    fill: bool,
    mode: DrawMode,
  );

  fn clear(&mut self);
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DrawMode {
  Erase,
  Copy,
  Not,
}
