use std::collections::HashMap;
use std::fs::File;
use std::io::prelude::*;

fn main() {
  let mut in_file = File::open("data/gb2312_symbol_16.dat").unwrap();
  let mut data = vec![];
  in_file.read_to_end(&mut data).unwrap();

  let mut out_file = File::create("gb2312_16.dat").unwrap();

  out_file.write_all(&data).unwrap();

  let mut in_file = File::open("GB2312.TXT").unwrap();
  let mut s = String::new();
  in_file.read_to_string(&mut s).unwrap();
  let table: HashMap<_, _> = s
    .lines()
    .map(|line| {
      let line: Vec<_> = line
        .split('\t')
        .take(2)
        .map(|x| u32::from_str_radix(&x[2..], 16).unwrap())
        .collect();
      (line[0] + 0x8080, line[1])
    })
    .collect();

  let mut in_file = File::open("data/unicode1.1_16.dat").unwrap();
  let mut data = vec![];
  in_file.read_to_end(&mut data).unwrap();

  for byte1 in 0xb0..0xf8 {
    for byte2 in 0xa1..0xff {
      let glyph = if let Some(&cp) = table.get(&((byte1 << 8) | byte2)) {
        assert!((0x4e00..=0x9fa5).contains(&cp));
        let offset = ((cp - 0x4e00) * 32) as usize;
        &data[offset..offset + 32]
      } else {
        &[0; 32]
      };
      let _ = out_file.write(glyph).unwrap();
    }
  }
}
