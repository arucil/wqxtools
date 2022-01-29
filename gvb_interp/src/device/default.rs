use super::*;
use crate::machine::{AddrProp, MachineProps};
use crate::ByteString;
use chrono::prelude::*;
use emulator_6502::{Interface6502, MOS6502};
use std::fs::{File as FsFile, OpenOptions};
use std::io::{self, prelude::*, SeekFrom};
use std::path::PathBuf;

const CHAR_HEIGHT: usize = 16;

const TEXT_COLUMNS: usize = 20;
const TEXT_ROWS: usize = 5;
const TEXT_BYTES: usize = TEXT_COLUMNS * TEXT_ROWS;

const ASCII_8_DATA: &[u8] = include_bytes!("../../data/ascii_8.dat");
const ASCII_16_DATA: &[u8] = include_bytes!("../../data/ascii_16.dat");
const GB2312_16_DATA: &[u8] = include_bytes!("../../data/gb2312_16.dat");
const EMOJI_16_DATA: &[u8] = include_bytes!("../../data/emoji_16.dat");

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
  data_dir: PathBuf,
  /// NOTE key mapping must be zero page address.
  key_mapping_addr_set: [u32; 8],
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

pub struct DefaultFileHandle {
  state: FileState,
  pos: usize,
}

enum FileState {
  Open {
    file: FsFile,
    data: Vec<u8>,
    dirty: bool,
  },
  Closed {
    len: usize,
  },
}

impl DefaultDevice {
  pub(crate) fn new<P>(props: MachineProps, data_dir: P) -> Self
  where
    P: Into<PathBuf>,
  {
    let mut d = Self {
      props,
      memory: [0; 65536],
      inverse_text: [false; TEXT_BYTES],
      row: 0,
      column: 0,
      screen_mode: ScreenMode::Text,
      print_mode: PrintMode::Normal,
      cursor: CursorState::None,
      graphics_dirty: None,
      data_dir: data_dir.into(),
      key_mapping_addr_set: [0; 8],
    };
    for &addr in &d.props.key_mapping_addrs {
      d.key_mapping_addr_set[addr as usize >> 5] |= 1 << (addr & 31);
    }
    d.memory[0xffff] = 0x40; // RTI
    d.reset();
    d
  }

  pub fn reset(&mut self) {
    self.memory.fill(0);
    for &addr in &self.props.key_mapping_addrs {
      self.memory[addr as usize] = 0xff;
    }
    self.inverse_text.fill(false);
    self.row = 0;
    self.column = 0;
    self.screen_mode = ScreenMode::Text;
    self.print_mode = PrintMode::Normal;
    self.cursor = CursorState::None;
    self.graphics_dirty = None;
  }

  pub fn fire_key_down(&mut self, key: u8) {
    self.memory[self.props.key_buffer_addr as usize] = key | 0x80;
    if let Some((addr, mask)) = self.props.key_masks[key as usize] {
      self.memory[addr as usize] &= !mask;
    }
  }

  pub fn fire_key_up(&mut self, key: u8) {
    if let Some((addr, mask)) = self.props.key_masks[key as usize] {
      self.memory[addr as usize] |= mask;
    }
  }

  pub fn blink_cursor(&mut self) {
    if self.screen_mode != ScreenMode::Text {
      return;
    }

    if self.cursor == CursorState::None {
      let char_addr = self.props.text_buffer_base_addr as usize
        + self.row as usize * TEXT_COLUMNS
        + self.column as usize;
      self.cursor = if self.memory[char_addr] >= 128 {
        CursorState::FullWidth
      } else {
        CursorState::HalfWidth
      };
      self.inverse_cursor(self.cursor);
    } else {
      self.inverse_cursor(self.cursor);
      self.cursor = CursorState::None;
    }
  }

  #[cfg(test)]
  fn text_buffer(&self) -> &[u8] {
    &self.memory[self.props.text_buffer_base_addr as usize
      ..self.props.text_buffer_base_addr as usize + TEXT_ROWS * TEXT_COLUMNS]
  }

  pub fn graphic_memory(&self) -> &[u8] {
    let base_addr = self.props.graphics_base_addr as usize;
    &self.memory[base_addr..base_addr + screen::BYTES]
  }

  pub fn take_dirty_area(&mut self) -> Option<Rect> {
    self.graphics_dirty.take()
  }

  fn inverse_cursor(&mut self, cursor: CursorState) {
    use screen as s;
    let mut graph_addr = self.props.graphics_base_addr as usize
      + self.row as usize * s::WIDTH_IN_BYTE * CHAR_HEIGHT
      + self.column as usize;
    for i in (0..s::WIDTH_IN_BYTE * CHAR_HEIGHT).step_by(s::WIDTH_IN_BYTE) {
      self.memory[graph_addr + i] ^= 0xff;
    }

    if cursor == CursorState::FullWidth && self.column < 19 {
      graph_addr += 1;
      for i in (0..s::WIDTH_IN_BYTE * CHAR_HEIGHT).step_by(s::WIDTH_IN_BYTE) {
        self.memory[graph_addr + i] ^= 0xff;
      }
    }

    let left = (self.column as usize) << 3;
    let top = self.row as usize * CHAR_HEIGHT;
    let right = if cursor == CursorState::FullWidth {
      left + 16
    } else {
      left + 8
    };
    let bottom = top + CHAR_HEIGHT;
    self.update_dirty_area(left, top, right, bottom);
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

    let text_buffer_addr = self.props.text_buffer_base_addr as usize;
    self.memory.copy_within(
      text_buffer_addr + TEXT_COLUMNS
        ..text_buffer_addr + TEXT_COLUMNS * TEXT_ROWS,
      text_buffer_addr,
    );
    self.memory[text_buffer_addr + TEXT_COLUMNS * (TEXT_ROWS - 1)
      ..text_buffer_addr + TEXT_COLUMNS * TEXT_ROWS]
      .fill(0);

    self.inverse_text.copy_within(TEXT_COLUMNS.., 0);
    self.inverse_text[TEXT_COLUMNS * (TEXT_ROWS - 1)..].fill(false);
  }

