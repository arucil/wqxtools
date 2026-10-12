use super::*;

pub(super) struct SimulatorDialog {
  studio: Entity<Studio>,
  _subscription: Subscription,
  _blur: Subscription,
  _activation: Subscription,
}

impl SimulatorDialog {
  pub(super) fn new(
    studio: Entity<Studio>,
    window: &mut Window,
    cx: &mut Context<Self>,
  ) -> Self {
    let subscription = cx.observe(&studio, |_, _, cx| cx.notify());
    let focus = studio.read(cx).simulator_focus.clone();
    let blur = cx.on_blur(&focus, window, |this, _, cx| {
      this
        .studio
        .update(cx, |studio, _| studio.release_simulator_keys());
    });
    let activation =
      cx.observe_window_activation(window, |this, window, cx| {
        if !window.is_window_active() {
          this
            .studio
            .update(cx, |studio, _| studio.release_simulator_keys());
        }
      });
    Self {
      studio,
      _subscription: subscription,
      _blur: blur,
      _activation: activation,
    }
  }
}

impl Studio {
  fn simulator_panel(
    &self,
    window: &mut Window,
    view: Entity<Studio>,
  ) -> impl IntoElement {
    let p = self.palette();
    let down_view = view.clone();
    let up_view = view.clone();
    let modifier_view = view.clone();
    let screen_focus = self.simulator_focus.clone();
    let screen_border = if self.simulator_focus.is_focused(window) {
      p.accent
    } else {
      p.border
    };
    let mut simulator_panel = div()
      .w(px(470.))
      .flex_1()
      .min_h_0()
      .flex()
      .flex_col()
      .gap(px(3.))
      .px(px(8.))
      .pb(px(8.))
      .bg(rgb(p.panel))
      .track_focus(&self.simulator_focus)
      .on_key_down(move |event, window, cx| {
        if event.keystroke.modifiers.platform || event.keystroke.modifiers.alt {
          return;
        }
        if let Some(code) = hardware_key(&event.keystroke.key) {
          down_view.update(cx, |view, cx| {
            view.simulator_key_down(
              format!("hardware-{}", event.keystroke.key.to_ascii_lowercase()),
              code,
              window,
              cx,
            )
          });
          cx.stop_propagation();
        }
      })
      .on_key_up(move |event, _, cx| {
        if hardware_key(&event.keystroke.key).is_some() {
          up_view.update(cx, |view, _| {
            view.simulator_key_up(&format!(
              "hardware-{}",
              event.keystroke.key.to_ascii_lowercase()
            ))
          });
          cx.stop_propagation();
        }
      })
      .on_modifiers_changed(move |event, window, cx| {
        modifier_view.update(cx, |view, cx| {
          for (source, pressed, code) in [
            ("control", event.modifiers.control, 25),
            ("shift", event.modifiers.shift, 26),
          ] {
            if pressed {
              view.simulator_key_down(source.to_owned(), code, window, cx);
            } else {
              view.simulator_key_up(source);
            }
          }
        });
      })
      .overflow_y_scrollbar()
      .child(
        div().w_full().flex().justify_center().pb(px(18.)).child(
          div()
            .id("simulator-screen")
            .border_1()
            .border_color(rgb(screen_border))
            .cursor_pointer()
            .on_click(move |_, window, cx| window.focus(&screen_focus, cx))
            .child(
              img(self.screen.clone())
                .w(px((runtime::SCREEN_WIDTH * runtime::SCREEN_SCALE) as f32))
                .h(px((runtime::SCREEN_HEIGHT * runtime::SCREEN_SCALE) as f32)),
            ),
        ),
      );
    for row in 0..KEYS.len() {
      simulator_panel = simulator_panel.child(self.key_row(row, &view));
    }
    simulator_panel
  }

