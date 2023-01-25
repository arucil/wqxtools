use bin_dasm::DasmOptions;
use clap::{crate_version, Arg, Command};
use std::error::Error;
use std::fs::File;
use std::io::{BufReader, BufWriter, Read};
use std::num::IntErrorKind;
use std::path::{Path, PathBuf};

fn main() -> Result<(), Box<dyn Error>> {
  let matches = Command::new("dasm")
    .version(crate_version!())
    .about("Disassemble 6502")
    .arg(
      Arg::new("bin")
        .short('b')
        .long("bin")
        .help("Disassemble .BIN file"),
    )
    .arg(
      Arg::new("origin")
        .short('g')
        .long("origin")
        .help(
          "the starting address of the .BIN program, in hexadecimal notation",
        )
        .value_parser(parse_hex)
    )
    .arg(
      Arg::new("output")
        .short('o')
        .long("output")
        .value_name("OUTPUT")
        .help("file for dumping assembly"),
    )
    .arg(Arg::new("FILE").help("source .BIN file").required(true))
    .get_matches();

  let file = matches.get_one::<String>("FILE").unwrap();
  let origin = matches
    .get_one("origin")
    .map(|&o| u16::from_str_radix(o, 16).unwrap());
  let output = matches.get_one::<String>("output").map_or_else(
    || {
      let mut path = Path::new(file).file_stem().unwrap().to_owned();
      path.push(".txt");
      PathBuf::from(path)
    },
    PathBuf::from,
  );

  let mut bytes = vec![];
  BufReader::new(File::open(file)?).read_to_end(&mut bytes)?;
  let output = BufWriter::new(File::create(output)?);

  ::bin_dasm::disassemble(
    &bytes,
    output,
    DasmOptions {
      starting_address: origin,
      bin: matches.contains_id("bin"),
    },
  )?;

  Ok(())
}

fn parse_hex(s: &str) -> Result<u16, String> {
  u16::from_str_radix(s, 16).map_err(|err| {
    match err.kind() {
      IntErrorKind::InvalidDigit => {
        "origin must be a hexadecimal number".to_owned()
      }
      IntErrorKind::NegOverflow | IntErrorKind::PosOverflow => {
        "origin must be in the range of [0, 0xffff]".to_owned()
      }
      _ => err.to_string(),
    }
  })
}