  fn paint_hex_code(&mut self, row: usize, column: usize, mut c: u8) {
    unsafe {
      let mut g = self.memory.as_mut_ptr().add(
        self.props.graphics_base_addr as usize
          + row * screen::WIDTH_IN_BYTE * CHAR_HEIGHT
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

  unsafe fn draw_hor_line_unchecked(
    ptr: *mut u8,
    x1: u8,
    x2: u8,
    y: u8,
    mode: DrawMode,
  ) {
    let mut g = ptr.add(y as usize * screen::WIDTH_IN_BYTE);
    let x1_byte = x1 >> 3;
    let x2_byte = x2 >> 3;
    let start_mask = START_BIT_MASK[x1 as usize & 7];
    let end_mask = END_BIT_MASK[x2 as usize & 7];
    if x1_byte == x2_byte {
      mode.mask(g.add(x1_byte as usize), start_mask & end_mask);
      return;
    }

    mode.mask(g.add(x2_byte as usize), end_mask);
    g = g.add(x1_byte as usize);
    mode.mask(g, start_mask);
    for _ in x1_byte + 1..x2_byte {
      g = g.add(1);
      mode.mask(g, 255);
    }
  }

  fn draw_hor_line(&mut self, left: u8, mut right: u8, y: u8, mode: DrawMode) {
    if y >= screen::HEIGHT as u8 {
      return;
    }
    if left >= screen::WIDTH as u8 {
      return;
    }
    if right >= screen::WIDTH as u8 {
      right = screen::WIDTH as u8 - 1;
    }

    unsafe {
      Self::draw_hor_line_unchecked(
        self
          .memory
          .as_mut_ptr()
          .add(self.props.graphics_base_addr as usize),
        left,
        right,
        y,
        mode,
      );
    }
  }

  fn draw_ver_line(&mut self, x: u8, top: u8, mut bottom: u8, mode: DrawMode) {
    if x >= screen::WIDTH as u8 {
      return;
    }

    if top >= screen::HEIGHT as u8 {
      return;
    }
    if bottom >= screen::HEIGHT as u8 {
      bottom = screen::HEIGHT as u8 - 1;
    }

    let mut g = unsafe {
      self.memory.as_mut_ptr().add(
        self.props.graphics_base_addr as usize
          + top as usize * screen::WIDTH_IN_BYTE
          + (x as usize >> 3),
      )
    };
    let mask = POINT_BIT_MASK[x as usize & 7];
    unsafe {
      for _ in top..=bottom {
        mode.mask(g, mask);
        g = g.add(screen::WIDTH_IN_BYTE);
      }
    }
  }

  unsafe fn ellipse_hor_line(
    ptr: *mut u8,
    mut x1: i32,
    mut x2: i32,
    y: i32,
    mode: DrawMode,
  ) {
    if y >= screen::HEIGHT as i32 {
      return;
    }
    if x1 > x2 {
      let t = x1;
      x1 = x2;
      x2 = t;
    }
    if x1 >= screen::WIDTH as i32 {
      return;
    }
    if x2 >= screen::WIDTH as i32 {
      x2 = screen::WIDTH as i32 - 1;
    }

    Self::draw_hor_line_unchecked(ptr, x1 as u8, x2 as u8, y as u8, mode);
  }

  unsafe fn ellipse_point(ptr: *mut u8, x: i32, y: i32, mode: DrawMode) {
    if x >= 0 && x < screen::WIDTH as i32 && y >= 0 && y < screen::HEIGHT as i32
    {
      mode.point(ptr, x as usize, y as usize);
    }
  }

  unsafe fn ellipse_part(
    ptr: *mut u8,
    x: i32,
    y: i32,
    rx: i32,
    ry: i32,
    fill: bool,
    mode: DrawMode,
  ) {
    if fill {
      Self::ellipse_hor_line(ptr, x - rx, x + rx, y - ry, mode);
      Self::ellipse_hor_line(ptr, x - rx, x + rx, y + ry, mode);
    } else {
      Self::ellipse_point(ptr, x - rx, y - ry, mode);
      Self::ellipse_point(ptr, x + rx, y - ry, mode);
      Self::ellipse_point(ptr, x - rx, y + ry, mode);
      Self::ellipse_point(ptr, x + rx, y + ry, mode);
    }
  }
}

const START_BIT_MASK: &[u8] = &[255, 127, 63, 31, 15, 7, 3, 1];
const END_BIT_MASK: &[u8] = &[128, 192, 224, 240, 248, 252, 254, 255];
const POINT_BIT_MASK: &[u8] = &[128, 64, 32, 16, 8, 4, 2, 1];

impl DrawMode {
  unsafe fn point(&self, ptr: *mut u8, x: usize, y: usize) {
    let ptr = ptr.add(y * 20 + (x >> 3));
    match self {
      Self::Copy => *ptr |= POINT_BIT_MASK[x & 7],
      Self::Erase => *ptr &= !POINT_BIT_MASK[x & 7],
      Self::Not => *ptr ^= POINT_BIT_MASK[x & 7],
    }
  }

  unsafe fn mask(&self, ptr: *mut u8, mask: u8) {
    match self {
      Self::Copy => *ptr |= mask,
      Self::Erase => *ptr &= !mask,
      Self::Not => *ptr ^= mask,
    }
  }
}

impl Device for DefaultDevice {
  type File = DefaultFileHandle;
  type AsmState = MOS6502;

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
    let inversed = self.print_mode != PrintMode::Normal;
    let text_buffer = unsafe {
      self
        .memory
        .as_mut_ptr()
        .add(self.props.text_buffer_base_addr as usize)
    };
    let inv_buffer = self.inverse_text.as_mut_ptr();
    let mut i = 0;
    while i < str.len() {
      let c = str[i];
      if c >= 128 && self.column as usize == TEXT_COLUMNS - 1 {
        let i = self.row as usize * TEXT_COLUMNS + self.column as usize;
        unsafe {
          *text_buffer.add(i) = b' ';
          *inv_buffer.add(i) = inversed;
        }
        self.newline();
      }
      unsafe {
        let offset = self.row as usize * TEXT_COLUMNS + self.column as usize;
        *text_buffer.add(offset) = c;
        *inv_buffer.add(offset) = inversed;
        if c >= 128 && i < str.len() - 1 {
          let offset = offset + 1;
          *text_buffer.add(offset) = str[i + 1];
          *inv_buffer.add(offset) = inversed;
          self.column += 2;
          i += 2;
        } else {
          self.column += 1;
          i += 1;
        }
      }
      if self.column as usize == TEXT_COLUMNS {
        self.newline();
      }
    }

    let mut i = self.row as usize * TEXT_COLUMNS + self.column as usize;
    unsafe {
      while i < 100 {
        *text_buffer.add(i) = 0;
        i += 1;
        if *text_buffer.add(i) == 0 {
          break;
        }
      }
    }
  }

  fn newline(&mut self) {
    if self.column == 0 {
      return;
    }
    if self.row as usize == TEXT_ROWS - 1 {
      self.scroll_text();
    } else {
      self.row += 1;
    }
    self.column = 0;
  }

  fn flush(&mut self) {
    if self.screen_mode == ScreenMode::Text {
      let graph_addr = self.props.graphics_base_addr as usize;
      self.memory[graph_addr..graph_addr + screen::BYTES].fill(0);
    }

    let mut char_ptr = unsafe {
      self
        .memory
        .as_ptr()
        .add(self.props.text_buffer_base_addr as usize)
    };
    let mut inv_ptr = self.inverse_text.as_ptr();
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
        let inv_mask = if unsafe { *inv_ptr } { 0xff } else { 0 };
        if c == 0 {
          char_ptr = unsafe { char_ptr.add(1) };
          inv_ptr = unsafe { inv_ptr.add(1) };
          col += 1;
          continue;
        }

        if c < 128 {
          let mut g = unsafe { graph.add(col) };
          let mut ascii_ptr =
            unsafe { ASCII_16_DATA.as_ptr().add(c as usize * CHAR_HEIGHT) };
          for _ in 0..CHAR_HEIGHT {
            unsafe {
              *g = *ascii_ptr ^ inv_mask;
              g = g.add(screen::WIDTH_IN_BYTE);
              ascii_ptr = ascii_ptr.add(1);
            }
          }
          char_ptr = unsafe { char_ptr.add(1) };
          inv_ptr = unsafe { inv_ptr.add(1) };
          col += 1;
          continue;
        }

        if row == TEXT_ROWS - 1 && col == TEXT_COLUMNS - 1 {
          self.paint_hex_code(row, col, c);
          char_ptr = unsafe { char_ptr.add(1) };
          inv_ptr = unsafe { inv_ptr.add(1) };
          col += 1;
          continue;
        }

        let c2 = unsafe { *char_ptr.add(1) };
        let inv_mask2 = if unsafe { *inv_ptr.add(1) } { 0xff } else { 0 };

        let mut data_ptr;
        if let Some(emoji_index) = self
          .props
          .emoji_version
          .code_to_index((c as u16) << 8 | c2 as u16)
        {
          data_ptr = unsafe {
            EMOJI_16_DATA.as_ptr().add(emoji_index * 2 * CHAR_HEIGHT)
          };
        } else if c >= 161 && c < 248 && c2 >= 161 && c2 < 255 {
          let mut sec = c as usize - 161;
          if sec > 8 {
            sec -= 6;
          }
          let gb_offset = (sec * 94 + (c2 as usize - 161)) * 2 * CHAR_HEIGHT;
          // NOTE shouldn't happen
          // if gb_offset + 2 * CHAR_HEIGHT > GB2312_16_DATA.len() {
          //   unreachable!();
          // }

          data_ptr = unsafe { GB2312_16_DATA.as_ptr().add(gb_offset) };
        } else if let Some(&offset) =
          self.props.extra_symbols.get((c as u64) << 8 | c2 as u64)
        {
          data_ptr =
            unsafe { self.props.extra_symbol_data.as_ptr().add(offset) };
        } else {
          self.paint_hex_code(row, col, c);
          self.paint_hex_code(row, col + 1, c2);
          char_ptr = unsafe { char_ptr.add(2) };
          inv_ptr = unsafe { inv_ptr.add(2) };
          col += 2;
          continue;
        }

        let mut g = unsafe { graph.add(col) };
        if col == TEXT_COLUMNS - 1 {
          // 汉字位于行尾时分成两半显示...
          for _ in 0..CHAR_HEIGHT {
            unsafe {
              *g = *data_ptr ^ inv_mask;
              *g.add(screen::WIDTH_IN_BYTE * (CHAR_HEIGHT - 1) + 1) =
                *data_ptr.add(1) ^ inv_mask2;
              g = g.add(screen::WIDTH_IN_BYTE);
              data_ptr = data_ptr.add(2);
            }
          }
        } else {
          for _ in 0..CHAR_HEIGHT {
            unsafe {
              *g = *data_ptr ^ inv_mask;
              *g.add(1) = *data_ptr.add(1) ^ inv_mask2;
              g = g.add(screen::WIDTH_IN_BYTE);
              data_ptr = data_ptr.add(2);
            }
          }
        }

        char_ptr = unsafe { char_ptr.add(2) };
        inv_ptr = unsafe { inv_ptr.add(2) };
        col += 2;
      }
      row += 1;
      graph = unsafe { graph.add(screen::WIDTH_IN_BYTE * CHAR_HEIGHT) };
    }

    // TODO finer grained dirty area
    self.update_dirty_area(0, 0, screen::WIDTH, screen::HEIGHT);
  }