  fn debugger_panel(&self, view: Entity<Studio>) -> Div {
    let p = self.palette();
    let mut rows = div().flex_1().min_h_0().overflow_y_scrollbar();
    if let Some(simulator) = &self.simulator {
      let bindings = simulator.bindings();
      if bindings.is_empty() {
        rows = rows.child(
          div()
            .p(px(12.))
            .text_xs()
            .text_color(rgb(p.muted))
            .child("当前程序尚无变量"),
        );
      }
      for binding in bindings {
        match binding {
          DebugBinding::Variable { name, value } => {
            let edit_view = view.clone();
            let edit_name = name.clone();
            let edit_value = value.clone();
            rows = rows.child(
              div()
                .h(px(31.))
                .flex()
                .items_center()
                .border_b_1()
                .border_color(rgb(p.border))
                .text_xs()
                .child(
                  div()
                    .w(px(100.))
                    .pl(px(8.))
                    .text_color(rgb(p.accent))
                    .child(name.clone()),
                )
                .child(
                  div().flex_1().min_w_0().child(
                    Button::new(format!("binding-{name}"))
                      .ghost()
                      .compact()
                      .xsmall()
                      .disabled(simulator.state != RunState::Paused)
                      .tooltip("暂停后可修改变量")
                      .label(value.display())
                      .on_click(move |_, window, cx| {
                        edit_view.update(cx, |view, cx| {
                          view.begin_debug_edit(
                            DebugTarget::Variable(edit_name.clone()),
                            edit_value.clone(),
                            window,
                            cx,
                          );
                        });
                      }),
                  ),
                ),
            );
          }
          DebugBinding::Array { name, dimensions } => {
            let inspect_view = view.clone();
            let inspect_name = name.clone();
            let inspect_dimensions = dimensions.clone();
            rows = rows.child(
              div()
                .h(px(31.))
                .flex()
                .items_center()
                .border_b_1()
                .border_color(rgb(p.border))
                .text_xs()
                .child(
                  div()
                    .w(px(100.))
                    .pl(px(8.))
                    .text_color(rgb(p.accent))
                    .child(name.clone()),
                )
                .child(
                  div().flex_1().min_w_0().child(
                    Button::new(format!("array-{name}"))
                      .ghost()
                      .compact()
                      .xsmall()
                      .label(format!("数组 {:?}", dimensions))
                      .on_click(move |_, _, cx| {
                        inspect_view.update(cx, |view, cx| {
                          view.inspect_array(
                            inspect_name.clone(),
                            inspect_dimensions.clone(),
                            cx,
                          );
                        });
                      }),
                  ),
                ),
            );
          }
        }
      }
    }
    div()
      .flex_1()
      .min_w_0()
      .h_full()
      .flex()
      .flex_col()
      .bg(rgb(p.bg))
      .border_l_1()
      .border_color(rgb(p.border))
      .child(
        div()
          .h(px(31.))
          .flex()
          .items_center()
          .bg(rgb(p.surface))
          .border_b_1()
          .border_color(rgb(p.border))
          .text_xs()
          .text_color(rgb(p.muted))
          .child(div().w(px(100.)).pl(px(8.)).child("变量名"))
          .child(div().flex_1().child("值")),
      )
      .child(rows)
  }

