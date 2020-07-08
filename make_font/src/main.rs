use std::io::prelude::*;
use std::fs::File;
use bdf::{Glyph, Bitmap, BoundingBox, Property};
use std::collections::HashMap;
use encoding::all::GBK;
use encoding::{Encoding, DecoderTrap};

fn main() {
  let mut font = bdf::read("\
STARTFONT 2.1
FONT wqx
SIZE 16 72 72
FONTBOUNDINGBOX 16 16 0 0
CHARS 0
ENDFONT
  ".as_bytes()).unwrap();

  font.properties_mut().insert("POINT_SIZE".into(), Property::Integer(160));
  font.properties_mut().insert("PIXEL_SIZE".into(), Property::Integer(16));

  let glyphs = font.glyphs_mut();

  make_ascii_glyphs(glyphs);
  make_gb2312_glyphs(glyphs);

  bdf::write(File::create("1.bdf").unwrap(), &font).unwrap();
}

fn make_ascii_glyphs(glyphs: &mut HashMap<char, Glyph>) {
  let mut data = vec![];
  let mut f = File::open("data/ascii8.dat").unwrap();
  f.read_to_end(&mut data).unwrap();

  for i in 0..128 {
    let mut glyph = Glyph::new(format!("U+{:04x}", i), i as u8 as char);
    let bitmap_data = &data[i * 16..i * 16 + 16];

    // calculate bounding box

    /*
    let mut y0 = 0;
    while y0 < 16 && bitmap_data[y0 as usize] == 0 {
      y0 += 1;
    }

    let mut y1;
    let mut x0;
    let mut x1;

    if y0 == 16 {
      y0 = 0;
      y1 = 0;
      x0 = 0;
      x1 = 0;
    } else {
      y1 = 15;
      while bitmap_data[y1 as usize] == 0 {
        y1 -= 1;
      }

      x0 = std::u32::MAX;
      for y in 0..16 {
        for x in 0..8 {
          if bitmap_data[y] & (1 << (7 - x)) != 0 {
            if x < x0 {
              x0 = x;
            }
            break;
          }
        }
      }

      x1 = 0;
      for y in 0..16 {
        for x in (0..8).rev() {
          if bitmap_data[y] & (1 << (7 - x)) != 0 {
            if x > x1 {
              x1 = x;
            }
            break;
          }
        }
      }
    }

    let width = x1 - x0 + 1;
    let height = y1 - y0 + 1;
    */

    let mut bitmap = Bitmap::new(8, 16);

    for y in 0..16 {
      for x in 0..8 {
        let bit = bitmap_data[y as usize] & 1 << (7 - x) != 0;
        bitmap.set(x, y, bit);
      }
    }
    glyph.set_map(bitmap);
    glyph.set_bounds(BoundingBox {
      x: 0,
      y: 0,
      width: 8,
      height: 16,
    });
    glyph.set_scalable_width(Some((500, 0)));
    glyph.set_device_width(Some((8, 0)));
    glyphs.insert(i as u8 as char, glyph);
  }
}

fn make_gb2312_glyphs(glyphs: &mut HashMap<char, Glyph>) {
  let mut data = vec![];
  let mut f = File::open("data/gb16.dat").unwrap();
  f.read_to_end(&mut data).unwrap();

  for i in 0..7614 {
    let mut byte1 = (i / 94 + 161) as u8;
    if byte1 > 160 + 9 {
      byte1 += 6;
    }
    let byte2 = (i % 94 + 161) as u8;

    let str = GBK.decode(&[byte1, byte2], DecoderTrap::Strict).unwrap();
    let cp = str.chars().next().unwrap();
    let mut glyph = Glyph::new(format!("U+{:04x}", cp as u32), cp);
    let bitmap_data = &data[i * 32..i * 32 + 32];

    let mut bitmap = Bitmap::new(16, 16);

    for y in 0..16 {
      for x in 0..16 {
        let bit = bitmap_data[y * 2 + x / 8] & 1 << (7 - x % 8) != 0;
        bitmap.set(x as u32, y as u32, bit);
      }
    }
    glyph.set_map(bitmap);
    glyph.set_bounds(BoundingBox {
      x: 0,
      y: 0,
      width: 16,
      height: 16,
    });
    glyph.set_scalable_width(Some((1000, 0)));
    glyph.set_device_width(Some((16, 0)));
    glyphs.insert(cp, glyph);
  }
}