  fn check_point(&self, (x, y): (i32, i32)) -> bool {
    if x < 0 || x >= screen::WIDTH as i32 || y < 0 || y >= screen::HEIGHT as i32
    {
      return false;
    }

    let x = x as usize;
    let y = y as usize;
    let ptr = unsafe {
      self
        .memory
        .as_ptr()
        .add(self.props.graphics_base_addr as usize + y * 20 + (x >> 3))
    };
    unsafe { *ptr & POINT_BIT_MASK[x & 7] != 0 }
  }

  fn draw_point(&mut self, (x, y): (u8, u8), mode: DrawMode) {
    if x < screen::WIDTH as u8 && y < screen::HEIGHT as u8 {
      unsafe {
        let g = self
          .memory
          .as_mut_ptr()
          .add(self.props.graphics_base_addr as usize);
        mode.point(g, x as usize, y as usize);
      }
      self.update_dirty_area(
        x as usize,
        y as usize,
        x as usize + 1,
        y as usize + 1,
      );
    }
  }

  fn draw_line(
    &mut self,
    (mut x1, mut y1): (u8, u8),
    (mut x2, mut y2): (u8, u8),
    mode: DrawMode,
  ) {
    if y1 == y2 {
      if x1 > x2 {
        let t = x1;
        x1 = x2;
        x2 = t;
      }
      self.draw_hor_line(x1, x2, y1, mode);
      self.update_dirty_area(
        x1 as usize,
        y1 as usize,
        x2 as usize + 1,
        y1 as usize + 1,
      );
      return;
    }

    if x1 == x2 {
      if y1 > y2 {
        let t = y1;
        y1 = y2;
        y2 = t;
      }
      self.draw_ver_line(x1, y1, y2, mode);
      self.update_dirty_area(
        x1 as usize,
        y1 as usize,
        x1 as usize + 1,
        y2 as usize + 1,
      );
      return;
    }

    if x1 > x2 {
      let t = x1;
      x1 = x2;
      x2 = t;
      let t = y1;
      y1 = y2;
      y2 = t;
    }

    let delta_x = x2 - x1;
    let delta_y = y2.abs_diff(y1);
    let inc_y = if y2 > y1 { 1 } else { u8::MAX };
    let dist = delta_x.max(delta_y);
    let mut error_x = 0;
    let mut error_y = 0;
    let mut x = x1;
    let mut y = y1;
    let g = unsafe {
      self
        .memory
        .as_mut_ptr()
        .add(self.props.graphics_base_addr as usize)
    };
    for _ in 0..=dist {
      unsafe { mode.point(g, x as usize, y as usize) };
      error_x += delta_x;
      error_y += delta_y;
      if error_x >= dist {
        error_x -= dist;
        x += 1;
      }
      if error_y >= dist {
        error_y -= dist;
        y = y.wrapping_add(inc_y);
      }
    }

    if y1 > y2 {
      let t = y1;
      y1 = y2;
      y2 = t;
    }
    self.update_dirty_area(
      x1 as usize,
      y1 as usize,
      x2 as usize + 1,
      y2 as usize + 1,
    );
  }

