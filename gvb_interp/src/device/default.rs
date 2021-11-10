use super::*;
use crate::machine::MachineProps;
use std::io;

const TEXT_BUFFER_ADDR: usize = 0x2c0;
const KEY_BUFFER_ADDR: usize = 199;
const CHAR_HEIGHT: usize = 16;

const TEXT_COLUMNS: usize = 20;
const TEXT_ROWS: usize = 5;
const TEXT_BYTES: usize = TEXT_COLUMNS * TEXT_ROWS;

const ASCII_8_DATA: &[u8] = include_bytes!("data/ascii_8.dat");
const ASCII_16_DATA: &[u8] = include_bytes!("data/ascii_16.dat");
const GB2312_16_DATA: &[u8] = include_bytes!("data/gb2312_16.dat");
const EMOJI_16_DATA: &[u8] = include_bytes!("data/emoji_16.dat");

mod screen {
  pub const WIDTH: usize = 160;
  pub const HEIGHT: usize = 80;
  pub const WIDTH_IN_BYTE: usize = WIDTH >> 3;
  pub const BYTES: usize = WIDTH_IN_BYTE * HEIGHT;
}

pub struct DefaultDevice {
  props: MachineProps,
  memory: [u8; 65536],
  inverse_text: [bool; TEXT_BYTES],
  row: u8,
  column: u8,
  screen_mode: ScreenMode,
  print_mode: PrintMode,
  cursor: CursorState,
  graphics_dirty: Option<Rect>,
}

pub struct Rect {
  pub left: usize,
  pub top: usize,
  pub right: usize,
  pub bottom: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CursorState {
  None,
  HalfWidth,
  FullWidth,
}

pub struct DefaultFileHandle {}

impl DefaultDevice {
  pub fn new(props: MachineProps) -> Self {
    Self {
      props,
      memory: [0; 65536],
      inverse_text: [false; TEXT_BYTES],
      row: 0,
      column: 0,
      screen_mode: ScreenMode::Text,
      print_mode: PrintMode::Normal,
      cursor: CursorState::None,
      graphics_dirty: None,
    }
  }

  pub fn fire_key_down(&mut self, key: u8) {
    todo!()
  }

  pub fn fire_key_up(&mut self, key: u8) {
    todo!()
  }

  pub fn key(&mut self) -> Option<u8> {
    let key = self.memory[KEY_BUFFER_ADDR];
    if key < 128 {
      None
    } else {
      self.memory[KEY_BUFFER_ADDR] &= 0x7f;
      Some(key & 0x7f)
    }
  }

  pub fn blink_cursor(&mut self) {
    if self.screen_mode != ScreenMode::Text {
      return;
    }

    if self.cursor == CursorState::None {
      let char_addr = TEXT_BUFFER_ADDR
        + self.row as usize * TEXT_COLUMNS
        + self.column as usize;
      self.cursor = if self.memory[char_addr] >= 128 {
        CursorState::FullWidth
      } else {
        CursorState::HalfWidth
      };
    }

    self.inverse_cursor();
  }

  pub fn graphics_addr(&self) -> &[u8] {
    let base_addr = self.props.graphics_base_addr as usize;
    &self.memory[base_addr..base_addr + screen::BYTES]
  }

  pub fn take_dirty_area(&mut self) -> Option<Rect> {
    self.graphics_dirty.take()
  }

  fn inverse_cursor(&mut self) {
    use screen as s;
    let mut graph_addr = self.props.graphics_base_addr as usize
      + self.row as usize * s::WIDTH_IN_BYTE * CHAR_HEIGHT
      + self.column as usize;
    for i in (0..s::WIDTH_IN_BYTE * CHAR_HEIGHT).step_by(s::WIDTH_IN_BYTE) {
      self.memory[graph_addr + i] ^= 0xff;
    }

    if self.cursor == CursorState::FullWidth && self.column < 19 {
      graph_addr += 1;
      for i in (0..s::WIDTH_IN_BYTE * CHAR_HEIGHT).step_by(s::WIDTH_IN_BYTE) {
        self.memory[graph_addr + i] ^= 0xff;
      }
    }
  }

