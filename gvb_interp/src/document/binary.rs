use crate::machine::EmojiStyle;
use std::fmt::Write;

include!(concat!(env!("OUT_DIR"), "/keyword.rs"));

pub const DEFAULT_BASE_ADDR: u16 = 0x7000;

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

#[derive(Debug, Clone)]
pub struct SaveError {
  pub line: usize,
  pub message: String,
  pub bas_specific: bool,
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

  let mut second_line_addr = 0;

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

    if second_line_addr == 0 {
      second_line_addr = addr;
    } else if base_addr == 0 {
      if let Some(b) = second_line_addr.checked_sub(offset as u16 + 1) {
        base_addr = b;
      } else {
        return Err(LoadError {
          location: 1,
          message: format!("address underflow"),
        });
      }
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
      text.push_str("\r\n");
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
          text.push(char::from_u32(u as _).unwrap());
        } else {
          let u = guessed_emoji_style.code_to_char(gbcode).unwrap();
          text.push(char::from_u32(u as _).unwrap());
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
  let base_addr = DEFAULT_BASE_ADDR;
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
        text.push(char::from_u32(u as _).unwrap());
      } else {
        guessed_emoji_styles.retain(|s| s.code_to_char(gbcode).is_some());
        if guessed_emoji_styles.is_empty() {
          return Err(LoadError {
            location: (line, i - line_offset),
            message: format!("unable to determine emoji style"),
          });
        } else {
          let u = guessed_emoji_styles[0].code_to_char(gbcode).unwrap();
          text.push(char::from_u32(u as _).unwrap());
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

pub fn save_bas(
  text: impl AsRef<str>,
  emoji_style: EmojiStyle,
  base_addr: u16,
) -> Result<Vec<u8>, SaveError> {
  let text = save_txt(text, emoji_style)?;
  let mut bytes = vec![0u8];

  let mut line_start_addr = base_addr + 1;
  let mut line = 0;
  let mut i = 0;
  while i < text.len() {
    if !text[i].is_ascii_digit() {
      return Err(SaveError {
        line,
        message: format!("缺少行号"),
        bas_specific: true,
      });
    }

    line += 1;

    let label_start = i;
    while i < text.len() && text[i].is_ascii_digit() {
      i += 1;
    }

    let label =
      match unsafe { std::str::from_utf8_unchecked(&text[label_start..i]) }
        .parse::<u16>()
      {
        Ok(label) if label <= 9999 => label,
        _ => {
          return Err(SaveError {
            line,
            message: format!("行号超出范围（0~9999）"),
            bas_specific: true,
          });
        }
      };

    let line_start = bytes.len();
    let mut skip_space = true;

    bytes.push(0);
    bytes.push(0);
    bytes.push(label as _);
    bytes.push((label >> 8) as _);

    'line_loop: while i < text.len() {
      let b = text[i];
      match b {
        128..=255 => {
          if i < text.len() - 1 && text[i + 1] >= 128 {
            bytes.push(0x1f);
            bytes.push(b);
            bytes.push(text[i + 1]);
            i += 2;
          } else {
            return Err(SaveError {
              line,
              message: format!("非法字符：U+{:04X}", b),
              bas_specific: true,
            });
          }
          skip_space = true;
        }
        b'a'..=b'z' | b'A'..=b'Z' => {
          let start = i;
          while i < text.len() && text[i].is_ascii_alphabetic() {
            i += 1;
          }
          if i < text.len() && text[i] == b'$' {
            i += 1;
          }
          if let Some(&b) = KEYWORD_TO_BYTE.get(
            &unsafe { std::str::from_utf8_unchecked(&text[start..i]) }
              .to_ascii_uppercase(),
          ) {
            bytes.push(b);
            skip_space = true;
          } else {
            bytes.extend(&text[start..i]);
            skip_space = false;
          }
        }
        b'\r' => {
          i += 1;
        }
        b'\n' => {
          i += 1;
          break 'line_loop;
        }
        b' ' => {
          if !skip_space {
            bytes.push(b' ');
            skip_space = true;
          }
          i += 1;
        }
        b'"' => {
          bytes.push(b'"');
          i += 1;
          while i < text.len() {
            let b = text[i];
            if let b'\r' | b'\n' | b'"' = b {
              break;
            }
            if b >= 128 {
              if i < text.len() - 1 && text[i + 1] >= 128 {
                bytes.push(0x1f);
                bytes.push(b);
                bytes.push(text[i + 1]);
                i += 2;
              } else {
                return Err(SaveError {
                  line,
                  message: format!("非法字符：U+{:04X}", b),
                  bas_specific: true,
                });
              }
            } else {
              bytes.push(b);
              i += 1;
            }
          }
          if i < text.len() && text[i] == b'"' {
            bytes.push(b'"');
            i += 1;
          }
          skip_space = true;
        }
        _ => {
          if let Some(&b) = KEYWORD_TO_BYTE
            .get(unsafe { std::str::from_utf8_unchecked(&text[i..i + 1]) })
          {
            bytes.push(b);
            i += 1;
            skip_space = true;
          } else {
            bytes.push(b);
            i += 1;
            skip_space = b":;,()".contains(&b);
          }
        }
      }
    }

    bytes.push(0);
    let line_len = bytes.len() - line_start;
    /*
    if line_len > 256 {
      return Err(SaveError {
        line,
        message: format!("该行经过转译(tokenization)后超过了256字节"),
        bas_specific: true,
      });
    }
    */
    if let Some(next_line_start_addr) =
      line_start_addr.checked_add(line_len as _)
    {
      bytes[line_start] = next_line_start_addr as _;
      bytes[line_start + 1] = (next_line_start_addr >> 8) as _;
      line_start_addr = next_line_start_addr;
    } else {
      return Err(SaveError {
        line,
        message: format!("转译(tokenization)后文件大小超过了64KB"),
        bas_specific: true,
      });
    }

    if i == text.len() {
      bytes.push(0);
      bytes.push(0);
    }
  }

  if bytes.len() > 65536 {
    return Err(SaveError {
      line,
      message: format!("转译(tokenization)后文件大小超过了64KB"),
      bas_specific: true,
    });
  }

  Ok(bytes)
}

pub fn save_txt(
  text: impl AsRef<str>,
  emoji_style: EmojiStyle,
) -> Result<Vec<u8>, SaveError> {
  let text = text.as_ref();
  let mut bytes = vec![];
  let mut line = 1;
  for c in text.chars() {
    if c == '\n' {
      line += 1;
    }
    if (c as u32) < 256 {
      bytes.push(c as u8);
    } else if (c as u32) < 65536 {
      if let Some(&gbcode) = crate::gb2312::UNICODE_TO_GB2312.get(&(c as u16)) {
        bytes.push((gbcode >> 8) as _);
        bytes.push(gbcode as _);
      } else if let Some(gbcode) = emoji_style.char_to_code(c) {
        bytes.push((gbcode >> 8) as _);
        bytes.push(gbcode as _);
      } else {
        return Err(SaveError {
          line,
          message: format!("非法字符：U+{:04X}", c as u32),
          bas_specific: false,
        });
      }
    } else {
      return Err(SaveError {
        line,
        message: format!("非法字符：U+{:06X}", c as u32),
        bas_specific: false,
      });
    }
  }

  Ok(bytes)
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
  fn test_save_bas() {
    let bytes = std::fs::read(
      std::env::current_dir()
        .unwrap()
        .join("test/fixtures/鹿逐中原.bas"),
    )
    .unwrap();

    let doc = load_bas(&bytes, None).unwrap();

    let saved =
      save_bas(doc.text, doc.guessed_emoji_style, doc.base_addr).unwrap();

    assert_eq!(bytes, saved);
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