  fn draw_box(
    &mut self,
    (mut x1, mut y1): (u8, u8),
    (mut x2, mut y2): (u8, u8),
    fill: bool,
    mode: DrawMode,
  ) {
    if x1 > x2 {
      let t = x1;
      x1 = x2;
      x2 = t;
    }
    if y1 > y2 {
      let t = y1;
      y1 = y2;
      y2 = t;
    }
    if x1 >= screen::WIDTH as u8 || y1 >= screen::HEIGHT as u8 {
      return;
    }

    if fill {
      if x2 >= screen::WIDTH as u8 {
        x2 = screen::WIDTH as u8 - 1;
      }
      if y2 >= screen::HEIGHT as u8 {
        y2 = screen::HEIGHT as u8 - 1;
      }
      let ptr = unsafe {
        self
          .memory
          .as_mut_ptr()
          .add(self.props.graphics_base_addr as usize)
      };
      for y in y1..=y2 {
        unsafe {
          Self::draw_hor_line_unchecked(ptr, x1, x2, y, mode);
        }
      }
    } else {
      self.draw_hor_line(x1, x2, y1, mode);
      self.draw_hor_line(x1, x2, y2, mode);
      self.draw_ver_line(x1, y1, y2, mode);
      self.draw_ver_line(x2, y1, y2, mode);
    }

    self.update_dirty_area(
      x1 as usize,
      y1 as usize,
      x2 as usize + 1,
      y2 as usize + 1,
    );
  }

  fn draw_circle(
    &mut self,
    (x, y): (u8, u8),
    r: u8,
    fill: bool,
    mode: DrawMode,
  ) {
    self.draw_ellipse((x, y), (r, r), fill, mode);
  }

  fn draw_ellipse(
    &mut self,
    (x0, y0): (u8, u8),
    (rx, ry): (u8, u8),
    fill: bool,
    mode: DrawMode,
  ) {
    if rx == 0 && ry == 0 {
      self.draw_point((x0, y0), mode);
      return;
    }

    let dist_x = rx as i32;
    let dist_y = ry as i32;
    let r = dist_x.max(dist_y);
    let mut inc_x = -1;
    let mut inc_y = 1;
    let mut fy = 1i32;
    let mut fx = 1 - 2 * r;
    let mut fxy = 0;
    let mut delta_x = 0;
    let mut delta_y = 0;
    let mut tmp_x = rx as i32;
    let mut tmp_y = 0;
    let mut part_start = false;
    let g = unsafe {
      self
        .memory
        .as_mut_ptr()
        .add(self.props.graphics_base_addr as usize)
    };
    let x = x0 as i32;
    let y = y0 as i32;
    unsafe {
      Self::ellipse_part(g, x, y, tmp_x, tmp_y, fill, mode);
    }
    while tmp_x != 0 {
      if fxy >= 0 {
        delta_x += dist_x;
        if delta_x >= r {
          tmp_x += inc_x;
          delta_x -= r;
          if tmp_x + 1 != dist_x {
            unsafe {
              Self::ellipse_part(g, x, y, tmp_x, tmp_y, fill, mode);
            }
          }
        }
        fxy -= fx.abs();
        fx += 2;
        if fx < 0 || fx >= 3 {
          continue;
        }
        inc_y = -inc_y;
        fy = -fy + 2;
        fxy = -fxy;
      } else {
        delta_y += dist_y;
        if delta_y >= r {
          delta_y -= r;
          tmp_y += inc_y;
          unsafe {
            if !part_start && (tmp_y == 1 || tmp_y == 2) {
              Self::ellipse_part(g, x, y, dist_x, tmp_y, fill, mode);
            } else {
              part_start = true;
              Self::ellipse_part(g, x, y, tmp_x, tmp_y, fill, mode);
            }
          }
        }
        fxy += fy.abs();
        fy += 2;
        if fy < 0 || fy > 2 {
          continue;
        }
        inc_x = -inc_x;
        fx = -fx + 2;
        fxy = -fxy;
      }
    }

    self.update_dirty_area(
      x0.checked_sub(rx).unwrap_or(0) as usize,
      y0.checked_sub(ry).unwrap_or(0) as usize,
      (x0 as usize + rx as usize + 1).max(screen::WIDTH),
      (y0 as usize + ry as usize + 1).max(screen::HEIGHT),
    );
  }

  fn check_key(&self, key: u8) -> bool {
    if let Some((addr, mask)) = self.props.key_masks[key as usize] {
      self.memory[addr as usize] & mask == 0
    } else {
      false
    }
  }

  fn key(&mut self) -> Option<u8> {
    let addr = self.props.key_buffer_addr as usize;
    let key = self.memory[addr];
    if key < 128 {
      None
    } else {
      self.memory[addr] &= 0x7f;
      Some(key & 0x7f)
    }
  }

  fn read_byte(&self, addr: u16) -> u8 {
    if let Some(prop) = self.props.addrs.get(addr as _) {
      let now = Local::now();
      match prop {
        AddrProp::Year => (now.year() - 1881) as _,
        AddrProp::Month => now.month0() as _,
        AddrProp::Day => now.day0() as _,
        AddrProp::WeekDay => now.weekday().num_days_from_sunday() as _,
        AddrProp::Hour => now.hour() as _,
        AddrProp::Minute => now.minute() as _,
        AddrProp::HalfSecond => {
          ((now.second() as f64 + now.nanosecond() as f64 / 1e9) * 2.0) as _
        }
        AddrProp::SecondMult2 => (now.second() * 2) as _,
      }
    } else {
      self.memory[addr as usize]
    }
  }