  fn scroll_text(&mut self) {
    if self.screen_mode == ScreenMode::Graph {
      use screen as s;
      let graph_addr = self.props.graphics_base_addr as usize;
      self.memory.copy_within(
        graph_addr + s::WIDTH_IN_BYTE * CHAR_HEIGHT..graph_addr + s::BYTES,
        graph_addr,
      );
      self.memory[graph_addr + s::BYTES - s::WIDTH_IN_BYTE * CHAR_HEIGHT
        ..graph_addr + s::BYTES]
        .fill(0);
    }

    self.memory.copy_within(
      TEXT_BUFFER_ADDR + TEXT_COLUMNS
        ..TEXT_BUFFER_ADDR + TEXT_COLUMNS * TEXT_ROWS,
      TEXT_BUFFER_ADDR,
    );
    self.memory[TEXT_BUFFER_ADDR + TEXT_COLUMNS * (TEXT_ROWS - 1)
      ..TEXT_BUFFER_ADDR + TEXT_COLUMNS * TEXT_ROWS]
      .fill(0);

    self.inverse_text.copy_within(TEXT_COLUMNS.., 0);
    self.inverse_text[TEXT_COLUMNS * (TEXT_ROWS - 1)..].fill(false);
  }

  fn paint_hex_code(&mut self, row: usize, column: usize) {
    let mut c = self.memory[TEXT_BUFFER_ADDR + row * TEXT_COLUMNS + column];
    unsafe {
      let mut g = self.memory.as_mut_ptr().add(
        self.props.graphics_base_addr as usize
          + row * screen::WIDTH_IN_BYTE
          + column,
      );
      for _ in 0..2 {
        let mut ptr = Self::nibble_to_ascii8_ptr(c >> 4);
        c <<= 4;
        for _ in 0..8 {
          *g = *ptr;
          ptr = ptr.add(1);
          g = g.add(screen::WIDTH_IN_BYTE);
        }
      }
    }
  }

  unsafe fn nibble_to_ascii8_ptr(n: u8) -> *const u8 {
    if n < 10 {
      ASCII_8_DATA.as_ptr().add((48 + n as usize) << 3)
    } else {
      ASCII_8_DATA.as_ptr().add((65 + n as usize - 10) << 3)
    }
  }

  fn update_dirty_area(
    &mut self,
    left: usize,
    top: usize,
    right: usize,
    bottom: usize,
  ) {
    if let Some(dirty) = self.graphics_dirty.as_mut() {
      if left < dirty.left {
        dirty.left = left;
      }
      if top < dirty.top {
        dirty.top = top;
      }
      if right > dirty.right {
        dirty.right = right;
      }
      if bottom > dirty.bottom {
        dirty.bottom = bottom;
      }
    } else {
      self.graphics_dirty = Some(Rect {
        left,
        top,
        right,
        bottom,
      });
    }
  }
}

impl Device for DefaultDevice {
  type File = DefaultFileHandle;

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

  fn print(&mut self, str: &[u8]) {
    let text_buffer = unsafe { self.memory.as_mut_ptr().add(TEXT_BUFFER_ADDR) };
    for &c in str {
      if c >= 128 && self.row as usize == TEXT_COLUMNS - 1 {
        unsafe {
          *text_buffer
            .add(self.row as usize * TEXT_COLUMNS + self.column as usize) =
            b' ';
        }
        self.newline();
      }
      unsafe {
        *text_buffer
          .add(self.row as usize * TEXT_COLUMNS + self.column as usize) = c;
      }
      self.column += 1;
      if self.column as usize == TEXT_COLUMNS {
        self.newline();
      }
    }

    let mut i = self.row as usize * TEXT_COLUMNS + self.column as usize;
    unsafe {
      while i < 100 && *text_buffer.add(i) != 0 {
        *text_buffer.add(i) = 0;
        i += 1;
      }
    }
  }

