#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EmojiVersion {
  V1,
  V2,
}

impl EmojiVersion {
  pub fn code_to_index(&self, code: u16) -> Option<usize> {
    let hi = code >> 8;
    let lo = code & 255;
    match self {
      Self::V1 => {
        let c = match hi {
          0xfa => match lo {
            70..=126 => lo - 70,
            161..=254 => lo - 161 + 57,
            _ => return None,
          },
          0xfb => match lo {
            64..=126 => lo - 64 + 57 + 94,
            161..=254 => lo - 161 + 57 + 94 + 63,
            _ => return None,
          },
          0xfc => match lo {
            64..=126 => lo - 64 + 57 + 94 + 63 + 94,
            161..=254 => lo - 161 + 57 + 94 + 63 + 94 + 63,
            _ => return None,
          },
          0xfd => match lo {
            64..=125 => lo - 64 + 57 + 94 + 63 + 94 + 63 + 94,
            _ => return None,
          },
          _ => return None,
        };
        Some(c as _)
      }
      Self::V2 => {
        let c = match hi {
          0xf8..=0xfc => match lo {
            161..=254 => (hi - 0xf8) * 94 + (lo - 161),
            _ => return None,
          },
          0xfd => match lo {
            161..=217 => 94 * 5 + (lo - 161),
            _ => return None,
          },
          _ => return None,
        };
        Some(c as _)
      }
    }
  }

  pub fn code_to_char(&self, code: u16) -> Option<char> {
    self
      .code_to_index(code)
      .map(|i| unsafe { char::from_u32_unchecked(i as u32 + 0xe000) })
  }

  pub fn char_to_code(&self, c: char) -> Option<u16> {
    let c = c as u32;
    if c < 0xe000 || c >= 0xe000 + 527 {
      return None;
    }

    let c = (c - 0xe000) as u16;
    match self {
      Self::V1 => match c {
        0..57 => Some(0xfa46 + c),
        57..151 => Some(0xfaa1 + c - 57),
        151..214 => Some(0xfb40 + c - 151),
        214..308 => Some(0xfba1 + c - 214),
        308..371 => Some(0xfc40 + c - 308),
        371..465 => Some(0xfca1 + c - 371),
        465..527 => Some(0xfd40 + c - 465),
        _ => unreachable!(),
      },
      Self::V2 => Some(0xf8a1 + ((c / 94) << 8) + c % 94),
    }
  }

  pub fn fallback_code_to_char(code: u16) -> Option<char> {
    if code < 0xf800 {
      return None;
    }
    let c = code as u32 - 0xf800;
    return Some(unsafe { char::from_u32_unchecked(c + 0xe300) });
  }

  pub fn fallback_char_to_code(c: char) -> Option<u16> {
    let c = c as u32;
    if c < 0xe300 || c > 0xeaff {
      return None;
    }
    let c = c - 0xe300;
    Some(0xf800 + c as u16)
  }

  pub fn default_machine_name(&self) -> &'static str {
    unsafe {
      match self {
        Self::V2 => super::DEFAULT_MACHINE_FOR_EMOJI_VERSION_2.as_ref(),
        Self::V1 => super::DEFAULT_MACHINE_FOR_EMOJI_VERSION_1.as_ref(),
      }
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use quickcheck::{Arbitrary, Gen};
  use quickcheck_macros::quickcheck;

  #[derive(Debug, Clone)]
  struct Gb(u16);

  impl Arbitrary for Gb {
    fn arbitrary(g: &mut Gen) -> Gb {
      Gb(
        (g.choose(&[247, 248, 249, 250, 251, 252, 253, 254]).unwrap() << 8)
          + u8::arbitrary(g) as u16,
      )
    }
  }

  #[quickcheck]
  fn version_2_is_symmetric(Gb(code): Gb) -> bool {
    EmojiVersion::V2.code_to_char(code).map_or(true, |c| {
      EmojiVersion::V2
        .char_to_code(c)
        .filter(|&new_code| new_code == code)
        .is_some()
    })
  }

  #[quickcheck]
  fn version_1_is_symmetric(Gb(code): Gb) -> bool {
    EmojiVersion::V1.code_to_char(code).map_or(true, |c| {
      EmojiVersion::V1
        .char_to_code(c)
        .filter(|&new_code| new_code == code)
        .is_some()
    })
  }

  #[quickcheck]
  fn fallback_is_symmetric(Gb(code): Gb) -> bool {
    EmojiVersion::fallback_code_to_char(code).map_or(true, |c| {
      EmojiVersion::fallback_char_to_code(c)
        .filter(|&new_code| new_code == code)
        .is_some()
    })
  }
}
