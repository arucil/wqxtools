use std::ops::Range;

use regex::RegexBuilder;
use super::*;

#[derive(Clone, Copy, Default)]
pub(super) struct Options {
  case_sensitive: bool,
  whole_word: bool,
  regex: bool,
}

#[derive(Debug, PartialEq)]
struct Match {
  range: Range<usize>,
  replacement: String,
}

fn matches(
  source: &str,
  query: &str,
  replacement: &str,
  options: Options,
) -> Result<Vec<Match>, String> {
  if query.is_empty() {
    return Ok(Vec::new());
  }
  let pattern = if options.regex {
    query.to_owned()
  } else {
    regex::escape(query)
  };
  let pattern = if options.whole_word {
    format!(r"\b(?:{pattern})\b")
  } else {
    pattern
  };
  let regex = RegexBuilder::new(&pattern)
    .case_insensitive(!options.case_sensitive)
    .multi_line(true)
    .build()
    .map_err(|error| format!("正则表达式无效：{error}"))?;
  Ok(
    regex
      .captures_iter(source)
      .map(|captures| {
        let found = captures
          .get(0)
          .expect("a capture always contains the whole match");
        let mut expanded = String::new();
        if options.regex {
          captures.expand(replacement, &mut expanded);
        } else {
          expanded.push_str(replacement);
        }
        Match {
          range: found.range(),
          replacement: expanded,
        }
      })
      .collect(),
  )
}

fn replace_matches(source: &str, matches: &[Match]) -> String {
  let mut result = String::new();
  let mut end = 0;
  for found in matches {
    result.push_str(&source[end..found.range.start]);
    result.push_str(&found.replacement);
    end = found.range.end;
  }
  result.push_str(&source[end..]);
  result
}

pub(super) struct SearchPanel {
  pub visible: bool,
  replace: bool,
  query: Entity<InputState>,
  replacement: Entity<InputState>,
  options: Options,
  _subscriptions: Vec<Subscription>,
}

impl SearchPanel {
  pub fn new(window: &mut Window, cx: &mut Context<Studio>) -> Self {
    let query = cx.new(|cx| InputState::new(window, cx).placeholder("查找"));
    let replacement =
      cx.new(|cx| InputState::new(window, cx).placeholder("替换为"));
    let subscriptions = vec![
      cx.subscribe_in(
        &query,
        window,
        |studio, _, event: &InputEvent, window, cx| {
          if matches!(event, InputEvent::PressEnter { .. }) {
            studio.search_next(false, window, cx);
          }
          cx.notify();
        },
      ),
      cx.observe(&replacement, |_, _, cx| cx.notify()),
    ];
    Self {
      visible: false,
      replace: false,
      query,
      replacement,
      options: Options::default(),
      _subscriptions: subscriptions,
    }
  }

  pub fn open(
    &mut self,
    replace: bool,
    editor: &Entity<EditorState>,
    window: &mut Window,
    cx: &mut Context<Studio>,
  ) {
    self.visible = true;
    self.replace = replace;
    let selection = editor.read(cx).selected_value();
    if !selection.is_empty() && !selection.contains('\n') {
      self
        .query
        .update(cx, |input, cx| input.set_value(selection, window, cx));
    }
    self.query.update(cx, |input, cx| {
      input.focus(window, cx);
      input.select_all(window, cx);
    });
    cx.notify();
  }

  fn matches(&self, source: &str, cx: &App) -> Result<Vec<Match>, String> {
    matches(
      source,
      &self.query.read(cx).value(),
      &self.replacement.read(cx).value(),
      self.options,
    )
  }

