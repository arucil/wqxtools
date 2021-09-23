#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EmojiStyle {
  Old,
  New,
}

impl EmojiStyle {
  pub fn code_to_char(&self, code: u16) -> Option<char> {
    let hi = code >> 8;
    let lo = code & 255;
    match self {
      Self::Old => {
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
        Some(unsafe { char::from_u32_unchecked(c as u32 + 0xe000) })
      }
      Self::New => {
        let c = match hi {
          248..=252 => match lo {
            161..=254 => (hi - 0xf8) * 94 + (lo - 161),
            _ => return None,
          },
          253 => match lo {
            161..=217 => 94 * 5 + lo - 161,
            _ => return None,
          },
          _ => return None,
        };
        Some(unsafe { char::from_u32_unchecked(c as u32 + 0xe000) })
      }
    }
  }

  pub fn char_to_code(&self, c: char) -> Option<u16> {
    let c = c as u32;
    if c < 0xe000 || c >= 0xe000 + 527 {
      return None;
    }

    let c = (c - 0xe000) as u16;
    match self {
      Self::Old => match c {
        0..57 => Some(0xfa46 + c),
        57..151 => Some(0xfaa1 + c - 57),
        151..214 => Some(0xfb40 + c - 151),
        214..308 => Some(0xfba1 + c - 214),
        308..371 => Some(0xfc40 + c - 308),
        371..465 => Some(0xfca1 + c - 371),
        465..527 => Some(0xfd40 + c - 465),
        _ => unreachable!(),
      },
      Self::New => Some(0xf8a1 + ((c / 94) << 8) + c % 94),
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
          + u8::arbitrary(g).checked_add(60).unwrap_or(255) as u16,
      )
    }
  }

  #[quickcheck]
  fn new_style_is_symmetric(Gb(code): Gb) -> bool {
    EmojiStyle::New.code_to_char(code).map_or(true, |c| {
      EmojiStyle::New
        .char_to_code(c)
        .filter(|&new_code| new_code == code)
        .is_some()
    })
  }

  #[quickcheck]
  fn old_style_is_symmetric(Gb(code): Gb) -> bool {
    EmojiStyle::Old.code_to_char(code).map_or(true, |c| {
      EmojiStyle::Old
        .char_to_code(c)
        .filter(|&new_code| new_code == code)
        .is_some()
    })
  }
}
