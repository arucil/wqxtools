use std::{num::IntErrorKind, str::FromStr};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Label(pub u16);

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ParseLabelError {
  OutOfBound,
  NotALabel,
}

impl FromStr for Label {
  type Err = ParseLabelError;

  fn from_str(s: &str) -> Result<Self, Self::Err> {
    match s.parse::<u16>() {
      Ok(label) => {
        if label > 9999 {
          Err(ParseLabelError::OutOfBound)
        } else {
          Ok(Self(label))
        }
      }
      Err(err) => match err.kind() {
        IntErrorKind::PosOverflow => Err(ParseLabelError::OutOfBound),
        _ => Err(ParseLabelError::NotALabel),
      },
    }
  }
}
