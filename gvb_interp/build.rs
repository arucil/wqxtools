use std::collections::HashMap;
use std::env;
use std::error::Error;
use std::fs;
use std::fs::OpenOptions;
use std::io::prelude::*;
use std::path::Path;
use tinyjson::JsonValue;

fn main() -> Result<(), Box<dyn Error>> {
  build_gb2312_mapping()?;
  build_gvb_keyword_mapping()?;
  build_machine_props_map()?;

  Ok(())
}

fn build_gb2312_mapping() -> Result<(), Box<dyn Error>> {
  println!("cargo:rerun-if-changed=data/GB2312.TXT");

  let file = fs::read_to_string("data/GB2312.TXT")?;
  let mut mapping = vec![];

  for line in file.lines() {
    if line.starts_with('#') {
      continue;
    }

    let segments = line.split_whitespace().take(2).collect::<Vec<_>>();
    let gbcode = u16::from_str_radix(&segments[0][2..], 16)?;
    let unicode = u16::from_str_radix(&segments[1][2..], 16)?;

    mapping.push((gbcode + 0x8080, unicode));
  }

  let out_dir = env::var("OUT_DIR")?;

  let mut file = OpenOptions::new()
    .create(true)
    .write(true)
    .open(Path::new(&out_dir).join("gb2312.rs"))?;

  writeln!(&mut file, "use phf::phf_map;")?;
  writeln!(&mut file)?;
  writeln!(
    &mut file,
    "pub(crate) static GB2312_TO_UNICODE: ::phf::Map<u16, u16> = phf_map! {{"
  )?;
  for (gbcode, unicode) in &mapping {
    writeln!(&mut file, "  {}u16 => {}u16,", gbcode, unicode)?;
  }
  writeln!(&mut file, "}};")?;
  writeln!(
    &mut file,
    "pub(crate) static UNICODE_TO_GB2312: ::phf::Map<u16, u16> = phf_map! {{"
  )?;
  for (gbcode, unicode) in &mapping {
    writeln!(&mut file, "  {}u16 => {}u16,", unicode, gbcode)?;
  }
  writeln!(&mut file, "}};")?;

  Ok(())
}

fn build_gvb_keyword_mapping() -> Result<(), Box<dyn Error>> {
  println!("cargo:rerun-if-changed=data/keyword.txt");

  let file = fs::read_to_string("data/keyword.txt")?;

  let mut mapping: Vec<(u8, &str)> = vec![];
  let mut space = vec![];

  for line in file.lines() {
    let segments = line.split_whitespace().collect::<Vec<_>>();
    let byte = u8::from_str_radix(segments[0], 16)?;
    mapping.push((byte, segments[1]));
    if let Some(&"1") = segments.get(2) {
      space.push(byte);
    }
  }

  let out_dir = env::var("OUT_DIR")?;

  let mut file = OpenOptions::new()
    .create(true)
    .write(true)
    .open(Path::new(&out_dir).join("keyword.rs"))?;

  writeln!(&mut file, "use phf::{{phf_map, phf_set}};")?;
  writeln!(&mut file)?;
  writeln!(
    &mut file,
    "pub(crate) static BYTE_TO_KEYWORD: ::phf::Map<u8, &'static str> = phf_map! {{"
  )?;
  for (byte, str) in &mapping {
    writeln!(&mut file, "  {}u8 => \"{}\",", byte, str)?;
  }
  writeln!(&mut file, "}};")?;

  writeln!(
    &mut file,
    "pub(crate) static KEYWORD_TO_BYTE: ::phf::Map<&'static str, u8> = phf_map! {{"
  )?;
  for (byte, str) in &mapping {
    writeln!(&mut file, "  \"{}\" => {}u8,", str, byte)?;
  }
  writeln!(&mut file, "}};")?;

  writeln!(
    &mut file,
    "pub(crate) static KEYWORD_REQUIRES_SPACE: ::phf::Set<u8> = phf_set! {{"
  )?;
  for byte in space {
    writeln!(&mut file, " {}u8,", byte)?;
  }
  writeln!(&mut file, "}};")?;

  Ok(())
}

fn build_machine_props_map() -> Result<(), Box<dyn Error>> {
  println!("cargo:rerun-if-changed=data/machine_props.json");

  let file = fs::read_to_string("data/machine_props.json")?;

  let map = file.parse::<JsonValue>()?;
  let map = map.get::<HashMap<String, JsonValue>>().unwrap();

  let out_dir = env::var("OUT_DIR")?;

  let mut file = OpenOptions::new()
    .create(true)
    .write(true)
    .open(Path::new(&out_dir).join("machines.rs"))?;

  writeln!(&mut file, "use phf::phf_map;")?;
  writeln!(&mut file)?;
  writeln!(
    &mut file,
    "pub const DEFAULT_MACHINE: &'static str = \"{}\";",
    map["default"].get::<String>().unwrap().to_ascii_uppercase()
  )?;
  writeln!(
    &mut file,
    "pub static MACHINES: phf::Map<&'static str, MachineProps> = phf_map! {{"
  )?;

  for (name, props) in map {
    if name == "default" {
      continue;
    }

    let props = props.get::<HashMap<String, JsonValue>>().unwrap();
    writeln!(
      &mut file,
      "  \"{}\" => MachineProps {{",
      name.to_ascii_uppercase(),
    )?;
    writeln!(&mut file, "    name: {:?},", name.to_ascii_uppercase())?;

    let emoji_style = props["emoji_style"].get::<String>().unwrap();
    let emoji_style = if emoji_style == "new" { "New" } else { "Old" };
    writeln!(&mut file, "    emoji_style: EmojiStyle::{},", emoji_style)?;

    let graphics_base_addr =
      *props["graphics_base_addr"].get::<f64>().unwrap() as u32;
    writeln!(&mut file, "    graphics_base_addr: {},", graphics_base_addr)?;

    let sleep_unit = props["sleep_unit"].get::<String>().unwrap();
    if let Some((num, unit)) =
      sleep_unit.split_once(|c: char| c.is_ascii_alphabetic())
    {
      let num = num
        .parse::<f64>()
        .expect(&format!("invalid sleep_unit: {}", sleep_unit));
      let ns = match unit {
        "s" => num * 1e9,
        "ms" => num * 1e6,
        "ns" => num,
        _ => panic!("invalid sleep_unit: {}", sleep_unit),
      };
      let ns = ns as u64;
      writeln!(
        &mut file,
        "    sleep_unit: std::time::Duration::from_nanos({}),",
        ns
      )?;
    } else {
      panic!("invalid sleep_unit: {}", sleep_unit);
    };

    writeln!(&mut file, "  }},")?;
  }

  writeln!(&mut file, "}};")?;

  Ok(())
}