  fn simulator_overlays(&self, mut root: Div, view: Entity<Studio>) -> Div {
    let p = self.palette();
    if let Some((prompt, fields)) =
      self.simulator.as_ref().and_then(Simulator::input_fields)
    {
      let submit_view = view.clone();
      let cancel_view = view.clone();
      let key_view = view.clone();
      let mut inputs = div()
        .w(px(410.))
        .max_h_full()
        .p(px(22.))
        .bg(rgb(p.surface))
        .rounded_lg()
        .border_1()
        .border_color(rgb(p.border))
        .flex()
        .flex_col()
        .gap(px(12.))
        .child(
          div()
            .text_sm()
            .font_weight(FontWeight::SEMIBOLD)
            .child(prompt.unwrap_or_else(|| "程序等待输入".to_owned())),
        );
      let mut rows = div().flex().flex_col().gap(px(8.)).overflow_y_scrollbar();
      for (field, input) in fields.iter().zip(&self.input_fields) {
        rows = rows.child(
          div()
            .flex()
            .items_center()
            .gap(px(8.))
            .child(
              div()
                .w(px(80.))
                .text_xs()
                .text_color(rgb(p.muted))
                .child(input_label(field)),
            )
            .child(div().flex_1().child(Input::new(input).small())),
        );
      }
      inputs = inputs
        .child(rows)
        .when_some(self.input_error.clone(), |inputs, error| {
          inputs.child(div().text_xs().text_color(rgb(p.error)).child(error))
        })
        .child(
          div()
            .flex()
            .justify_end()
            .gap(px(8.))
            .child(
              Button::new("cancel-input")
                .ghost()
                .xsmall()
                .label("停止程序")
                .on_click(move |_, _, cx| {
                  cx.stop_propagation();
                  cancel_view
                    .update(cx, |view, cx| view.stop_simulator_action(cx));
                }),
            )
            .child(
              Button::new("submit-input")
                .primary()
                .xsmall()
                .label("提交输入")
                .on_click(move |_, window, cx| {
                  cx.stop_propagation();
                  submit_view
                    .update(cx, |view, cx| view.submit_input(window, cx));
                }),
            ),
        );
      root = root.child(
        div()
          .id("input-modal")
          .absolute()
          .size_full()
          .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
          .on_mouse_up(MouseButton::Left, |_, _, cx| cx.stop_propagation())
          .on_key_down(move |event, window, cx| {
            if event.keystroke.key == "enter" {
              key_view.update(cx, |view, cx| view.submit_input(window, cx));
            } else if event.keystroke.key == "escape" {
              key_view.update(cx, |view, cx| view.stop_simulator_action(cx));
            }
            cx.stop_propagation();
          })
          .flex()
          .items_center()
          .justify_center()
          .bg(rgba(p.overlay))
          .child(inputs),
      );
    }
    let p = self.palette();
    if let Some(inspector) = &self.array_inspector {
      let close_view = view.clone();
      let mut content = div()
        .w(px(620.))
        .max_h(px(650.))
        .p(px(22.))
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
            .items_center()
            .justify_between()
            .child(
              div()
                .text_sm()
                .font_weight(FontWeight::SEMIBOLD)
                .child(format!("数组 {}", inspector.name)),
            )
            .child(
              Button::new("close-array")
                .ghost()
                .xsmall()
                .label("关闭")
                .on_click(move |_, _, cx| {
                  cx.stop_propagation();
                  close_view.update(cx, |view, cx| {
                    view.array_inspector = None;
                    cx.notify();
                  });
                }),
            ),
        );
      let mut axes = div().flex().items_center().gap(px(8.));
      for axis in 0..inspector.dimensions.len() {
        let view = view.clone();
        axes = axes.child(
          Button::new(format!("axis-{axis}"))
            .secondary()
            .compact()
            .xsmall()
            .label(format!(
              "维度 {}{}",
              axis + 1,
              if inspector.axis == axis { " ✓" } else { "" }
            ))
            .on_click(move |_, _, cx| {
              view.update(cx, |view, cx| {
                if let Some(inspector) = view.array_inspector.as_mut() {
                  inspector.axis = axis;
                  inspector.page = 0;
                  cx.notify();
                }
              });
            }),
        );
      }
      content = content.child(axes);
      for (dimension, &bound) in inspector.dimensions.iter().enumerate() {
        if dimension == inspector.axis {
          continue;
        }
        let decrease_view = view.clone();
        let increase_view = view.clone();
        content = content.child(
          div()
            .flex()
            .items_center()
            .gap(px(9.))
            .child(
              div()
                .w(px(110.))
                .text_sm()
                .text_color(rgb(p.muted))
                .child(format!("维度 {} 索引", dimension + 1)),
            )
            .child(
              Button::new(format!("index-minus-{dimension}"))
                .ghost()
                .compact()
                .xsmall()
                .label("−")
                .on_click(move |_, _, cx| {
                  decrease_view.update(cx, |view, cx| {
                    if let Some(inspector) = view.array_inspector.as_mut() {
                      inspector.indices[dimension] =
                        inspector.indices[dimension].saturating_sub(1);
                      cx.notify();
                    }
                  });
                }),
            )
            .child(
              div()
                .text_sm()
                .child(format!("{} / {bound}", inspector.indices[dimension])),
            )
            .child(
              Button::new(format!("index-plus-{dimension}"))
                .ghost()
                .compact()
                .xsmall()
                .label("+")
                .on_click(move |_, _, cx| {
                  increase_view.update(cx, |view, cx| {
                    if let Some(inspector) = view.array_inspector.as_mut() {
                      inspector.indices[dimension] = inspector.indices
                        [dimension]
                        .saturating_add(1)
                        .min(bound);
                      cx.notify();
                    }
                  });
                }),
            ),
        );
      }
      if let Some(simulator) = &self.simulator {
        match simulator.array_values(
          &inspector.name,
          &inspector.indices,
          inspector.axis,
        ) {
          Ok(values) => {
            let page_count = values.len().div_ceil(32);
            let page = inspector.page.min(page_count.saturating_sub(1));
            let mut entries = div()
              .flex_1()
              .min_h(px(120.))
              .overflow_y_scrollbar()
              .flex()
              .flex_col()
              .gap(px(3.));
            for (offset, value) in
              values.iter().enumerate().skip(page * 32).take(32)
            {
              let name = inspector.name.clone();
              let mut indices = inspector.indices.clone();
              indices[inspector.axis] = offset as u16;
              let value = value.clone();
              let edit_value = value.clone();
              let edit_view = view.clone();
              entries = entries.child(
                div()
                  .flex()
                  .items_center()
                  .justify_between()
                  .px(px(8.))
                  .py(px(2.))
                  .rounded_sm()
                  .hover(|style| style.bg(rgb(p.panel)))
                  .child(
                    div()
                      .text_xs()
                      .text_color(rgb(p.muted))
                      .child(format!("[{offset}]")),
                  )
                  .child(
                    Button::new(format!("array-value-{offset}"))
                      .ghost()
                      .compact()
                      .xsmall()
                      .disabled(simulator.state != RunState::Paused)
                      .tooltip("暂停后可修改数组")
                      .label(value.display())
                      .on_click(move |_, window, cx| {
                        edit_view.update(cx, |view, cx| {
                          view.begin_debug_edit(
                            DebugTarget::ArrayCell {
                              name: name.clone(),
                              indices: indices.clone(),
                            },
                            edit_value.clone(),
                            window,
                            cx,
                          );
                        });
                      }),
                  ),
              );
            }
            content = content.child(entries);
            let prev_view = view.clone();
            let next_view = view.clone();
            content = content.child(
              div()
                .flex()
                .items_center()
                .justify_end()
                .gap(px(8.))
                .child(
                  Button::new("array-prev")
                    .ghost()
                    .compact()
                    .xsmall()
                    .label("上一页")
                    .on_click(move |_, _, cx| {
                      prev_view.update(cx, |view, cx| {
                        if let Some(inspector) = view.array_inspector.as_mut() {
                          inspector.page = inspector.page.saturating_sub(1);
                          cx.notify();
                        }
                      });
                    }),
                )
                .child(div().text_xs().text_color(rgb(p.muted)).child(format!(
                  "{} / {}",
                  page + 1,
                  page_count
                )))
                .child(
                  Button::new("array-next")
                    .ghost()
                    .compact()
                    .xsmall()
                    .label("下一页")
                    .on_click(move |_, _, cx| {
                      next_view.update(cx, |view, cx| {
                        if let Some(inspector) = view.array_inspector.as_mut() {
                          inspector.page = (inspector.page + 1)
                            .min(page_count.saturating_sub(1));
                          cx.notify();
                        }
                      });
                    }),
                ),
            );
          }
          Err(error) => {
            content = content.child(div().text_color(rgb(p.error)).child(error))
          }
        }
      }
      root = root.child(
        div()
          .id("array-modal")
          .absolute()
          .size_full()
          .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
          .on_mouse_up(MouseButton::Left, |_, _, cx| cx.stop_propagation())
          .flex()
          .items_center()
          .justify_center()
          .bg(rgba(p.overlay))
          .child(content),
      );
    }
    if let Some(edit) = &self.debug_edit {
      let title = match &edit.target {
        DebugTarget::Variable(name) => format!("修改变量 {name}"),
        DebugTarget::ArrayCell { name, indices } => {
          format!("修改数组 {name}{indices:?}")
        }
      };
      let cancel_view = view.clone();
      let apply_view = view.clone();
      root = root.child(
        div()
          .id("debug-edit-modal")
          .absolute()
          .size_full()
          .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
          .on_mouse_up(MouseButton::Left, |_, _, cx| cx.stop_propagation())
          .flex()
          .items_center()
          .justify_center()
          .bg(rgba(p.overlay))
          .child(
            div()
              .w(px(410.))
              .p(px(24.))
              .rounded_lg()
              .bg(rgb(p.surface))
              .border_1()
              .border_color(rgb(p.border))
              .flex()
              .flex_col()
              .gap(px(14.))
              .child(
                div()
                  .text_sm()
                  .font_weight(FontWeight::SEMIBOLD)
                  .child(title),
              )
              .child(Input::new(&edit.input).small())
              .when_some(edit.error.clone(), |container, error| {
                container
                  .child(div().text_sm().text_color(rgb(p.error)).child(error))
              })
              .child(div().text_xs().text_color(rgb(p.muted)).child(
                "仅在模拟器暂停时可以修改；字符串须能编码为文曲星字符集。",
              ))
              .child(
                div()
                  .flex()
                  .justify_end()
                  .gap(px(8.))
                  .child(
                    Button::new("cancel-debug-edit")
                      .ghost()
                      .xsmall()
                      .label("取消")
                      .on_click(move |_, _, cx| {
                        cx.stop_propagation();
                        cancel_view.update(cx, |view, cx| {
                          view.debug_edit = None;
                          cx.notify();
                        });
                      }),
                  )
                  .child(
                    Button::new("apply-debug-edit")
                      .primary()
                      .xsmall()
                      .label("应用")
                      .on_click(move |_, _, cx| {
                        apply_view
                          .update(cx, |view, cx| view.apply_debug_edit(cx));
                      }),
                  ),
              ),
          ),
      );
    }
    root
  }
}

