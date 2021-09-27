use crate::machine::EmojiStyle;
use std::fmt::Write;

include!(concat!(env!("OUT_DIR"), "/keyword.rs"));

pub struct BasTextDocument {
  pub base_addr: u16,
  pub guessed_emoji_style: EmojiStyle,
  pub text: String,
}

#[derive(Debug, Clone)]
pub struct LoadError<L> {
  pub location: L,
  pub message: String,
}

pub fn load_bas(
  content: impl AsRef<[u8]>,
  emoji_style: Option<EmojiStyle>,
) -> Result<BasTextDocument, LoadError<usize>> {
  let mut content = content.as_ref();
  let mut base_addr = 0;
  let mut lines: Vec<&[u8]> = vec![];
  let mut offset = 0;
  let mut guessed_emoji_styles = if let Some(emoji_style) = emoji_style {
    vec![emoji_style]
  } else {
    vec![EmojiStyle::New, EmojiStyle::Old]
  };

  loop {
    if content.len() < 3 {
      return Err(LoadError {
        location: offset + content.len(),
        message: format!("unexpected EOF"),
      });
    }

    if content[0] != 0 {
      return Err(LoadError {
        location: offset,
        message: format!("expected 0x00, found 0x{:02X}", content[0]),
      });
    }

    let addr = content[1] as u16 + ((content[2] as u16) << 8);
    if addr == 0 {
      break;
    }

    if base_addr == 0 {
      base_addr = addr;
    }

    let mut i = 5;

    while content.len() > i {
      if content[i] == 0 {
        lines.push(&content[..i]);
        offset += i;
        content = &content[i..];
        break;
      }

      if content[i] == 0x1f {
        if content.len() <= i + 2 {
          return Err(LoadError {
            location: offset + i,
            message: format!("invalid full-width character"),
          });
        }

        let gbcode = ((content[i + 1] as u16) << 8) + content[i + 2] as u16;
        if !crate::gb2312::GB2312_TO_UNICODE.contains_key(&gbcode) {
          guessed_emoji_styles.retain(|s| s.code_to_char(gbcode).is_some());
          if guessed_emoji_styles.is_empty() {
            return Err(LoadError {
              location: offset + i + 1,
              message: format!("unable to determine emoji style"),
            });
          }
        }
        i += 3;
      } else {
        if content[i] >= 0x80 && !BYTE_TO_KEYWORD.contains_key(&content[i]) {
          return Err(LoadError {
            location: offset + i,
            message: format!("unrecognized bytecode 0x{:02x}", content[i]),
          });
        }
        i += 1;
      }
    }
  }

  let guessed_emoji_style = guessed_emoji_styles[0];
  let mut text = String::new();
  let mut newline = false;

  for line in lines {
    if newline {
      text.push('\n');
    }
    newline = true;

    let label = line[3] as u16 + ((line[4] as u16) << 8);
    write!(&mut text, "{} ", label).unwrap();

    let mut last_is_keyword = false;
    let mut i = 5;
    while i < line.len() {
      let b = line[i];
      if b == 0x1f {
        let gbcode = ((line[i + 1] as u16) << 8) + line[i + 2] as u16;
        if let Some(&u) = crate::gb2312::GB2312_TO_UNICODE.get(&gbcode) {
          text.push(char::from_u32(u as u32).unwrap());
        } else {
          let u = guessed_emoji_style.code_to_char(gbcode).unwrap();
          text.push(char::from_u32(u as u32).unwrap());
        }
        last_is_keyword = false;
        i += 3;
      } else if b >= 0x80 {
        let kw = BYTE_TO_KEYWORD[&b];
        let last = *text.as_bytes().last().unwrap();
        let first = kw.as_bytes().first().unwrap();
        if first.is_ascii_alphabetic()
          && (last == b'$' || last.is_ascii_alphanumeric())
        {
          text.push(' ');
        } else if let "THEN" | "ELSE" | "TO" = kw {
          if last != b' ' {
            text.push(' ');
          }
        }
        text.push_str(kw);
        if KEYWORD_REQUIRES_SPACE.contains(&b) {
          text.push(' ');
        }
        last_is_keyword = true;
        i += 1;
      } else {
        if last_is_keyword && b.is_ascii_alphanumeric() {
          let last = *text.as_bytes().last().unwrap();
          if last == b'$'
            || last.is_ascii_alphanumeric() && (b as u8).is_ascii_alphanumeric()
          {
            text.push(' ');
          }
        }
        text.push(b as char);
        last_is_keyword = false;
        i += 1;
      }
    }
  }

  Ok(BasTextDocument {
    base_addr,
    guessed_emoji_style,
    text,
  })
}

pub fn load_txt(
  content: impl AsRef<[u8]>,
  emoji_style: Option<EmojiStyle>,
) -> Result<BasTextDocument, LoadError<(usize, usize)>> {
  let base_addr = 0x100;
  let mut guessed_emoji_styles = if let Some(emoji_style) = emoji_style {
    vec![emoji_style]
  } else {
    vec![EmojiStyle::New, EmojiStyle::Old]
  };
  let mut text = String::new();

  let mut i = 0;
  let mut line = 0;
  let mut line_offset = 0;
  let content = content.as_ref();
  while i < content.len() {
    if content[i] == 0xa {
      line += 1;
      line_offset = i + 1;
      text.push('\n');
      i += 1;
    } else if content[i] >= 0x80 {
      if content.len() <= i + 1 {
        return Err(LoadError {
          location: (line, i - line_offset),
          message: format!("invalid character"),
        });
      }

      let gbcode = ((content[i] as u16) << 8) + content[i + 1] as u16;
      if let Some(&u) = crate::gb2312::GB2312_TO_UNICODE.get(&gbcode) {
        text.push(char::from_u32(u as u32).unwrap());
      } else {
        guessed_emoji_styles.retain(|s| s.code_to_char(gbcode).is_some());
        if guessed_emoji_styles.is_empty() {
          return Err(LoadError {
            location: (line, i - line_offset),
            message: format!("unable to determine emoji style"),
          });
        } else {
          let u = guessed_emoji_styles[0].code_to_char(gbcode).unwrap();
          text.push(char::from_u32(u as u32).unwrap());
        }
      }

      i += 2;
    } else {
      text.push(content[i] as char);
      i += 1;
    }
  }

  Ok(BasTextDocument {
    base_addr,
    guessed_emoji_style: guessed_emoji_styles[0],
    text,
  })
}

#[cfg(test)]
mod tests {
  use super::*;
  use insta::assert_debug_snapshot;
  use std::fmt::{self, Debug, Formatter};

  impl Debug for BasTextDocument {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
      writeln!(f, "base_addr: 0x{:04x}", self.base_addr)?;
      writeln!(f, "guessed_emoji_style: {:?}", self.guessed_emoji_style)?;
      writeln!(f, "-------------------------------------")?;
      writeln!(f, "{}", self.text)
    }
  }

  #[test]
  fn test_load_bas() {
    let bytes = std::fs::read(
      std::env::current_dir()
        .unwrap()
        .join("test/fixtures/鹿逐中原.bas"),
    )
    .unwrap();

    let doc = load_bas(bytes, None).unwrap();

    assert_debug_snapshot!(doc);
  }

  #[test]
  fn test_load_txt() {
    let bytes = std::fs::read(
      std::env::current_dir()
        .unwrap()
        .join("test/fixtures/鹿逐中原.txt"),
    )
    .unwrap();

    let doc = load_txt(bytes, None).unwrap();

    assert_debug_snapshot!(doc);
  }
}