  fn write_byte(&mut self, addr: u16, byte: u8) {
    if addr >= 0xe000 {
      return;
    }

    if addr < 256
      && self.key_mapping_addr_set[addr as usize >> 5] & (1 << (addr & 31)) != 0
    {
      return;
    }

    self.memory[addr as usize] = byte;

    let g = self.props.graphics_base_addr;
    if addr >= g && addr < g + screen::BYTES as u16 {
      let index = (addr - self.props.graphics_base_addr) as usize;
      let y = index / screen::WIDTH_IN_BYTE;
      let x = (index % screen::WIDTH_IN_BYTE) << 3;
      self.update_dirty_area(x, y, x + 8, y + 1);
    }
  }

  fn user_quit(&self) -> bool {
    let esc = self.memory[self.props.key_buffer_addr as usize]
      == 128 + KeyCode::Esc as u8;
    if self.props.key_buffer_quit {
      esc
    } else {
      let (addr, mask) = self.props.key_masks[27].unwrap();
      esc && self.memory[addr as usize] & mask == 0
    }
  }

  fn open_file(
    &mut self,
    file: &mut Self::File,
    name: &[u8],
    _read: bool,
    write: bool,
    truncate: bool,
  ) -> io::Result<()> {
    let mut options = OpenOptions::new();
    options
      .read(true)
      .write(write)
      .truncate(truncate)
      .create(write);
    let name = ByteString::from(name).to_string_lossy(self.props.emoji_version);
    let f = options.open(self.data_dir.join(name))?;
    file.open(f)
  }

  fn cls(&mut self) {
    let text_buffer_addr = self.props.text_buffer_base_addr as usize;
    self.memory[text_buffer_addr..text_buffer_addr + TEXT_BYTES].fill(0);
    let graph_addr = self.props.graphics_base_addr as usize;
    self.memory[graph_addr..graph_addr + screen::BYTES].fill(0);
    self.inverse_text.fill(false);
    self.row = 0;
    self.column = 0;
    self.update_dirty_area(0, 0, screen::WIDTH, screen::HEIGHT);
  }

  fn exec_asm(
    &mut self,
    steps: &mut usize,
    state: AsmExecState<MOS6502>,
  ) -> Option<MOS6502> {
    let mut sim = match state {
      AsmExecState::Start(addr) => {
        let mut sim = MOS6502::new();
        sim.set_program_counter(addr);
        sim
      }
      AsmExecState::Cont(sim) => sim,
    };
    while *steps > 0 {
      for _ in 0..50 {
        sim.execute_instruction(self);
        if sim.get_stack_pointer() > 0xfd {
          return None;
        }
        // brk
        if sim.get_status_register() & 0b00110000 == 0b00110000 {
          let sp = sim.get_stack_pointer() as usize;
          let code_addr_lo = self.memory[0x102 + sp];
          let code_addr_hi = self.memory[0x103 + sp];
          let code_addr =
            code_addr_lo as usize + ((code_addr_hi as usize) << 8);
          self.memory[0x102 + sp] = code_addr_lo.wrapping_add(2);
          if code_addr_lo >= 0xfe {
            self.memory[0x103 + sp] += 1;
          }
          let code = ((self.memory[code_addr] as u16) << 8)
            + self.memory[code_addr + 1] as u16;
          sim.set_program_counter(0xffff); // run RTI
          todo!("BRK ${:04X}", code)
        }
      }
      *steps -= 1;
    }
    Some(sim)
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

    self.blink_cursor();

    self.cursor = CursorState::None;
  }
}

impl Interface6502 for DefaultDevice {
  fn read(&mut self, address: u16) -> u8 {
    self.read_byte(address)
  }

  fn write(&mut self, address: u16, data: u8) {
    self.write_byte(address, data);
  }
}

impl DefaultFileHandle {
  fn open(&mut self, mut file: FsFile) -> io::Result<()> {
    let mut data = vec![];
    file.read_to_end(&mut data)?;
    if matches!(&self.state, FileState::Open { .. }) {
      Err(io::Error::new(io::ErrorKind::Other, "重复打开文件"))
    } else {
      self.state = FileState::Open {
        file,
        data,
        dirty: false,
      };
      self.pos = 0;
      Ok(())
    }
  }
}

impl Default for DefaultFileHandle {
  fn default() -> Self {
    Self {
      state: FileState::Closed { len: 0 },
      pos: 0,
    }
  }
}

impl FileHandle for DefaultFileHandle {
  fn len(&self) -> io::Result<u64> {
    match &self.state {
      FileState::Open { data, .. } => Ok(data.len() as _),
      FileState::Closed { len } => Ok(*len as _),
    }
  }

  fn seek(&mut self, pos: u64) -> io::Result<()> {
    match &self.state {
      FileState::Open { data, .. } => {
        if pos > data.len() as u64 {
          Err(io::Error::new(io::ErrorKind::Other, "文件指针超出文件大小"))
        } else {
          self.pos = pos as _;
          Ok(())
        }
      }
      FileState::Closed { .. } => {
        Err(io::Error::new(io::ErrorKind::Other, "未打开文件"))
      }
    }
  }

  fn pos(&self) -> io::Result<u64> {
    Ok(self.pos as _)
  }

  fn write(&mut self, written_data: &[u8]) -> io::Result<()> {
    match &mut self.state {
      FileState::Open { data, dirty, .. } => {
        let data_end = self.pos + written_data.len();
        let data_len = data.len();
        if data_end > data_len {
          if data_end > 65534 {
            return Err(io::Error::new(
              io::ErrorKind::FileTooLarge,
              format!("文件大小为 {} 字节，超出文件大小上限 65534", data_end),
            ));
          }
          data.resize(data_end, 0);
        }
        data[self.pos..data_end].copy_from_slice(written_data);
        self.pos = data_end;
        *dirty = true;
        Ok(())
      }
      FileState::Closed { .. } => {
        Err(io::Error::new(io::ErrorKind::Other, "未打开文件"))
      }
    }
  }

  fn read(&mut self, read_buf: &mut [u8]) -> io::Result<usize> {
    match &mut self.state {
      FileState::Open { data, .. } => {
        let mut len = read_buf.len();
        if self.pos + len > data.len() {
          len = data.len() - self.pos;
        }
        read_buf[..len].copy_from_slice(&data[self.pos..self.pos + len]);
        self.pos += len;
        Ok(len as _)
      }
      FileState::Closed { .. } => {
        Err(io::Error::new(io::ErrorKind::Other, "未打开文件"))
      }
    }
  }

