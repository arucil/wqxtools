use gpui_kit::component::text::{TextView, TextViewState};
use super::*;

const PAGES: &[(&str, &str, &str)] = &[
  (
    "GVBASIC 模拟器",
    "gvbsim/index.html",
    include_str!("../../docs/gvbsim/index.html"),
  ),
  (
    "语义说明",
    "gvbsim/semantics.html",
    include_str!("../../docs/gvbsim/semantics.html"),
  ),
  (
    "语法扩展",
    "gvbsim/extension.html",
    include_str!("../../docs/gvbsim/extension.html"),
  ),
  (
    "按键值列表",
    "keycodes.html",
    include_str!("../../docs/keycodes.html"),
  ),
  (
    "项目说明",
    "index.html",
    include_str!("../../docs/index.html"),
  ),
];

fn resolve_link(page: usize, href: &str) -> Option<usize> {
  let base = Path::new(PAGES[page].1)
    .parent()
    .expect("bundled relative path");
  let mut parts = Vec::new();
  let path = base.join(href);
  for component in path.components() {
    match component {
      std::path::Component::ParentDir => {
        parts.pop();
      }
      std::path::Component::Normal(part) => {
        parts.push(part.to_string_lossy().into_owned())
      }
      std::path::Component::CurDir => {}
      _ => return None,
    }
  }
  let path = parts.join("/");
  PAGES.iter().position(|page| page.1 == path)
}

pub(super) struct HelpBrowser {
  page: usize,
  state: Entity<TextViewState>,
  pub focus: FocusHandle,
}

impl HelpBrowser {
  pub fn new(cx: &mut Context<Studio>) -> Self {
    Self {
      page: 0,
      state: cx.new(|cx| TextViewState::html(PAGES[0].2, cx).selectable(true)),
      focus: cx.focus_handle(),
    }
  }

  fn navigate(
    &mut self,
    page: usize,
    window: &mut Window,
    cx: &mut Context<Studio>,
  ) {
    self.page = page;
    self.state =
      cx.new(|cx| TextViewState::html(PAGES[page].2, cx).selectable(true));
    // The old TextView may own focus when a link is clicked. Its replacement
    // has a new focus handle, so return focus to the document dialog.
    window.focus(&self.focus, cx);
    cx.notify();
  }

  pub fn render(&self, studio: Entity<Studio>, p: Palette) -> impl IntoElement {
    let close = studio.clone();
    let keys = studio.clone();
    let link = studio.clone();
    let mut index = div()
      .w(px(150.))
      .flex_shrink_0()
      .flex()
      .flex_col()
      .gap(px(6.));
    for (page, (title, _, _)) in PAGES.iter().enumerate() {
      let view = studio.clone();
      index = index.child(
        Button::new(format!("help-page-{page}"))
          .ghost()
          .small()
          .label(*title)
          .when(self.page == page, |button| button.bg(rgb(p.key_hover)))
          .on_click(move |_, window, cx| {
            view.update(cx, |s, cx| s.help.navigate(page, window, cx))
          }),
      );
    }
    div()
      .id("help-modal")
      .key_context("WqxHelp")
      .track_focus(&self.focus)
      .absolute()
      .size_full()
      .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
      .on_mouse_up(MouseButton::Left, |_, _, cx| cx.stop_propagation())
      .on_action(move |_: &CloseHelpAction, window, cx| {
        keys.update(cx, |s, cx| {
          s.show_help = false;
          s.editor.update(cx, |editor, cx| editor.focus(window, cx));
          cx.notify();
        });
        cx.stop_propagation();
      })
      .on_key_down(|_, _, cx| {
        cx.stop_propagation();
      })
      .flex()
      .items_center()
      .justify_center()
      .bg(rgba(p.overlay))
      .child(
        div()
          .w(px(820.))
          .max_w_full()
          .h(px(580.))
          .max_h_full()
          .p(px(18.))
          .rounded_lg()
          .bg(rgb(p.surface))
          .border_1()
          .border_color(rgb(p.border))
          .flex()
          .flex_col()
          .gap(px(12.))
          .child(
            div()
              .flex()
              .justify_between()
              .items_center()
              .child(
                div()
                  .text_sm()
                  .font_weight(FontWeight::SEMIBOLD)
                  .child("GVBASIC 文档"),
              )
              .child(
                Button::new("close-help")
                  .ghost()
                  .xsmall()
                  .label("关闭")
                  .on_click(move |_, window, cx| {
                    cx.stop_propagation();
                    close.update(cx, |s, cx| {
                      s.show_help = false;
                      s.editor
                        .update(cx, |editor, cx| editor.focus(window, cx));
                      cx.notify();
                    });
                  }),
              ),
          )
          .child(
            div()
              .flex_1()
              .min_h_0()
              .flex()
              .gap(px(16.))
              .child(index)
              .child(
                div().flex_1().min_w_0().min_h_0().h_full().text_sm().child(
                  TextView::new(&self.state).scrollable(true).on_link_click(
                    move |href, _, window, cx| {
                      if let Some(page) =
                        resolve_link(link.read(cx).help.page, href)
                      {
                        link.update(cx, |s, cx| {
                          s.help.navigate(page, window, cx)
                        });
                      } else if href.starts_with("https://")
                        || href.starts_with("http://")
                      {
                        cx.open_url(href);
                      }
                    },
                  ),
                ),
              ),
          ),
      )
  }
}

#[cfg(test)]
mod tests {
  use super::{PAGES, resolve_link};
  #[test]
  fn bundled_help_links_resolve_without_external_files() {
    assert_eq!(resolve_link(0, "extension.html"), Some(2));
    assert_eq!(resolve_link(2, "../keycodes.html"), Some(3));
    assert_eq!(resolve_link(4, "gvbsim/index.html"), Some(0));
    assert_eq!(resolve_link(0, "missing.html"), None);
    assert!(PAGES.iter().all(|page| page.2.contains("<body>")));
  }
}
