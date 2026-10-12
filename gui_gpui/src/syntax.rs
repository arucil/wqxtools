use std::{ops::Range, rc::Rc};

use gpui_kit::{Context, HighlightStyle, SharedString, Window, rgb};
use gpui_kit::component::ThemeMode;
use gpui_kit::component::input::{
  EditorState, FoldRange, HighlightStyleResolver, InputEdit, InputHighlighter,
  InputHighlighterFactory, Rope,
};

const KEYWORDS: &[&str] = &[
  "AND", "BEEP", "BOX", "CALL", "CHECKKEY", "CIRCLE", "CLEAR", "CLOSE", "CLS",
  "COLOR", "DATA", "DEF", "DIM", "DRAW", "ELSE", "END", "FIELD", "FOR",
  "FPUTC", "FREAD", "FSEEK", "FWRITE", "GET", "GOSUB", "GOTO", "IF", "INPUT",
  "LET", "LINE", "LOAD", "LOCATE", "LSET", "NEXT", "NOT", "ON", "OPEN", "OR",
  "PAINT", "PLAY", "POINT", "POKE", "POP", "PRINT", "PUT", "READ", "REM",
  "RESTORE", "RETURN", "RSET", "RUN", "SCREEN", "SET", "SLEEP", "STOP", "SWAP",
  "THEN", "TO", "USING", "WEND", "WHILE", "WRITE", "XOR",
];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Token {
  Plain,
  Keyword,
  String,
  Number,
  Comment,
  Label,
}

pub fn factory(mode: ThemeMode) -> InputHighlighterFactory {
  Rc::new(move |language| {
    (language == "gvbasic").then(|| {
      Box::new(GvbHighlighter {
        mode,
        tokens: Vec::new(),
      }) as Box<dyn InputHighlighter>
    })
  })
}

struct GvbHighlighter {
  mode: ThemeMode,
  tokens: Vec<Token>,
}

impl GvbHighlighter {
  fn update_text(&mut self, source: &str) {
    self.tokens = lex(source);
  }
}

impl InputHighlighter for GvbHighlighter {
  fn language(&self) -> SharedString {
    "gvbasic".into()
  }

  fn update(
    &mut self,
    _: Option<InputEdit>,
    text: &Rope,
    _: bool,
    _: &mut Window,
    _: &mut Context<EditorState>,
  ) {
    self.update_text(&text.to_string());
  }

  fn styles(
    &self,
    range: &Range<usize>,
    _: &dyn HighlightStyleResolver,
  ) -> Vec<(Range<usize>, HighlightStyle)> {
    let mut result = Vec::new();
    let mut start = range.start;
    while start < range.end {
      let token = self.tokens.get(start).copied().unwrap_or(Token::Plain);
      let mut end = start + 1;
      while end < range.end
        && self.tokens.get(end).copied().unwrap_or(Token::Plain) == token
      {
        end += 1;
      }
      result.push((start..end, style(token, self.mode)));
      start = end;
    }
    result
  }

  fn fold_ranges(&self, _: &Rope) -> Vec<FoldRange> {
    Vec::new()
  }
}

fn style(token: Token, mode: ThemeMode) -> HighlightStyle {
  let color = match (mode, token) {
    (_, Token::Plain) => return HighlightStyle::default(),
    (ThemeMode::Dark, Token::Keyword) => 0x69d9b9,
    (ThemeMode::Dark, Token::String) => 0xe8be82,
    (ThemeMode::Dark, Token::Number) => 0xc6a6e7,
    (ThemeMode::Dark, Token::Comment) => 0x84979e,
    (ThemeMode::Dark, Token::Label) => 0x85c9e2,
    (ThemeMode::Light, Token::Keyword) => 0x007d69,
    (ThemeMode::Light, Token::String) => 0x965e13,
    (ThemeMode::Light, Token::Number) => 0x7c3fb0,
    (ThemeMode::Light, Token::Comment) => 0x657586,
    (ThemeMode::Light, Token::Label) => 0x1d6fa5,
  };
  HighlightStyle {
    color: Some(rgb(color).into()),
    ..Default::default()
  }
}

fn lex(source: &str) -> Vec<Token> {
  let mut tokens = vec![Token::Plain; source.len()];
  let mut line_start = 0;
  for line in source.split('\n') {
    let bytes = line.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
      if bytes[i] == b'"' {
        let start = i;
        i += 1;
        while i < bytes.len() {
          if bytes[i] == b'"' {
            i += 1;
            if i < bytes.len() && bytes[i] == b'"' {
              i += 1;
              continue;
            }
            break;
          }
          i += 1;
        }
        tokens[line_start + start..line_start + i].fill(Token::String);
        continue;
      }
      if bytes[i].is_ascii_digit() {
        let start = i;
        while i < bytes.len()
          && (bytes[i].is_ascii_alphanumeric() || bytes[i] == b'.')
        {
          i += 1;
        }
        let kind = if line[..start].trim().is_empty() {
          Token::Label
        } else {
          Token::Number
        };
        tokens[line_start + start..line_start + i].fill(kind);
        continue;
      }
      if bytes[i].is_ascii_alphabetic() {
        let start = i;
        while i < bytes.len()
          && (bytes[i].is_ascii_alphanumeric() || bytes[i] == b'_')
        {
          i += 1;
        }
        let word = &line[start..i];
        if KEYWORDS
          .iter()
          .any(|keyword| word.eq_ignore_ascii_case(keyword))
        {
          if word.eq_ignore_ascii_case("REM") {
            tokens[line_start + start..line_start + bytes.len()]
              .fill(Token::Comment);
            break;
          }
          tokens[line_start + start..line_start + i].fill(Token::Keyword);
        }
        continue;
      }
      i += 1;
    }
    line_start += line.len() + 1;
  }
  tokens
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn highlights_gvbasic_and_preserves_utf8_offsets() {
    let source = "10 PRINT \"你好\"\n20 REM 备注\n30 END";
    let tokens = lex(source);
    assert_eq!(tokens.len(), source.len());
    assert_eq!(tokens[0], Token::Label);
    assert_eq!(tokens[3], Token::Keyword);
    assert_eq!(tokens[10], Token::String);
    let rem = source.find("REM").expect("sample has REM");
    assert_eq!(tokens[rem], Token::Comment);
    assert_eq!(
      tokens[source.find("备注").expect("sample has Chinese comment")],
      Token::Comment
    );
  }

  #[test]
  fn syntax_colors_remain_distinct_in_both_themes() {
    for mode in [ThemeMode::Dark, ThemeMode::Light] {
      assert_ne!(
        style(Token::Keyword, mode).color,
        style(Token::String, mode).color
      );
      assert_ne!(
        style(Token::Label, mode).color,
        style(Token::Comment, mode).color
      );
    }
  }
}