  fn close(&mut self) -> io::Result<()> {
    match &mut self.state {
      FileState::Open { file, data, dirty } => {
        let len = data.len();
        if *dirty {
          file.seek(SeekFrom::Start(0))?;
          file.write_all(data)?;
        }
        self.state = FileState::Closed { len };
        Ok(())
      }
      FileState::Closed { .. } => Err(io::Error::new(
        io::ErrorKind::Other,
        "未打开文件，不能关闭文件",
      )),
    }
  }

  fn is_open(&self) -> bool {
    matches!(&self.state, FileState::Open { .. })
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::machine::EmojiVersion;
  use crate::vm::ByteString;
  use insta::assert_snapshot;
  use pretty_assertions::assert_eq;
  use std::sync::Once;

  static INIT: Once = Once::new();

  pub fn initialize() {
    INIT.call_once(|| {
      crate::machine::init_machines().unwrap();
    });
  }

  fn new_device() -> DefaultDevice {
    initialize();
    DefaultDevice::new(
      crate::machine::machines()[EmojiVersion::V2.default_machine_name()]
        .clone(),
      "",
    )
  }

  fn device_screen_braille(device: &DefaultDevice) -> String {
    let mem = device.graphic_memory();
    let mut braille = String::new();
    for y in (0..80).step_by(4) {
      for x in 0..20 {
        let r1 = mem[y * 20 + x].reverse_bits();
        let r2 = mem[(y + 1) * 20 + x].reverse_bits();
        let r3 = mem[(y + 2) * 20 + x].reverse_bits();
        let r4 = mem[(y + 3) * 20 + x].reverse_bits();
        for i in (0..8).step_by(2) {
          let r1 = r1 >> i & 3;
          let r2 = r2 >> i & 3;
          let r3 = r3 >> i & 3;
          let r4 = r4 >> i & 3;
          let index = (r1 * 5) & 0b1001
            | (r2 * 10) & 0b10010
            | (r3 * 20) & 0b100100
            | (r4 << 6);
          let c = 0x2800 + index as u32;
          braille.push(unsafe { char::from_u32_unchecked(c) });
        }
      }
      braille.push('\n');
    }
    braille
  }

  fn string(str: &str) -> ByteString {
    ByteString::from_str(str, EmojiVersion::V2, true).0
  }

  fn pad_text_buffer(mut s: ByteString) -> ByteString {
    while s.len() < 100 {
      s.push(0);
    }
    s
  }

  fn inverse_buffer(d: &DefaultDevice) -> String {
    let mut s = String::new();
    let mut i = 0;
    for &b in &d.inverse_text {
      if b {
        s.push('#');
      } else {
        s.push(' ');
      }
      i += 1;
      if i % 20 == 0 {
        s.push('\n');
      }
    }
    s
  }

  const EMPTY_INVERSE_BUFFER: &str = "                    
                    
                    
                    
                    
";

  #[test]
  fn newline_at_first_column() {
    let mut device = new_device();

    device.newline();
    device.newline();
    device.newline();

    assert_eq!(device.column, 0);
    assert_eq!(device.row, 0);
  }

  #[test]
  fn print_simple() {
    let mut device = new_device();

    let mut str = string("A哈1\u{e050}0 k!-");
    str.drop_0x1f();
    device.print(&str);
    device.flush();

    assert_eq!(device.column, 11);
    assert_eq!(device.row, 0);

    device.newline();

    assert_eq!(device.column, 0);
    assert_eq!(device.row, 1);

    let str = pad_text_buffer(str);
    assert_eq!(device.text_buffer(), str.as_slice());
    assert_eq!(&inverse_buffer(&device), EMPTY_INVERSE_BUFFER);

    assert_snapshot!(device_screen_braille(&device));
  }

  #[test]
  fn print_at_last_column() {
    let mut device = new_device();

    let mut str = string("哈");
    str.drop_0x1f();
    device.set_column(18);
    device.print(&str);

    device.set_column(19);
    device.print(b"%");

    let mut str = String::from_utf8(vec![0; 18]).unwrap();
    str.push('哈');
    str.extend(vec!['\0'; 19]);
    str.push('%');
    let mut str = string(&str);
    str.drop_0x1f();
    let str = pad_text_buffer(str);
    assert_eq!(device.text_buffer(), str.as_slice());
  }

  #[test]
  fn locate() {
    let mut device = new_device();

    device.set_row(2);
    device.set_column(8);
    device.print(b"Abc123_");

    device.set_row(3);
    device.set_column(13);
    device.print(b"^$\\\x1e\x06`~");

    assert_eq!(device.column, 0);
    assert_eq!(device.row, 4);

    device.newline();

    assert_eq!(device.column, 0);
    assert_eq!(device.row, 4);

    device.set_column(18);
    device.print(&string("|"));

    device.flush();

    let mut s = String::from_utf8(vec![0; 48]).unwrap();
    s.push_str("Abc123_");
    s.extend(vec!['\0'; 18]);
    s.push_str("^$\\\x1e\x06`~");
    s.extend(vec!['\0'; 18]);
    s.push_str("|");
    let str = pad_text_buffer(string(&s));
    assert_eq!(device.text_buffer(), str.as_slice());
    assert_eq!(&inverse_buffer(&device), EMPTY_INVERSE_BUFFER);

    assert_snapshot!(device_screen_braille(&device));
  }

  #[test]
  fn clear_following_text_in_text_mode() {
    let mut device = new_device();

    device.print(b"abcd");

    device.set_column(5);
    device.print(b"ABC");

    device.set_column(1);
    device.print(b"%");

    device.flush();

    let str = pad_text_buffer(string("a%\0\0\0ABC"));
    assert_eq!(device.text_buffer(), str.as_slice());
    assert_eq!(&inverse_buffer(&device), EMPTY_INVERSE_BUFFER);

    assert_snapshot!(device_screen_braille(&device));
  }

  #[test]
  fn inverse_mode() {
    let mut device = new_device();
    device.set_screen_mode(ScreenMode::Graph);

    let mut str = string("A哈 1");
    str.drop_0x1f();
    device.print(&str);
    device.set_print_mode(PrintMode::Inverse);
    let mut str = string(" \u{e050}-3");
    str.drop_0x1f();
    device.print(&str);
    device.newline();
    device.print(b";");
    device.set_print_mode(PrintMode::Flash);
    device.print(b"=");

    let mut str = string("A哈 1 \u{e050}-3\0\0\0\0\0\0\0\0\0\0;=");
    str.drop_0x1f();
    let str = pad_text_buffer(str);
    assert_eq!(device.text_buffer(), str.as_slice());
    assert_eq!(
      &inverse_buffer(&device),
      "     #####          
#                   
                    
                    
                    
"
    );

    device.flush();
    assert_snapshot!(device_screen_braille(&device));

    device.set_column(0);
    device.print(b"?");

    let mut str = string("A哈 1 \u{e050}-3\0\0\0\0\0\0\0\0\0\0?");
    str.drop_0x1f();
    let str = pad_text_buffer(str);
    assert_eq!(device.text_buffer(), str.as_slice());
    assert_eq!(
      &inverse_buffer(&device),
      "     #####          
                    
                    
                    
                    
"
    );

    device.flush();

    assert_snapshot!(device_screen_braille(&device));
  }

  #[test]
  fn print_chinese_character_at_last_column() {
    let mut device = new_device();

    device.set_print_mode(PrintMode::Flash);
    device.set_column(19);
    let mut str = string("集");
    str.drop_0x1f();
    device.print(&str);

    device.write_byte((704 + 39) as u16, 176);
    device.write_byte((704 + 40) as u16, 161);

    device.flush();

    let mut str = String::new();
    str.extend(vec!['\0'; 19]);
    str.push_str(" 集");
    str.extend(vec!['\0'; 17]);
    str.push_str("啊");
    let mut str = string(&str);
    str.drop_0x1f();
    let str = pad_text_buffer(str);
    assert_eq!(device.text_buffer(), str.as_slice());
    assert_eq!(
      &inverse_buffer(&device),
      "                   #
##                  
                    
                    
                    
"
    );

    assert_snapshot!(device_screen_braille(&device));
  }

  #[test]
  fn print_hex_code() {
    let mut device = new_device();

    device.print(b"\xf8\x5e\xc7\xff \xae");
    device.write_byte((704 + 99) as u16, 0xb0);

    device.flush();

    let mut str = vec![0xf8u8, 0x5e, 0xc7, 0xff, 0x20, 0xae];
    str.extend(vec![0u8; 93]);
    str.push(0xb0);
    let str = pad_text_buffer(ByteString::from(str));
    assert_eq!(device.text_buffer(), str.as_slice());
    assert_eq!(&inverse_buffer(&device), EMPTY_INVERSE_BUFFER);

    assert_snapshot!(device_screen_braille(&device));
  }

  #[test]
  fn cursor_half() {
    let mut device = new_device();
    device.set_row(3);
    device.set_column(18);
    device.blink_cursor();

    assert_snapshot!(device_screen_braille(&device));

    device.blink_cursor();
    assert_eq!(device.graphic_memory(), &[0; 1600]);
  }

  #[test]
  fn cursor_full() {
    let mut device = new_device();

    device.print(b"   \xb0\xa1");
    device.set_column(3);
    device.flush();
    device.blink_cursor();

    assert_snapshot!(device_screen_braille(&device));

    device.blink_cursor();

    assert_snapshot!(device_screen_braille(&device));
  }

  #[test]
  fn cursor_full_at_latter_half_of_chinese_character() {
    let mut device = new_device();

    device.print(b"   \xb0\xa1");
    device.set_column(4);
    device.flush();
    device.blink_cursor();

    assert_snapshot!(device_screen_braille(&device));

    device.blink_cursor();

    assert_snapshot!(device_screen_braille(&device));
  }

  #[test]
  fn scroll_text_in_text_mode() {
    let mut device = new_device();

    device.print(b" A");
    device.newline();
    device.set_print_mode(PrintMode::Inverse);
    device.print(b"bc ");
    device.set_print_mode(PrintMode::Normal);
    device.print(b"de");
    device.newline();
    device.print(b"123");
    device.flush();

    device.set_column(0);
    device.print(b"");

    device.draw_line((0, 0), (159, 79), DrawMode::Copy);

    device.flush();
    device.set_row(4);
    device.set_column(18);
    device.print(b"678");

    device.flush();

    let mut str = string("bc de");
    str.extend(vec![0; 73]);
    str.extend(b"678");
    let str = pad_text_buffer(str);
    assert_eq!(device.text_buffer(), str.as_slice());
    assert_eq!(
      &inverse_buffer(&device),
      "###                 
                    
                    
                    
                    
"
    );

    assert_snapshot!(device_screen_braille(&device));
  }

  #[test]
  fn scroll_text_in_graph_mode() {
    let mut device = new_device();
    device.set_screen_mode(ScreenMode::Graph);

    device.print(b" A");
    device.newline();
    device.set_print_mode(PrintMode::Inverse);
    device.print(b"bc ");
    device.set_print_mode(PrintMode::Normal);
    device.print(b"de");
    device.newline();
    device.print(b"123");
    device.flush();

    device.set_column(0);
    device.print(b"");

    device.draw_line((0, 0), (159, 79), DrawMode::Copy);
    device.draw_line((0, 40), (159, 40), DrawMode::Not);

    device.flush();
    device.set_row(4);
    device.set_column(18);
    device.print(b"678");

    device.flush();

    let mut str = string("bc de");
    str.extend(vec![0; 73]);
    str.extend(b"678");
    let str = pad_text_buffer(str);
    assert_eq!(device.text_buffer(), str.as_slice());
    assert_eq!(
      &inverse_buffer(&device),
      "###                 
                    
                    
                    
                    
"
    );

    assert_snapshot!(device_screen_braille(&device));
  }

  #[test]
  fn draw_line() {
    let mut device = new_device();

    device.draw_line((0, 0), (159, 0), DrawMode::Copy);
    device.draw_line((0, 79), (159, 79), DrawMode::Copy);
    device.draw_line((0, 0), (0, 79), DrawMode::Copy);
    device.draw_line((159, 0), (159, 79), DrawMode::Copy);

    device.draw_line((3, 2), (5, 2), DrawMode::Copy);
    device.draw_line((5, 4), (9, 4), DrawMode::Copy);
    device.draw_line((5, 6), (17, 6), DrawMode::Copy);

    device.draw_box((80, 40), (100, 60), true, DrawMode::Copy);

    device.draw_line((75, 41), (84, 41), DrawMode::Erase);
    device.draw_line((75, 43), (84, 43), DrawMode::Not);

    device.draw_line((99, 35), (99, 45), DrawMode::Erase);
    device.draw_line((97, 35), (97, 45), DrawMode::Not);

    device.draw_line((2, 77), (12, 67), DrawMode::Copy);

    device.draw_box((120, 40), (150, 60), true, DrawMode::Copy);

    device.draw_line((115, 41), (125, 42), DrawMode::Not);
    device.draw_line((121, 43), (131, 49), DrawMode::Erase);

    device.draw_line((165, 77), (152, 77), DrawMode::Not);
    device.draw_line((155, 75), (155, 85), DrawMode::Not);
    device.draw_line((80, 85), (85, 71), DrawMode::Not);

    assert_snapshot!(device_screen_braille(&device));
  }

  #[test]
  fn draw_box() {
    let mut device = new_device();

    device.draw_box((0, 0), (159, 79), false, DrawMode::Copy);

    device.draw_box((2, 2), (148, 77), true, DrawMode::Copy);

    device.draw_box((150, 2), (170, 20), true, DrawMode::Not);
    device.draw_box((147, 4), (157, 6), true, DrawMode::Erase);

    device.draw_box((3, 3), (20, 10), false, DrawMode::Not);
    device.draw_box((4, 12), (22, 18), false, DrawMode::Erase);

    assert_snapshot!(device_screen_braille(&device));
  }

  #[test]
  fn draw_circle_unfilled_copy() {
    let mut device = new_device();

    device.draw_circle((10, 10), 1, false, DrawMode::Copy);

    device.draw_circle((10, 20), 2, false, DrawMode::Copy);

    device.draw_circle((10, 40), 3, false, DrawMode::Copy);

    device.draw_circle((10, 50), 4, false, DrawMode::Copy);

    device.draw_circle((10, 65), 5, false, DrawMode::Copy);

    device.draw_circle((40, 10), 6, false, DrawMode::Copy);

    device.draw_circle((40, 50), 14, false, DrawMode::Copy);

    device.draw_circle((100, 50), 35, false, DrawMode::Copy);

    assert_snapshot!(device_screen_braille(&device));
  }

  #[test]
  fn draw_circle_unfilled_not() {
    let mut device = new_device();

    device.draw_box((0, 0), (159, 39), true, DrawMode::Copy);

    device.draw_circle((10, 10), 1, false, DrawMode::Not);

    device.draw_circle((10, 20), 2, false, DrawMode::Not);

    device.draw_circle((10, 40), 3, false, DrawMode::Not);

    device.draw_circle((10, 50), 4, false, DrawMode::Not);

    device.draw_circle((10, 65), 5, false, DrawMode::Not);

    device.draw_circle((40, 10), 6, false, DrawMode::Not);

    device.draw_circle((40, 50), 14, false, DrawMode::Not);

    device.draw_circle((100, 50), 35, false, DrawMode::Not);

    assert_snapshot!(device_screen_braille(&device));
  }

  #[test]
  fn draw_circle_filled_copy() {
    let mut device = new_device();

    device.draw_circle((10, 10), 1, true, DrawMode::Copy);

    device.draw_circle((10, 20), 2, true, DrawMode::Copy);

    device.draw_circle((10, 40), 3, true, DrawMode::Copy);

    device.draw_circle((10, 50), 4, true, DrawMode::Copy);

    device.draw_circle((10, 65), 5, true, DrawMode::Copy);

    device.draw_circle((40, 10), 6, true, DrawMode::Copy);

    device.draw_circle((40, 50), 14, true, DrawMode::Copy);

    device.draw_circle((100, 50), 35, true, DrawMode::Copy);

    assert_snapshot!(device_screen_braille(&device));
  }

  #[test]
  fn draw_circle_filled_not() {
    let mut device = new_device();

    device.draw_box((0, 0), (159, 39), true, DrawMode::Copy);

    device.draw_circle((10, 10), 1, true, DrawMode::Not);

    device.draw_circle((10, 20), 2, true, DrawMode::Not);

    device.draw_circle((10, 40), 3, true, DrawMode::Not);

    device.draw_circle((10, 50), 4, true, DrawMode::Not);

    device.draw_circle((10, 65), 5, true, DrawMode::Not);

    device.draw_circle((40, 10), 6, true, DrawMode::Not);

    device.draw_circle((40, 50), 14, true, DrawMode::Not);

    device.draw_circle((100, 50), 35, true, DrawMode::Not);

    assert_snapshot!(device_screen_braille(&device));
  }

  #[test]
  fn draw_ellipse() {
    let mut device = new_device();

    device.draw_ellipse((10, 10), (1, 3), true, DrawMode::Copy);

    device.draw_ellipse((10, 20), (3, 1), true, DrawMode::Not);

    device.draw_ellipse((10, 40), (7, 3), true, DrawMode::Copy);

    device.draw_ellipse((10, 50), (2, 6), true, DrawMode::Not);

    device.draw_ellipse((10, 65), (5, 2), false, DrawMode::Copy);

    device.draw_ellipse((40, 10), (6, 10), false, DrawMode::Not);

    device.draw_ellipse((40, 50), (14, 4), true, DrawMode::Copy);

    device.draw_ellipse((100, 50), (35, 34), true, DrawMode::Not);

    assert_snapshot!(device_screen_braille(&device));
  }

  #[test]
  fn key() {
    let mut device = new_device();

    assert_eq!(device.key(), None);
    assert_eq!(device.read_byte(196), 0b1111_1111);

    device.fire_key_down(20);

    assert_eq!(device.read_byte(196), 0b1111_0111);
    assert_eq!(device.key(), Some(20));
    assert_eq!(device.key(), None);

    device.fire_key_down(99);

    assert_eq!(device.read_byte(196), 0b1011_0111);
    assert_eq!(device.key(), Some(99));

    device.fire_key_up(20);

    assert_eq!(device.read_byte(196), 0b1011_1111);

    device.fire_key_up(99);

    assert_eq!(device.read_byte(196), 0b1111_1111);
  }

  #[test]
  fn check_key() {
    let mut device = new_device();

    assert_eq!(device.check_key(97), false);
    assert_eq!(device.check_key(20), false);

    device.fire_key_down(20);

    assert_eq!(device.check_key(97), false);
    assert_eq!(device.check_key(20), true);

    device.fire_key_down(97);

    assert_eq!(device.check_key(97), true);
    assert_eq!(device.check_key(20), true);

    device.fire_key_up(97);
    device.fire_key_up(20);

    assert_eq!(device.check_key(97), false);
    assert_eq!(device.check_key(20), false);
  }

  #[test]
  fn check_point() {
    let mut device = new_device();

    assert_eq!(device.check_point((20, 10)), false);
    assert_eq!(device.check_point((-20, -10)), false);
    assert_eq!(device.check_point((180, 10)), false);

    device.draw_point((20, 10), DrawMode::Copy);
    device.draw_point((180, 10), DrawMode::Copy);

    assert_eq!(device.check_point((20, 10)), true);
    assert_eq!(device.check_point((180, 10)), false);
  }
}