impl Render for SimulatorDialog {
  fn render(
    &mut self,
    window: &mut Window,
    cx: &mut Context<Self>,
  ) -> impl IntoElement {
    let studio = self.studio.read(cx);
    let p = studio.palette();
    let state = studio.simulator.as_ref().map(|simulator| simulator.state);
    let error = studio.simulator.as_ref().and_then(Simulator::error_message);
    let message_color = if error.is_some() { p.error } else { p.muted };
    let message = error.or_else(|| studio.simulator_message.clone());
    window.set_window_title(&format!(
      "GVBASIC 模拟器 - {} [{}]",
      studio.filename(),
      state.map(RunState::label).unwrap_or("未启动")
    ));
    let panel = studio.simulator_panel(window, self.studio.clone());
    let controls = SimulatorControls::for_state(state);
    let primary_label = controls.primary.label();
    let primary_view = self.studio.clone();
    let stop_view = self.studio.clone();
    let theme_view = self.studio.clone();
    let toolbar = div()
      .h(px(75.))
      .flex()
      .items_center()
      .justify_center()
      .gap(px(24.))
      .bg(rgb(p.panel))
      .child(
        Button::new("sim-primary")
          .ghost()
          .compact()
          .child(legacy_icon(controls.primary.icon()))
          .tooltip(format!("{primary_label} · F5"))
          .accessibility_label(primary_label)
          .on_click(move |_, window, cx| {
            primary_view
              .update(cx, |view, cx| view.simulator_primary(window, cx));
          }),
      )
      .child(
        Button::new("sim-stop")
          .ghost()
          .compact()
          .disabled(!controls.stop_enabled)
          .child(
            legacy_icon(LegacyIcon::Stop).opacity(if controls.stop_enabled {
              1.0
            } else {
              0.3
            }),
          )
          .tooltip("停止 · F7")
          .accessibility_label("停止")
          .on_click(move |_, _, cx| {
            stop_view.update(cx, |view, cx| view.stop_simulator_action(cx));
          }),
      )
      .child(
        Button::new("sim-theme-toggle")
          .ghost()
          .compact()
          .icon(theme_icon(studio.theme_mode))
          .tooltip(theme_toggle_label(studio.theme_mode))
          .accessibility_label(theme_toggle_label(studio.theme_mode))
          .on_click(move |_, _, cx| {
            theme_view.update(cx, |view, cx| view.toggle_theme(cx));
          }),
      );
    let debugger = studio.debugger_panel(self.studio.clone());
    let root = div()
      .size_full()
      .relative()
      .key_context("WqxSimulator")
      .on_action(cx.listener(|view, _: &RunProgram, window, cx| {
        view
          .studio
          .update(cx, |studio, cx| studio.simulator_primary(window, cx));
      }))
      .on_action(cx.listener(|view, _: &StopProgram, _, cx| {
        view
          .studio
          .update(cx, |studio, cx| studio.stop_simulator_action(cx));
      }))
      .on_action(cx.listener(|view, _: &ToggleThemeAction, _, cx| {
        view.studio.update(cx, |studio, cx| studio.toggle_theme(cx));
      }))
      .bg(rgb(p.bg))
      .text_color(rgb(p.text))
      .flex()
      .flex_row()
      .child(
        div()
          .w(px(470.))
          .min_h_0()
          .flex()
          .flex_col()
          .child(toolbar)
          .when_some(message, |column, message| {
            column.child(
              div()
                .mx(px(8.))
                .mb(px(6.))
                .px(px(10.))
                .py(px(8.))
                .rounded_md()
                .border_1()
                .border_color(rgb(message_color))
                .bg(rgb(p.surface))
                .text_xs()
                .text_color(rgb(message_color))
                .child(message),
            )
          })
          .child(panel),
      )
      .child(debugger);
    studio.simulator_overlays(root, self.studio.clone())
  }
}
