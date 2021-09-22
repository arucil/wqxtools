
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EmojiStyle {
  Old,
  New,
}

impl EmojiStyle {
  pub fn code_to_char(&self, code: u16) -> Option<char> {
    todo!()
  }
}