  fn newline(&mut self) {
    if self.column == 0 {
      return;
    }
    if self.row as usize == TEXT_ROWS - 1 {
      self.scroll_text();
      self.column = 0;
    } else {
      self.row += 1;
    }
  }

  fn flush(&mut self) {
    if self.screen_mode == ScreenMode::Text {
      let graph_addr = self.props.graphics_base_addr as usize;
      self.memory[graph_addr..graph_addr + screen::BYTES].fill(0);
    }

    let mut char_ptr = unsafe { self.memory.as_ptr().add(TEXT_BUFFER_ADDR) };
    let mut graph = unsafe {
      self
        .memory
        .as_mut_ptr()
        .add(self.props.graphics_base_addr as usize)
    };
    let mut row = 0;
    while row < TEXT_ROWS {
      let mut col = 0;
      while col < TEXT_COLUMNS {
        let c = unsafe { *char_ptr };
        if c == 0 {
          char_ptr = unsafe { char_ptr.add(1) };
          col += 1;
          continue;
        }

        if c < 128 {
          let mut g = unsafe { graph.add(col) };
          let mut ascii_ptr =
            unsafe { ASCII_16_DATA.as_ptr().add(c as usize * CHAR_HEIGHT) };
          for _ in 0..CHAR_HEIGHT {
            unsafe {
              *g = *ascii_ptr;
              g = g.add(screen::WIDTH_IN_BYTE);
              ascii_ptr = ascii_ptr.add(1);
            }
          }
          char_ptr = unsafe { char_ptr.add(1) };
          col += 1;
          continue;
        }

        if row == TEXT_ROWS - 1 && col == TEXT_COLUMNS - 1 {
          self.paint_hex_code(row, col);
          char_ptr = unsafe { char_ptr.add(1) };
          col += 1;
          continue;
        }

        let c2 = unsafe { *char_ptr.add(1) };

        let mut data_ptr;
        if let Some(emoji_index) = self
          .props
          .emoji_style
          .code_to_index((c as u16) << 8 | c2 as u16)
        {
          data_ptr = unsafe {
            EMOJI_16_DATA.as_ptr().add(emoji_index * 2 * CHAR_HEIGHT)
          };
        } else if c >= 161 && c2 >= 161 {
          let mut gb_index = c as usize - 161;
          if gb_index > 8 {
            gb_index -= 6;
          }
          let gb_index =
            (gb_index * 94 + (c2 as usize - 161)) * 2 * CHAR_HEIGHT;
          if gb_index + 2 * CHAR_HEIGHT > GB2312_16_DATA.len() {
            self.paint_hex_code(row, col);
            char_ptr = unsafe { char_ptr.add(1) };
            col += 1;
            continue;
          }

          data_ptr =
            unsafe { GB2312_16_DATA.as_ptr().add(gb_index * 2 * CHAR_HEIGHT) };
        } else {
          self.paint_hex_code(row, col);
          char_ptr = unsafe { char_ptr.add(1) };
          col += 1;
          continue;
        }

        let mut g = unsafe { graph.add(col) };
        if col == TEXT_COLUMNS - 1 {
          // 汉字位于行尾时分成两半显示...
          for _ in 0..CHAR_HEIGHT {
            unsafe {
              *g = *data_ptr;
              *g.add(screen::WIDTH_IN_BYTE * (CHAR_HEIGHT - 1) + 1) =
                *data_ptr.add(1);
              g = g.add(screen::WIDTH_IN_BYTE);
              data_ptr = data_ptr.add(2);
            }
          }
        } else {
          for _ in 0..CHAR_HEIGHT {
            unsafe {
              *g = *data_ptr;
              *g.add(1) = *data_ptr.add(1);
              g = g.add(screen::WIDTH_IN_BYTE);
              data_ptr = data_ptr.add(2);
            }
          }
        }

        char_ptr = unsafe { char_ptr.add(2) };
        col += 2;
      }
      row += 1;
      graph = unsafe { graph.add(screen::WIDTH_IN_BYTE * CHAR_HEIGHT) };
    }

    // TODO finer grained dirty area
    self.update_dirty_area(0, 0, screen::WIDTH, screen::HEIGHT);
  }

