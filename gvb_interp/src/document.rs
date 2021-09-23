use std::fs;
use std::io;
use std::path::Path;
use std::path::PathBuf;

use crate::machine::EmojiStyle;
use crate::machine::MachineProps;

mod binary;
mod gb2312 {
  include!(concat!(env!("OUT_DIR"), "/gb2312.rs"));
}

pub struct Document {
  path: PathBuf,
  base_addr: u16,
  emoji_style: EmojiStyle,
  machine_props: Option<MachineProps>,
}

#[derive(Debug)]
pub enum DocumentError {
  Io(io::Error),
  UnknownExt(Option<String>),
  LoadBas(binary::LoadError<usize>),
  LoadTxt(binary::LoadError<(usize, usize)>),
}

impl From<io::Error> for DocumentError {
  fn from(err: io::Error) -> Self {
    Self::Io(err)
  }
}

impl From<binary::LoadError<usize>> for DocumentError {
  fn from(err: binary::LoadError<usize>) -> Self {
    Self::LoadBas(err)
  }
}

impl From<binary::LoadError<(usize, usize)>> for DocumentError {
  fn from(err: binary::LoadError<(usize, usize)>) -> Self {
    Self::LoadTxt(err)
  }
}

impl Document {
  /// Load a `.BAS` or `.txt` file.
  pub fn load<P>(path: impl AsRef<Path>) -> Result<Self, DocumentError> {
    let path = path.as_ref();
    let ext = path.extension().map(|ext| ext.to_ascii_lowercase());
    let is_bas = if let Some(ext) = ext {
      match ext.to_str() {
        Some("bas") => true,
        Some("txt") => false,
        ext => {
          return Err(DocumentError::UnknownExt(ext.map(|ext| ext.to_owned())))
        }
      }
    } else {
      return Err(DocumentError::UnknownExt(None));
    };

    let data = fs::read(path)?;

    let mut doc = if is_bas {
      binary::load_bas(&data, None)?
    } else {
      binary::load_txt(&data, None)?
    };

    let mut emoji_style = doc.guessed_emoji_style;

    let machine_props = detect_machine_props(&doc.text)
      .and_then(|p| p.ok())
      .cloned();
    if let Some(props) = &machine_props {
      emoji_style = props.emoji_style;
      doc = if is_bas {
        binary::load_bas(&data, Some(emoji_style))?
      } else {
        binary::load_txt(&data, Some(emoji_style))?
      };
    }

    Ok(Document {
      path: path.to_owned(),
      base_addr: doc.base_addr,
      emoji_style,
      machine_props,
    })
  }
}

fn detect_machine_props(
  text: &str,
) -> Option<Result<&'static MachineProps, ()>> {
  let first_line = text.lines().next().unwrap();
  if let Some(start) = first_line.rfind('{') {
    let first_line = &first_line[start + 1..];
    if let Some(end) = first_line.find('}') {
      let name = first_line[..end].trim();
      if !name.is_empty() {
        match crate::machine::MACHINES.get(name) {
          Some(props) => return Some(Ok(props)),
          None => return Some(Err(())),
        }
      }
    }
  }
  None
}
