#![feature(int_error_matching)]

use clap::{App, Arg, crate_version};
use std::fs::File;
use std::io::{BufReader, BufWriter, Read};
use std::error::Error;
use std::num::IntErrorKind;
use std::path::{Path, PathBuf};
use bin_dasm::DasmOptions;

fn main() -> Result<(), Box<dyn Error>> {
  let matches = App::new("dasm")
    .version(crate_version!())
    .about("Disassemble .BIN file")
    .arg(Arg::with_name("origin")
      .short("g")
      .long("origin")
      .help("the starting address of the .BIN program, in hexadecimal notation")
      .takes_value(true)
      .validator(validate_hex))
    .arg(Arg::with_name("output")
      .short("o")
      .long("output")
      .value_name("OUTPUT")
      .help("file for dumping assembly")
      .takes_value(true))
    .arg(Arg::with_name("FILE")
      .help("source .BIN file")
      .required(true))
    .get_matches();

  let file = matches.value_of("FILE").unwrap();
  let origin = matches.value_of("origin").map(|o| o.parse().unwrap());
  let output = matches.value_of("output")
    .map_or_else(
      || {
        let mut path = Path::new(file).file_stem().unwrap().to_owned();
        path.push(".txt");
        PathBuf::from(path)
      },
      PathBuf::from);

  let mut bytes = vec![];
  BufReader::new(File::open(file)?).read_to_end(&mut bytes)?;
  let output = BufWriter::new(File::create(output)?);

  ::bin_dasm::disassemble(&bytes, output, DasmOptions {
    starting_address: origin,
  })?;

  Ok(())
}

fn validate_hex(s: String) -> Result<(), String> {
  match u16::from_str_radix(&s, 16) {
    Ok(_) => Ok(()),
    Err(err) => {
      match err.kind() {
        IntErrorKind::InvalidDigit => Err("origin must be a hexadecimal number".to_owned()),
        IntErrorKind::Underflow | IntErrorKind::Overflow => Err("origin must be in the range of [0, 0xffff]".to_owned()),
        _ => Err(err.to_string()),
      }
    }
  }
}