  fn draw_point(&mut self, x: u8, y: u8, mode: DrawMode) {}

  fn draw_line(&mut self, x1: u8, y1: u8, x2: u8, y2: u8, mode: DrawMode) {}

  fn draw_box(
    &mut self,
    x1: u8,
    y1: u8,
    x2: u8,
    y2: u8,
    fill: bool,
    mode: DrawMode,
  ) {
  }

  fn draw_circle(&mut self, x: u8, y: u8, r: u8, fill: bool, mode: DrawMode) {}

  fn draw_ellipse(
    &mut self,
    x: u8,
    y: u8,
    rx: u8,
    ry: u8,
    fill: bool,
    mode: DrawMode,
  ) {
  }

  fn get_byte(&self, addr: u16) -> u8 {
    self.memory[addr as usize]
  }

  fn set_byte(&mut self, addr: u16, value: u8) -> bool {
    self.memory[addr as usize] = value;
    if addr >= self.props.graphics_base_addr
      && addr < self.props.graphics_base_addr + screen::BYTES as u16
    {
      let index = (addr - self.props.graphics_base_addr) as usize;
      let y = index / screen::WIDTH_IN_BYTE;
      let x = (index % screen::WIDTH_IN_BYTE) << 3;
      self.update_dirty_area(y, x, y + 1, x + 8);
    }
    addr as usize == TEXT_BUFFER_ADDR && value == 128 + 27
  }

  fn open_file(
    &mut self,
    name: &[u8],
    read: bool,
    write: bool,
    truncate: bool,
  ) -> io::Result<Self::File> {
    todo!()
  }

  fn cls(&mut self) {
    self.memory[TEXT_BUFFER_ADDR..TEXT_BUFFER_ADDR + TEXT_BYTES].fill(0);
    let graph_addr = self.props.graphics_base_addr as usize;
    self.memory[graph_addr..graph_addr + screen::BYTES].fill(0);
    self.inverse_text.fill(false);
    self.row = 0;
    self.column = 0;
    self.update_dirty_area(0, 0, screen::WIDTH, screen::HEIGHT);
  }

  /// Returns true if execution is finished, otherwise false is returned.
  ///
  /// `steps` will be the steps left the when exec_asm() is returned.
  ///
  /// If `start_addr` is None, continue previous unfinished execution.
  fn exec_asm(&mut self, steps: &mut usize, start_addr: Option<u16>) -> bool {
    todo!()
  }

  fn set_screen_mode(&mut self, mode: ScreenMode) {
    self.screen_mode = mode;
    self.cls();
  }

  fn set_print_mode(&mut self, mode: PrintMode) {
    self.print_mode = match (self.print_mode, mode) {
      (PrintMode::Inverse, PrintMode::Flash) => PrintMode::Normal,
      _ => mode,
    };
  }

  fn sleep_unit(&self) -> std::time::Duration {
    self.props.sleep_unit
  }

  fn beep(&mut self) {
    // do nothing
  }

  fn play_notes(&mut self, _notes: &[u8]) {
    // do nothing
  }

  fn clear_cursor(&mut self) {
    if self.cursor == CursorState::None {
      return;
    }

    self.inverse_cursor();

    self.cursor = CursorState::None;
  }
}

impl FileHandle for DefaultFileHandle {
  fn len(&self) -> io::Result<u64> {
    todo!()
  }

  fn seek(&mut self, pos: u64) -> io::Result<()> {
    todo!()
  }

  fn pos(&self) -> io::Result<u64> {
    todo!()
  }

  fn write(&mut self, data: &[u8]) -> io::Result<()> {
    todo!()
  }

  fn read(&mut self, data: &mut [u8]) -> io::Result<usize> {
    todo!()
  }

  fn close(self) -> io::Result<()> {
    todo!()
  }
}