  pub fn render(
    &self,
    studio: Entity<Studio>,
    p: Palette,
    source: &str,
    selection: Range<usize>,
    cx: &App,
  ) -> Div {
    let matches = self.matches(source, cx);
    let feedback = match &matches {
      Ok(matches) => {
        let index = matches.iter().position(|found| found.range == selection);
        format!("{} / {}", index.map_or(0, |i| i + 1), matches.len())
      }
      Err(error) => error.clone(),
    };
    let previous = studio.clone();
    let next = studio.clone();
    let close = studio.clone();
    let key = studio.clone();
    let mut controls = div()
      .flex()
      .items_center()
      .gap(px(4.))
      .child(
        div()
          .flex_1()
          .min_w_0()
          .child(Input::new(&self.query).small()),
      )
      .child(
        Button::new("search-prev")
          .ghost()
          .xsmall()
          .label("↑")
          .tooltip("上一个")
          .on_click(move |_, window, cx| {
            previous.update(cx, |s, cx| s.search_next(true, window, cx))
          }),
      )
      .child(
        Button::new("search-next")
          .ghost()
          .xsmall()
          .label("↓")
          .tooltip("下一个")
          .on_click(move |_, window, cx| {
            next.update(cx, |s, cx| s.search_next(false, window, cx))
          }),
      );
    for (id, label, enabled) in [
      ("case", "Aa", self.options.case_sensitive),
      ("word", "全词", self.options.whole_word),
      ("regex", ".*", self.options.regex),
    ] {
      let view = studio.clone();
      controls = controls.child(
        Button::new(id)
          .ghost()
          .xsmall()
          .label(label)
          .when(enabled, |button| button.bg(rgb(p.key_hover)))
          .on_click(move |_, _, cx| {
            view.update(cx, |s, cx| {
              match id {
                "case" => {
                  s.search.options.case_sensitive =
                    !s.search.options.case_sensitive
                }
                "word" => {
                  s.search.options.whole_word = !s.search.options.whole_word
                }
                "regex" => s.search.options.regex = !s.search.options.regex,
                _ => unreachable!("fixed search option"),
              }
              cx.notify();
            })
          }),
      );
    }
    controls = controls.child(
      Button::new("close-search")
        .ghost()
        .xsmall()
        .label("×")
        .tooltip("关闭 · Esc")
        .on_click(move |_, window, cx| {
          close.update(cx, |s, cx| s.close_search(window, cx))
        }),
    );
    let mut panel = div()
      .px(px(10.))
      .py(px(5.))
      .bg(rgb(p.panel))
      .border_b_1()
      .border_color(rgb(p.border))
      .flex()
      .flex_col()
      .gap(px(4.))
      .on_key_down(move |event, window, cx| {
        if event.keystroke.key == "escape" {
          key.update(cx, |s, cx| s.close_search(window, cx));
          cx.stop_propagation();
        }
      })
      .child(controls);
    if self.replace {
      let one = studio.clone();
      let all = studio;
      panel = panel.child(
        div()
          .flex()
          .items_center()
          .gap(px(6.))
          .child(
            div()
              .flex_1()
              .min_w_0()
              .child(Input::new(&self.replacement).small()),
          )
          .child(
            Button::new("replace-one")
              .ghost()
              .xsmall()
              .label("替换")
              .on_click(move |_, window, cx| {
                one.update(cx, |s, cx| s.search_replace(false, window, cx))
              }),
          )
          .child(
            Button::new("replace-all")
              .ghost()
              .xsmall()
              .label("全部替换")
              .on_click(move |_, window, cx| {
                all.update(cx, |s, cx| s.search_replace(true, window, cx))
              }),
          ),
      );
    }
    panel.child(
      div()
        .text_xs()
        .text_color(rgb(if matches.is_err() { p.error } else { p.muted }))
        .child(feedback),
    )
  }
}

impl Studio {
  fn close_search(&mut self, window: &mut Window, cx: &mut Context<Self>) {
    self.search.visible = false;
    self
      .editor
      .update(cx, |editor, cx| editor.focus(window, cx));
    cx.notify();
  }

  fn search_next(
    &mut self,
    backwards: bool,
    _: &mut Window,
    cx: &mut Context<Self>,
  ) {
    let Ok(matches) = self.search.matches(self.model.source(), cx) else {
      return;
    };
    let selected = self.editor.read(cx).selected_range();
    let exact = matches.iter().position(|found| found.range == selected);
    let index = if let Some(index) = exact {
      if backwards {
        (index + matches.len() - 1) % matches.len()
      } else {
        (index + 1) % matches.len()
      }
    } else if backwards {
      matches
        .iter()
        .rposition(|found| found.range.start < selected.start)
        .or_else(|| matches.len().checked_sub(1))
        .unwrap_or(0)
    } else {
      matches
        .iter()
        .position(|found| found.range.start >= selected.end)
        .unwrap_or(0)
    };
    if let Some(found) = matches.get(index) {
      self.editor.update(cx, |editor, cx| {
        editor.set_selected_range(found.range.clone(), cx)
      });
    }
    cx.notify();
  }

  fn search_replace(
    &mut self,
    all: bool,
    window: &mut Window,
    cx: &mut Context<Self>,
  ) {
    let Ok(matches) = self.search.matches(self.model.source(), cx) else {
      return;
    };
    if matches.is_empty() {
      return;
    }
    if all {
      let updated = replace_matches(self.model.source(), &matches);
      self
        .editor
        .update(cx, |editor, cx| editor.replace_all(updated, window, cx));
    } else {
      let selection = self.editor.read(cx).selected_range();
      let Some(found) = matches.iter().find(|found| found.range == selection)
      else {
        self.search_next(false, window, cx);
        return;
      };
      self.editor.update(cx, |editor, cx| {
        editor.replace(found.replacement.clone(), window, cx)
      });
    }
    cx.notify();
  }
}

#[cfg(test)]
mod tests {
  use super::{Options, matches, replace_matches};
  #[test]
  fn literal_case_words_and_unicode_byte_offsets() {
    let options = Options {
      whole_word: true,
      ..Options::default()
    };
    let found =
      matches("中文 PRINT print PRINTX", "print", "$1", options).unwrap();
    assert_eq!(
      found.iter().map(|m| m.range.clone()).collect::<Vec<_>>(),
      [7..12, 13..18]
    );
    assert_eq!(
      replace_matches("中文 PRINT print PRINTX", &found),
      "中文 $1 $1 PRINTX"
    );
    assert_eq!(
      matches(
        "print PRINT",
        "PRINT",
        "",
        Options {
          case_sensitive: true,
          ..options
        }
      )
      .unwrap()
      .len(),
      1
    );
  }
  #[test]
  fn regex_capture_and_zero_width_replacement() {
    let options = Options {
      regex: true,
      ..Options::default()
    };
    let source = "10 PRINT\n20 END";
    let found = matches(source, r"^(\d+)", "${1}0", options).unwrap();
    assert_eq!(replace_matches(source, &found), "100 PRINT\n200 END");
    let found = matches("中文", "^|$", "_", options).unwrap();
    assert_eq!(replace_matches("中文", &found), "_中文_");
    assert!(matches("", "[", "", options).is_err());
    assert!(matches("abc", "", "_", options).unwrap().is_empty());
  }
}
