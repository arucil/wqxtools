use std::path::{Path, PathBuf};
use std::time::Instant;

use eframe::egui::{self, Color32, RichText, TextureHandle, TextureOptions};
use egui_code_editor::{CodeEditor, ColorTheme, Syntax};
use egui_extras::{Column, TableBuilder};
use gvb_interp::device::Device;
use gvb_interp::device::default::DefaultDevice;
use gvb_interp::{
  Binding, Document, Edit, EditKind, ExecInput, ExecResult, KeyboardInput,
  KeyboardInputType, ReplaceText, Severity, Value, VirtualMachine,
};
use widestring::{Utf16Str, Utf16String};

const EXEC_STEPS: usize = 4_000;
const SCREEN_WIDTH: usize = 160;
const SCREEN_HEIGHT: usize = 80;

mod theme {
  use eframe::egui::Color32;

  pub const BACKGROUND: Color32 = Color32::from_rgb(13, 19, 26);
  pub const PANEL: Color32 = Color32::from_rgb(19, 28, 37);
  pub const SURFACE: Color32 = Color32::from_rgb(24, 35, 45);
  pub const EDITOR: Color32 = Color32::from_rgb(16, 24, 32);
  pub const BORDER: Color32 = Color32::from_rgb(45, 62, 73);
  pub const TEXT: Color32 = Color32::from_rgb(229, 238, 239);
  pub const MUTED: Color32 = Color32::from_rgb(147, 166, 174);
  pub const ACCENT: Color32 = Color32::from_rgb(91, 217, 178);
  pub const ACCENT_DARK: Color32 = Color32::from_rgb(25, 78, 69);
  pub const WARNING: Color32 = Color32::from_rgb(237, 190, 103);
  pub const ERROR: Color32 = Color32::from_rgb(247, 127, 125);
}

const EDITOR_THEME: ColorTheme = ColorTheme {
  name: "WQX Studio",
  dark: true,
  bg: "101820",
  cursor: "5bd9b2",
  selection: "254d50",
  comments: "788d93",
  functions: "83c7df",
  keywords: "6cddba",
  literals: "e5bd80",
  numerics: "d5a5d7",
  punctuation: "b8c8ca",
  strs: "e6bd83",
  types: "8cbce2",
  special: "eebd73",
};

const KEYBOARD: [[(&str, u8); 10]; 5] = [
  [
    ("", 0),
    ("", 0),
    ("", 0),
    ("", 0),
    ("", 0),
    ("", 0),
    ("F1", 28),
    ("F2", 29),
    ("F3", 30),
    ("F4", 31),
  ],
  [
    ("Q", 113),
    ("W", 119),
    ("E", 101),
    ("R", 114),
    ("T/7", 116),
    ("Y/8", 121),
    ("U/9", 117),
    ("I", 105),
    ("O", 111),
    ("P", 112),
  ],
  [
    ("A", 97),
    ("S", 115),
    ("D", 100),
    ("F", 102),
    ("G/4", 103),
    ("H/5", 104),
    ("J/6", 106),
    ("K", 107),
    ("L", 108),
    ("输入", 13),
  ],
  [
    ("Z", 122),
    ("X", 120),
    ("C", 99),
    ("V", 118),
    ("B/1", 98),
    ("N/2", 110),
    ("M/3", 109),
    ("上翻", 19),
    ("↑", 20),
    ("下翻", 14),
  ],
  [
    ("求助", 25),
    ("中英", 26),
    ("输入法", 18),
    ("跳出", 27),
    ("符号/0", 48),
    (".", 46),
    ("空格", 32),
    ("←", 23),
    ("↓", 21),
    ("→", 22),
  ],
];

#[derive(Clone)]
struct UiDiagnostic {
  line: usize,
  column: usize,
  char_offset: usize,
  severity: Severity,
  message: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum RunState {
  Running,
  Paused,
  Waiting,
  Sleeping,
  Finished,
  Error,
}

impl RunState {
  fn label(self) -> &'static str {
    match self {
      Self::Running => "运行中",
      Self::Paused => "已暂停",
      Self::Waiting => "等待输入",
      Self::Sleeping => "休眠",
      Self::Finished => "已结束",
      Self::Error => "出错",
    }
  }
}

struct Simulator {
  vm: VirtualMachine<DefaultDevice>,
  state: RunState,
  result: ExecResult,
  wake_at: Option<Instant>,
  next_cursor_blink: Instant,
  texture: Option<TextureHandle>,
  input_values: Vec<String>,
  input_error: Option<String>,
  array_inspector: Option<ArrayInspector>,
}

struct ArrayInspector {
  name: String,
  dimensions: Vec<u16>,
  axis: usize,
  indices: Vec<u16>,
}

pub struct WqxToolsApp {
  document: Document,
  source: String,
  source_synced: String,
  path: Option<PathBuf>,
  dirty: bool,
  diagnostics: Vec<UiDiagnostic>,
  status: String,
  show_problems: bool,
  simulator: Option<Simulator>,
  syntax: Syntax,
  close_prompt: bool,
  show_find: bool,
  show_replace: bool,
  find_text: String,
  replace_text: String,
  match_case: bool,
  relabel_open: bool,
  relabel_start: u16,
  relabel_step: u16,
  help_open: bool,
  emoji_open: bool,
  select_range: Option<(usize, usize)>,
  machine_names: Vec<String>,
  cursor_utf16: usize,
}

impl WqxToolsApp {
  pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
    configure_style(&cc.egui_ctx);
    egui_extras::install_image_loaders(&cc.egui_ctx);
    let document = Document::new();
    let source = document.text().to_string();
    let syntax = Syntax::new("GVBASIC")
      .with_case_sensitive(false)
      .with_comment("REM")
      .with_comment_multiline(["REM", "\n"])
      .with_keywords([
        "AND", "BEEP", "BOX", "CALL", "CHECKKEY", "CIRCLE", "CLEAR", "CLOSE",
        "CLS", "COLOR", "DATA", "DEF", "DIM", "DRAW", "ELSE", "END", "FIELD",
        "FOR", "FPUTC", "FREAD", "FSEEK", "FWRITE", "GET", "GOSUB", "GOTO",
        "IF", "INPUT", "LET", "LINE", "LOAD", "LOCATE", "LSET", "NEXT", "NOT",
        "ON", "OPEN", "OR", "PAINT", "PLAY", "POINT", "POKE", "POP", "PRINT",
        "PUT", "READ", "REM", "RESTORE", "RETURN", "RSET", "RUN", "SCREEN",
        "SET", "SLEEP", "STOP", "SWAP", "THEN", "TO", "USING", "WEND", "WHILE",
        "WRITE", "XOR",
      ]);

    let mut app = Self {
      document,
      source_synced: source.clone(),
      source,
      path: None,
      dirty: false,
      diagnostics: Vec::new(),
      status: "准备就绪".to_owned(),
      show_problems: true,
      simulator: None,
      syntax,
      close_prompt: false,
      show_find: false,
      show_replace: false,
      find_text: String::new(),
      replace_text: String::new(),
      match_case: false,
      relabel_open: false,
      relabel_start: 10,
      relabel_step: 10,
      help_open: false,
      emoji_open: false,
      select_range: None,
      machine_names: gvb_interp::machine::names()
        .map(ToString::to_string)
        .collect(),
      cursor_utf16: 0,
    };
    app.refresh_diagnostics();
    app
  }

  fn title(&self) -> String {
    let name = self
      .path
      .as_deref()
      .and_then(Path::file_name)
      .and_then(|name| name.to_str())
      .unwrap_or("未命名.bas.txt");
    format!("WQX 工具箱 — {name}{}", if self.dirty { " *" } else { "" })
  }

  fn new_document(&mut self) {
    self.stop_simulator();
    self.document = Document::new();
    self.source = self.document.text().to_string();
    self.source_synced = self.source.clone();
    self.path = None;
    self.dirty = false;
    self.status = "已新建 GVBASIC 文档".to_owned();
    self.refresh_diagnostics();
  }

  fn open_document(&mut self) {
    let Some(path) = rfd::FileDialog::new()
      .add_filter("GVBASIC", &["bas", "txt"])
      .pick_file()
    else {
      return;
    };
    match Document::load_file(&path) {
      Ok(document) => {
        self.stop_simulator();
        self.document = document;
        self.source = self.document.text().to_string();
        self.source_synced = self.source.clone();
        self.path = Some(path.clone());
        self.dirty = false;
        self.status = format!("已打开 {}", path.display());
        self.refresh_diagnostics();
      }
      Err(error) => self.status = format!("打开失败: {error:?}"),
    }
  }

  fn save(&mut self, save_as: bool) {
    self.sync_document();
    let path = if save_as || self.path.is_none() {
      let Some(path) = rfd::FileDialog::new()
        .add_filter("GVBASIC 文本", &["txt"])
        .add_filter("GVBASIC 二进制", &["bas"])
        .set_file_name("program.bas.txt")
        .save_file()
      else {
        return;
      };
      path
    } else {
      self.path.clone().expect("path checked above")
    };

    match self.document.save(&path) {
      Ok(()) => {
        self.path = Some(path.clone());
        self.dirty = false;
        self.status = format!("已保存 {}", path.display());
      }
      Err(error) => self.status = format!("保存失败: {error:?}"),
    }
  }

  fn sync_document(&mut self) {
    if self.source == self.source_synced {
      return;
    }
    let old_len = self.document.text().len();
    if old_len > 0 {
      self.document.apply_edit(Edit {
        pos: 0,
        kind: EditKind::Delete(old_len),
      });
    }
    let utf16 = Utf16String::from(self.source.as_str());
    if !utf16.is_empty() {
      self.document.apply_edit(Edit {
        pos: 0,
        kind: EditKind::Insert(&utf16),
      });
    }
    self.source_synced = self.source.clone();
  }

  fn refresh_diagnostics(&mut self) {
    self.sync_document();
    self.diagnostics = self
      .document
      .diagnostics()
      .iter()
      .enumerate()
      .flat_map(|(line, result)| {
        result
          .diagnostics
          .iter()
          .map(move |diagnostic| UiDiagnostic {
            line: line + 1,
            column: diagnostic.range.start + 1,
            char_offset: result.line_start + diagnostic.range.start,
            severity: diagnostic.severity,
            message: diagnostic.message.clone(),
          })
      })
      .collect();
  }

  fn stop_simulator(&mut self) {
    if let Some(mut simulator) = self.simulator.take() {
      self.status = match simulator.vm.stop() {
        Ok(()) => "模拟器已停止".to_owned(),
        Err(ExecResult::Error { message, .. }) => {
          format!("停止模拟器时出错: {message}")
        }
        Err(result) => format!("停止模拟器时出错: {result:?}"),
      };
    }
  }

  fn apply_replacements(&mut self, edits: &[ReplaceText]) {
    let mut utf16 = Utf16String::from(self.source.as_str());
    for edit in edits {
      let mut next = Utf16String::with_capacity(
        utf16.len() - edit.range.len() + edit.str.len(),
      );
      next.push_utfstr(&utf16[..edit.range.start]);
      next.push_utfstr(&edit.str);
      next.push_utfstr(&utf16[edit.range.end..]);
      utf16 = next;
    }
    self.source = utf16.to_string();
    self.dirty = true;
    self.refresh_diagnostics();
  }

  fn set_machine(&mut self, name: &str) {
    self.sync_document();
    let name = Utf16String::from(name);
    match self.document.compute_machine_name_edit(&name) {
      Ok(edit) => {
        self.apply_replacements(&[edit]);
        self.sync_document();
        match self.document.sync_machine_name() {
          Ok(_) => {
            self.source = self.document.text().to_string();
            self.source_synced = self.source.clone();
            self.dirty = true;
            self.refresh_diagnostics();
            self.status =
              format!("已切换机型为 {}", self.document.machine_name());
          }
          Err(error) => self.status = format!("同步机型失败: {error:?}"),
        }
      }
      Err(error) => self.status = format!("切换机型失败: {error:?}"),
    }
  }

  fn relabel(&mut self) {
    self.sync_document();
    match self
      .document
      .compute_relabel_edits(self.relabel_start, self.relabel_step)
    {
      Ok(edits) => {
        self.apply_replacements(&edits);
        self.relabel_open = false;
        self.status = "已重排行号及跳转引用".to_owned();
      }
      Err(error) => self.status = format!("重排行号失败: {error:?}"),
    }
  }

  fn add_label(&mut self, target: gvb_interp::LabelTarget) {
    self.sync_document();
    match self
      .document
      .compute_add_label_edit(target, self.cursor_utf16)
    {
      Ok(result) => {
        self.apply_replacements(&[result.edit]);
        self.status = "已添加行号".to_owned();
      }
      Err(error) => self.status = format!("添加行号失败: {error:?}"),
    }
  }

  fn replace_all(&mut self) {
    if self.find_text.is_empty() {
      return;
    }
    let count = if self.match_case {
      let count = self.source.matches(&self.find_text).count();
      self.source = self.source.replace(&self.find_text, &self.replace_text);
      count
    } else {
      replace_all_ignore_ascii_case(
        &mut self.source,
        &self.find_text,
        &self.replace_text,
      )
    };
    if count > 0 {
      self.dirty = true;
      self.refresh_diagnostics();
    }
    self.status = format!("替换了 {count} 处");
  }

  fn find_next(&mut self) {
    if self.find_text.is_empty() {
      return;
    }
    let haystack = if self.match_case {
      self.source.clone()
    } else {
      self.source.to_ascii_uppercase()
    };
    let needle = if self.match_case {
      self.find_text.clone()
    } else {
      self.find_text.to_ascii_uppercase()
    };
    if let Some(start) = haystack.find(&needle) {
      self.select_range = Some((
        self.source[..start].chars().count(),
        self.source[..start + needle.len()].chars().count(),
      ));
      self.status = format!("找到“{}”", self.find_text);
    } else {
      self.status = format!("未找到“{}”", self.find_text);
    }
  }

  fn start_simulator(&mut self, ctx: &egui::Context) {
    self.sync_document();
    self.refresh_diagnostics();
    if self
      .diagnostics
      .iter()
      .any(|diagnostic| diagnostic.severity == Severity::Error)
    {
      self.status = "源码含错误，修复后才能运行".to_owned();
      self.show_problems = true;
      return;
    }

    let data_dir = PathBuf::from("dat_files");
    if let Err(error) = std::fs::create_dir_all(&data_dir) {
      self.status = format!("创建数据目录失败: {error}");
      return;
    }

    let device = self.document.create_device(data_dir);
    let Ok(mut vm) = self.document.create_vm(device) else {
      self.status = "编译失败".to_owned();
      return;
    };
    vm.device_mut().reset();
    vm.start();
    let result = vm.exec(ExecInput::None, EXEC_STEPS);
    let texture = ctx.load_texture(
      "wqx-screen",
      screen_image(vm.device()),
      TextureOptions::NEAREST,
    );
    self.simulator = Some(Simulator {
      vm,
      state: RunState::Running,
      result,
      wake_at: None,
      next_cursor_blink: Instant::now(),
      texture: Some(texture),
      input_values: Vec::new(),
      input_error: None,
      array_inspector: None,
    });
    self.status = "模拟器已启动".to_owned();
  }

  fn advance_simulator(&mut self, ctx: &egui::Context) {
    let Some(simulator) = self.simulator.as_mut() else {
      return;
    };
    if simulator.state == RunState::Paused {
      return;
    }
    if let Some(wake_at) = simulator.wake_at {
      if Instant::now() < wake_at {
        ctx.request_repaint_after(
          wake_at.saturating_duration_since(Instant::now()),
        );
        return;
      }
      simulator.wake_at = None;
      simulator.state = RunState::Running;
      simulator.result = simulator.vm.exec(ExecInput::None, EXEC_STEPS);
    }

    if simulator.state == RunState::Waiting
      && Instant::now() >= simulator.next_cursor_blink
    {
      simulator.vm.device_mut().blink_cursor();
      simulator.next_cursor_blink =
        Instant::now() + std::time::Duration::from_millis(500);
      ctx.request_repaint_after(std::time::Duration::from_millis(500));
    }

    let next_input = match &simulator.result {
      ExecResult::Continue => Some(ExecInput::None),
      ExecResult::Sleep(duration) => {
        simulator.state = RunState::Sleeping;
        simulator.wake_at = Some(Instant::now() + *duration);
        ctx.request_repaint_after(*duration);
        None
      }
      ExecResult::InKey => {
        simulator.state = RunState::Waiting;
        ctx.request_repaint_after(std::time::Duration::from_millis(500));
        None
      }
      ExecResult::KeyboardInput { fields, .. } if fields.is_empty() => {
        Some(ExecInput::KeyboardInput(Vec::new()))
      }
      ExecResult::KeyboardInput { fields, .. } => {
        simulator.state = RunState::Waiting;
        if simulator.input_values.len() != fields.len() {
          simulator.input_values = vec![String::new(); fields.len()];
        }
        None
      }
      ExecResult::End => {
        simulator.state = RunState::Finished;
        None
      }
      ExecResult::Error { message, .. } => {
        simulator.state = RunState::Error;
        self.status = format!("运行时错误: {message}");
        None
      }
    };
    if let Some(input) = next_input {
      simulator.state = RunState::Running;
      simulator.result = simulator.vm.exec(input, EXEC_STEPS);
      ctx.request_repaint();
    }

    if simulator.vm.device_mut().take_dirty_area().is_some()
      && let Some(texture) = simulator.texture.as_mut()
    {
      texture.set(screen_image(simulator.vm.device()), TextureOptions::NEAREST);
    }
  }

  fn simulator_key(&mut self, key: u8, pressed: bool, ctx: &egui::Context) {
    let Some(simulator) = self.simulator.as_mut() else {
      return;
    };
    if pressed {
      simulator.vm.device_mut().fire_key_down(key);
      if matches!(simulator.result, ExecResult::InKey)
        && let Some(actual) = simulator.vm.device_mut().key()
      {
        simulator.result =
          simulator.vm.exec(ExecInput::Key(actual), EXEC_STEPS);
        simulator.state = RunState::Running;
        ctx.request_repaint();
      }
    } else {
      simulator.vm.device_mut().fire_key_up(key);
    }
  }

  fn top_bar(&mut self, root: &mut egui::Ui) {
    let ctx = root.ctx().clone();
    egui::Panel::top("top_bar")
      .frame(
        egui::Frame::new()
          .fill(theme::PANEL)
          .inner_margin(egui::Margin::symmetric(16, 8))
          .stroke(egui::Stroke::new(1.0, theme::BORDER)),
      )
      .show(root, |ui| {
        egui::MenuBar::new().ui(ui, |ui| {
          ui.label(
            RichText::new("WQX")
              .size(17.0)
              .strong()
              .color(theme::ACCENT),
          );
          ui.label(
            RichText::new("TOOLS")
              .size(12.0)
              .strong()
              .color(theme::MUTED),
          );
          ui.add_space(12.0);
          ui.separator();
          ui.menu_button("文件", |ui| {
            if ui.button("新建  Ctrl+N").clicked() {
              self.new_document();
              ui.close();
            }
            if ui.button("打开…  Ctrl+O").clicked() {
              self.open_document();
              ui.close();
            }
            if ui.button("保存  Ctrl+S").clicked() {
              self.save(false);
              ui.close();
            }
            if ui.button("另存为…  Ctrl+Shift+S").clicked() {
              self.save(true);
              ui.close();
            }
          });
          ui.menu_button("程序", |ui| {
            if ui.button("运行  F5").clicked() {
              self.start_simulator(&ctx);
              ui.close();
            }
            if ui.button("停止  Shift+F5").clicked() {
              self.stop_simulator();
              ui.close();
            }
          });
          ui.menu_button("编辑", |ui| {
            if ui.button("查找  Ctrl+F").clicked() {
              self.show_find = true;
              ui.close();
            }
            if ui.button("替换  Ctrl+H").clicked() {
              self.show_find = true;
              self.show_replace = true;
              ui.close();
            }
            if ui.button("重排行号…").clicked() {
              self.relabel_open = true;
              ui.close();
            }
            if ui.button("为当前行添加行号").clicked() {
              self.add_label(gvb_interp::LabelTarget::CurLine);
              ui.close();
            }
            if ui.button("在上一行插入带行号的行").clicked() {
              self.add_label(gvb_interp::LabelTarget::PrevLine);
              ui.close();
            }
            if ui.button("在下一行插入带行号的行").clicked() {
              self.add_label(gvb_interp::LabelTarget::NextLine);
              ui.close();
            }
            if ui.button("文曲星符号…").clicked() {
              self.emoji_open = true;
              ui.close();
            }
          });
          if ui.button("帮助").clicked() {
            self.help_open = true;
          }
        });
        ui.add_space(7.0);
        ui.horizontal(|ui| {
          if ui.button("新建").on_hover_text("Ctrl+N").clicked() {
            self.new_document();
          }
          if ui.button("打开").on_hover_text("Ctrl+O").clicked() {
            self.open_document();
          }
          if ui
            .add(egui::Button::image_and_text(
              toolbar_icon(
                egui::include_image!("../assets/icons/save.svg"),
                theme::TEXT,
              ),
              "保存",
            ))
            .on_hover_text("Ctrl+S")
            .clicked()
          {
            self.save(false);
          }
          ui.add_space(7.0);
          ui.separator();
          if ui
            .add(egui::Button::image_and_text(
              toolbar_icon(
                egui::include_image!("../assets/icons/search.svg"),
                theme::TEXT,
              ),
              "查找",
            ))
            .on_hover_text("Ctrl+F")
            .clicked()
          {
            self.show_find = true;
          }
          if ui
            .add(egui::Button::image_and_text(
              toolbar_icon(
                egui::include_image!("../assets/icons/replace.svg"),
                theme::TEXT,
              ),
              "替换",
            ))
            .on_hover_text("Ctrl+H")
            .clicked()
          {
            self.show_find = true;
            self.show_replace = true;
          }
          ui.add_space(7.0);
          ui.separator();
          if ui
            .add(
              egui::Button::image_and_text(
                toolbar_icon(
                  egui::include_image!("../assets/icons/play.svg"),
                  theme::BACKGROUND,
                ),
                RichText::new("运行").strong().color(theme::BACKGROUND),
              )
              .fill(theme::ACCENT)
              .stroke(egui::Stroke::NONE)
              .corner_radius(6),
            )
            .on_hover_text("F5")
            .clicked()
          {
            self.start_simulator(&ctx);
          }
          if ui
            .add(egui::Button::image_and_text(
              toolbar_icon(
                egui::include_image!("../assets/icons/stop.svg"),
                theme::TEXT,
              ),
              "停止",
            ))
            .on_hover_text("Shift+F5")
            .clicked()
          {
            self.stop_simulator();
          }
          ui.add_space(7.0);
          ui.separator();
          ui.label(RichText::new("机型").color(theme::MUTED));
          let current_machine = self.document.machine_name().to_owned();
          egui::ComboBox::from_id_salt("machine-selector")
            .selected_text(&current_machine)
            .show_ui(ui, |ui| {
              for name in self.machine_names.clone() {
                if ui
                  .selectable_label(name == current_machine, &name)
                  .clicked()
                {
                  self.set_machine(&name);
                  ui.close();
                }
              }
            });
          ui.with_layout(
            egui::Layout::right_to_left(egui::Align::Center),
            |ui| {
              let filename = self
                .path
                .as_deref()
                .and_then(Path::file_name)
                .and_then(|name| name.to_str())
                .unwrap_or("未命名.bas.txt");
              ui.label(RichText::new(filename).color(theme::MUTED));
            },
          );
        });
      });
  }

  fn editor_panel(&mut self, root: &mut egui::Ui) {
    egui::CentralPanel::default()
      .frame(
        egui::Frame::new()
          .fill(theme::BACKGROUND)
          .inner_margin(egui::Margin::symmetric(16, 14)),
      )
      .show(root, |ui| {
        ui.horizontal(|ui| {
          ui.label(
            RichText::new("GVBASIC 编辑器")
              .size(16.0)
              .strong()
              .color(theme::TEXT),
          );
          ui.add_space(7.0);
          if self.dirty {
            ui.label(RichText::new("● 未保存").small().color(theme::WARNING));
          } else {
            ui.label(
              RichText::new(if self.path.is_some() {
                "● 已保存"
              } else {
                "● 未修改"
              })
              .small()
              .color(theme::MUTED),
            );
          }
          ui.with_layout(
            egui::Layout::right_to_left(egui::Align::Center),
            |ui| {
              ui.label(
                RichText::new("F5 运行 · Ctrl+F 查找")
                  .small()
                  .color(theme::MUTED),
              );
            },
          );
        });
        ui.add_space(9.0);
        if self.show_find {
          egui::Frame::new()
            .fill(theme::SURFACE)
            .corner_radius(6)
            .inner_margin(egui::Margin::symmetric(10, 7))
            .show(ui, |ui| {
              ui.horizontal_wrapped(|ui| {
                ui.label("查找");
                let find = ui.add_sized(
                  [180.0, 24.0],
                  egui::TextEdit::singleline(&mut self.find_text),
                );
                if find.lost_focus()
                  && ui.input(|input| input.key_pressed(egui::Key::Enter))
                {
                  self.find_next();
                }
                if ui.button("下一个").clicked() {
                  self.find_next();
                }
                if self.show_replace {
                  ui.separator();
                  ui.label("替换为");
                  ui.add_sized(
                    [180.0, 24.0],
                    egui::TextEdit::singleline(&mut self.replace_text),
                  );
                  if ui.button("全部替换").clicked() {
                    self.replace_all();
                  }
                }
                ui.checkbox(&mut self.match_case, "匹配大小写");
                if ui.button("×").clicked() {
                  self.show_find = false;
                  self.show_replace = false;
                }
              });
            });
          ui.add_space(9.0);
        }
        let available_rows =
          ((ui.available_height() - 28.0) / 17.0).max(12.0) as usize;
        egui::Frame::new()
          .fill(theme::EDITOR)
          .stroke(egui::Stroke::new(1.0, theme::BORDER))
          .corner_radius(8)
          .inner_margin(egui::Margin::symmetric(12, 10))
          .show(ui, |ui| {
            let editor = ui.add_enabled_ui(self.simulator.is_none(), |ui| {
              CodeEditor::default()
                .id_source("gvbasic-editor")
                .with_rows(available_rows)
                .with_fontsize(15.0)
                .with_theme(EDITOR_THEME)
                .with_numlines(true)
                .with_clickable_links(false)
                .show(ui, &mut self.source, &self.syntax)
            });
            let (mut output, _) = editor.inner;
            if let Some((start, end)) = self.select_range.take() {
              output.state.cursor.set_char_range(Some(
                egui::text::CCursorRange::two(
                  egui::text::CCursor::new(start),
                  egui::text::CCursor::new(end),
                ),
              ));
              output.state.store(ui.ctx(), output.response.id);
              output.response.request_focus();
            }
            if let Some(range) = output.cursor_range {
              let char_offset: usize = range.primary.index.into();
              self.cursor_utf16 = self
                .source
                .chars()
                .take(char_offset)
                .map(char::len_utf16)
                .sum();
            }
            if output.response.changed() {
              self.dirty = true;
              self.refresh_diagnostics();
            }
          });
      });
  }

  fn problems_panel(&mut self, root: &mut egui::Ui) {
    let error_count = self
      .diagnostics
      .iter()
      .filter(|item| item.severity == Severity::Error)
      .count();
    let warning_count = self.diagnostics.len() - error_count;
    let mut panel = egui::Panel::bottom("problems").frame(
      egui::Frame::new()
        .fill(theme::PANEL)
        .inner_margin(egui::Margin::symmetric(16, 8))
        .stroke(egui::Stroke::new(1.0, theme::BORDER)),
    );
    panel = if self.show_problems {
      panel.resizable(true).default_size(170.0).min_size(145.0)
    } else {
      panel.exact_size(48.0)
    };
    panel.show(root, |ui| {
      ui.horizontal(|ui| {
        if ui
          .selectable_label(self.show_problems, RichText::new("问题").strong())
          .clicked()
        {
          self.show_problems = !self.show_problems;
        }
        if error_count > 0 {
          ui.label(
            RichText::new(format!("● {error_count} 错误")).color(theme::ERROR),
          );
        }
        if warning_count > 0 {
          ui.label(
            RichText::new(format!("● {warning_count} 警告"))
              .color(theme::WARNING),
          );
        }
        if self.diagnostics.is_empty() {
          ui.label(RichText::new("● 无问题").color(theme::ACCENT));
        }
        ui.separator();
        ui.label(RichText::new(&self.status).color(theme::MUTED));
      });
      if !self.show_problems {
        return;
      }
      ui.add_space(6.0);
      if self.diagnostics.is_empty() {
        ui.add_space(22.0);
        ui.vertical_centered(|ui| {
          ui.label(
            RichText::new("✓  当前文档没有诊断问题").color(theme::MUTED),
          );
        });
        return;
      }
      TableBuilder::new(ui)
        .striped(true)
        .column(Column::exact(72.0))
        .column(Column::remainder())
        .column(Column::exact(110.0))
        .header(22.0, |mut header| {
          header.col(|ui| {
            ui.strong("级别");
          });
          header.col(|ui| {
            ui.strong("问题");
          });
          header.col(|ui| {
            ui.strong("位置");
          });
        })
        .body(|mut body| {
          for item in &self.diagnostics {
            body.row(22.0, |mut row| {
              row.col(|ui| {
                let (text, color) = match item.severity {
                  Severity::Error => ("● 错误", theme::ERROR),
                  Severity::Warning => ("● 警告", theme::WARNING),
                };
                ui.label(RichText::new(text).color(color));
              });
              row.col(|ui| {
                if ui.selectable_label(false, &item.message).clicked() {
                  let offset = self.source
                    [..byte_offset_for_utf16(&self.source, item.char_offset)]
                    .chars()
                    .count();
                  self.select_range = Some((offset, offset));
                }
              });
              row.col(|ui| {
                ui.label(format!("{}:{}", item.line, item.column));
              });
            });
          }
        });
    });
  }

  fn simulator_window(&mut self, ctx: &egui::Context) {
    let Some(state) = self.simulator.as_ref().map(|simulator| simulator.state)
    else {
      return;
    };
    let mut open = true;
    egui::Window::new("文曲星模拟器")
      .id(egui::Id::new("simulator-window"))
      .open(&mut open)
      .default_size([840.0, (ctx.content_rect().height() - 32.0).min(710.0)])
      .resizable(true)
      .vscroll(true)
      .show(ctx, |ui| {
        ui.horizontal(|ui| {
          ui.label(RichText::new("GVBASIC").strong().color(theme::ACCENT));
          ui.label(RichText::new("·").color(theme::MUTED));
          let state_color = match state {
            RunState::Error => theme::ERROR,
            RunState::Running => theme::ACCENT,
            _ => theme::WARNING,
          };
          ui.label(RichText::new(state.label()).color(state_color));
          ui.add_space(12.0);
          let running = self
            .simulator
            .as_ref()
            .is_some_and(|simulator| simulator.state == RunState::Running);
          if ui
            .add_enabled(
              matches!(state, RunState::Running | RunState::Paused),
              egui::Button::image_and_text(
                toolbar_icon(
                  if running {
                    egui::include_image!("../assets/icons/pause.svg")
                  } else {
                    egui::include_image!("../assets/icons/play.svg")
                  },
                  theme::TEXT,
                ),
                if running { "暂停" } else { "继续" },
              ),
            )
            .clicked()
            && let Some(simulator) = self.simulator.as_mut()
          {
            simulator.state = if running {
              RunState::Paused
            } else {
              RunState::Running
            };
            ctx.request_repaint();
          }
          if ui
            .add(egui::Button::image_and_text(
              toolbar_icon(
                egui::include_image!("../assets/icons/stop.svg"),
                theme::TEXT,
              ),
              "停止",
            ))
            .clicked()
          {
            self.stop_simulator();
          }
        });
        ui.add_space(10.0);
        ui.columns(2, |columns| {
          egui::Frame::new()
            .fill(theme::SURFACE)
            .stroke(egui::Stroke::new(1.0, theme::BORDER))
            .corner_radius(10)
            .inner_margin(egui::Margin::same(14))
            .show(&mut columns[0], |ui| {
              ui.label(
                RichText::new("LCD DISPLAY")
                  .small()
                  .strong()
                  .color(theme::MUTED),
              );
              ui.add_space(8.0);
              egui::Frame::new()
                .fill(Color32::from_rgb(49, 62, 56))
                .corner_radius(5)
                .inner_margin(egui::Margin::same(8))
                .show(ui, |ui| {
                  ui.with_layout(
                    egui::Layout::top_down(egui::Align::Center),
                    |ui| {
                      if let Some(texture) = self
                        .simulator
                        .as_ref()
                        .and_then(|simulator| simulator.texture.as_ref())
                      {
                        ui.add(egui::Image::new(texture).fit_to_exact_size(
                          egui::vec2(
                            SCREEN_WIDTH as f32 * 2.25,
                            SCREEN_HEIGHT as f32 * 2.25,
                          ),
                        ));
                      }
                    },
                  );
                });
              ui.add_space(6.0);
              ui.label(
                RichText::new("160 × 80 MONOCHROME")
                  .small()
                  .color(theme::MUTED),
              );
            });
          egui::Frame::new()
            .fill(theme::SURFACE)
            .stroke(egui::Stroke::new(1.0, theme::BORDER))
            .corner_radius(10)
            .inner_margin(egui::Margin::same(14))
            .show(&mut columns[1], |ui| {
              ui.label(RichText::new("变量监视").strong().color(theme::TEXT));
              ui.add_space(8.0);
              egui::ScrollArea::vertical()
                .max_height(190.0)
                .show(ui, |ui| {
                  self.bindings_ui(ui);
                });
            });
        });
        ui.add_space(12.0);
        let keyboard_width = ui.available_width();
        egui::Frame::new()
          .fill(theme::SURFACE)
          .stroke(egui::Stroke::new(1.0, theme::BORDER))
          .corner_radius(10)
          .inner_margin(egui::Margin::symmetric(14, 10))
          .show(ui, |ui| {
            ui.set_min_width(keyboard_width - 28.0);
            ui.label(RichText::new("设备键盘").strong().color(theme::TEXT));
            ui.add_space(7.0);
            self.keyboard_ui(ui, ctx);
          });
        self.input_ui(ui, ctx);
      });
    if !open {
      self.stop_simulator();
    }
    self.array_window(ctx);
  }

  fn bindings_ui(&mut self, ui: &mut egui::Ui) {
    let Some(simulator) = self.simulator.as_mut() else {
      return;
    };
    let editable = simulator.state == RunState::Paused;
    for (name, binding) in simulator.vm.bindings() {
      ui.horizontal(|ui| {
        ui.monospace(&name);
        match binding {
          Binding::Array { dimensions } => {
            if ui.button(format!("数组 {:?}", dimensions)).clicked() {
              let len = dimensions.len();
              simulator.array_inspector = Some(ArrayInspector {
                name: name.clone(),
                dimensions,
                axis: 0,
                indices: vec![0; len],
              });
            }
          }
          Binding::Var {
            value: Value::Integer(value),
          } => {
            let mut next = value;
            if ui
              .add_enabled(editable, egui::DragValue::new(&mut next))
              .changed()
            {
              simulator.vm.modify_var(&name, Value::Integer(next));
            }
          }
          Binding::Var {
            value: Value::Real(value),
          } => {
            let mut next = f64::from(value);
            if ui
              .add_enabled(editable, egui::DragValue::new(&mut next).speed(0.1))
              .changed()
              && let Ok(next) = next.try_into()
            {
              simulator.vm.modify_var(&name, Value::Real(next));
            }
          }
          Binding::Var {
            value: Value::String(value),
          } => {
            let mut next = simulator.vm.string_from_byte_string_lossy(value);
            if ui
              .add_enabled(editable, egui::TextEdit::singleline(&mut next))
              .changed()
            {
              let utf16 = Utf16String::from(next.as_str());
              let (next, problems) =
                simulator.vm.byte_string_from_utf16str(&utf16);
              if problems.is_empty() && next.len() <= 255 {
                simulator.vm.modify_var(&name, Value::String(next));
              }
            }
          }
        }
      });
      ui.separator();
    }
    if !editable {
      ui.small("暂停程序后可修改变量");
    }
  }

  fn array_window(&mut self, ctx: &egui::Context) {
    let Some(simulator) = self.simulator.as_mut() else {
      return;
    };
    let Some(mut inspector) = simulator.array_inspector.take() else {
      return;
    };
    let mut open = true;
    egui::Window::new(format!("数组 {}", inspector.name))
      .open(&mut open)
      .show(ctx, |ui| {
        ui.horizontal(|ui| {
          ui.label("查看维度");
          for axis in 0..inspector.dimensions.len() {
            ui.selectable_value(
              &mut inspector.axis,
              axis,
              (axis + 1).to_string(),
            );
          }
        });
        for (axis, bound) in inspector.dimensions.iter().copied().enumerate() {
          if axis != inspector.axis {
            ui.horizontal(|ui| {
              ui.label(format!("维度 {} 索引", axis + 1));
              ui.add(
                egui::DragValue::new(&mut inspector.indices[axis])
                  .range(0..=bound),
              );
            });
          }
        }
        ui.separator();
        let editable = simulator.state == RunState::Paused;
        let values = simulator.vm.arr_dimension_values(
          &inspector.name,
          &inspector.indices,
          inspector.axis,
        );
        egui::ScrollArea::vertical()
          .max_height(320.0)
          .show(ui, |ui| match values {
            gvb_interp::DimensionValues::Integer(values) => {
              for (index, value) in values.into_iter().enumerate() {
                let mut next = value;
                ui.horizontal(|ui| {
                  ui.monospace(index.to_string());
                  if ui
                    .add_enabled(editable, egui::DragValue::new(&mut next))
                    .changed()
                  {
                    let mut indices = inspector.indices.clone();
                    indices[inspector.axis] = index as u16;
                    simulator.vm.modify_arr(
                      &inspector.name,
                      &indices,
                      Value::Integer(next),
                    );
                  }
                });
              }
            }
            gvb_interp::DimensionValues::Real(values) => {
              for (index, value) in values.into_iter().enumerate() {
                let mut next = f64::from(value);
                ui.horizontal(|ui| {
                  ui.monospace(index.to_string());
                  if ui
                    .add_enabled(
                      editable,
                      egui::DragValue::new(&mut next).speed(0.1),
                    )
                    .changed()
                    && let Ok(next) = next.try_into()
                  {
                    let mut indices = inspector.indices.clone();
                    indices[inspector.axis] = index as u16;
                    simulator.vm.modify_arr(
                      &inspector.name,
                      &indices,
                      Value::Real(next),
                    );
                  }
                });
              }
            }
            gvb_interp::DimensionValues::String(values) => {
              for (index, value) in values.into_iter().enumerate() {
                let mut next =
                  simulator.vm.string_from_byte_string_lossy(value);
                ui.horizontal(|ui| {
                  ui.monospace(index.to_string());
                  if ui
                    .add_enabled(
                      editable,
                      egui::TextEdit::singleline(&mut next),
                    )
                    .changed()
                  {
                    let utf16 = Utf16String::from(next.as_str());
                    let (next, problems) =
                      simulator.vm.byte_string_from_utf16str(&utf16);
                    if problems.is_empty() && next.len() <= 255 {
                      let mut indices = inspector.indices.clone();
                      indices[inspector.axis] = index as u16;
                      simulator.vm.modify_arr(
                        &inspector.name,
                        &indices,
                        Value::String(next),
                      );
                    }
                  }
                });
              }
            }
          });
      });
    if open {
      simulator.array_inspector = Some(inspector);
    }
  }

  fn keyboard_ui(&mut self, ui: &mut egui::Ui, ctx: &egui::Context) {
    ui.vertical(|ui| {
      let left_padding = ((ui.available_width() - 592.0) / 2.0).max(0.0);
      for row in KEYBOARD {
        ui.horizontal(|ui| {
          if left_padding > 0.0 {
            ui.add_space(left_padding);
          }
          for (label, code) in row {
            if code == 0 {
              ui.add_space(52.0);
            } else {
              let response = ui.add_sized(
                [50.0, 30.0],
                egui::Button::new(label)
                  .fill(theme::PANEL)
                  .stroke(egui::Stroke::new(1.0, theme::BORDER))
                  .corner_radius(5),
              );
              if response.is_pointer_button_down_on() {
                self.simulator_key(code, true, ctx);
              } else if response.clicked() {
                self.simulator_key(code, true, ctx);
                self.simulator_key(code, false, ctx);
              }
            }
          }
        });
      }
    });
  }

  fn input_ui(&mut self, ui: &mut egui::Ui, ctx: &egui::Context) {
    let fields = self
      .simulator
      .as_ref()
      .and_then(|simulator| match &simulator.result {
        ExecResult::KeyboardInput { prompt, fields } => {
          Some((prompt.clone(), fields.clone()))
        }
        _ => None,
      });
    let Some((prompt, fields)) = fields else {
      return;
    };
    ui.separator();
    if let Some(prompt) = prompt {
      ui.strong(prompt);
    }
    let Some(simulator) = self.simulator.as_mut() else {
      return;
    };
    for (index, field) in fields.iter().enumerate() {
      ui.horizontal(|ui| {
        ui.label(input_label(field));
        ui.text_edit_singleline(&mut simulator.input_values[index]);
      });
    }
    if let Some(error) = &simulator.input_error {
      ui.label(RichText::new(error).color(Color32::from_rgb(248, 113, 113)));
    }
    if ui.button("提交输入").clicked() {
      match parse_inputs(&simulator.vm, &fields, &simulator.input_values) {
        Ok(inputs) => {
          simulator.input_error = None;
          simulator.result = simulator
            .vm
            .exec(ExecInput::KeyboardInput(inputs), EXEC_STEPS);
          simulator.state = RunState::Running;
          ctx.request_repaint();
        }
        Err(error) => simulator.input_error = Some(error),
      }
    }
  }

  fn shortcuts(&mut self, ctx: &egui::Context) {
    if ctx
      .input_mut(|input| input.consume_key(egui::Modifiers::CTRL, egui::Key::N))
    {
      self.new_document();
    }
    if ctx
      .input_mut(|input| input.consume_key(egui::Modifiers::CTRL, egui::Key::O))
    {
      self.open_document();
    }
    if ctx.input_mut(|input| {
      input.consume_key(
        egui::Modifiers::CTRL | egui::Modifiers::SHIFT,
        egui::Key::S,
      )
    }) {
      self.save(true);
    }
    if ctx
      .input_mut(|input| input.consume_key(egui::Modifiers::CTRL, egui::Key::S))
    {
      self.save(false);
    }
    if ctx.input_mut(|input| {
      input.consume_key(egui::Modifiers::SHIFT, egui::Key::F5)
    }) {
      self.stop_simulator();
    }
    if ctx.input_mut(|input| {
      input.consume_key(egui::Modifiers::NONE, egui::Key::F5)
    }) {
      self.start_simulator(ctx);
    }
    if ctx
      .input_mut(|input| input.consume_key(egui::Modifiers::CTRL, egui::Key::F))
    {
      self.show_find = true;
    }
    if ctx
      .input_mut(|input| input.consume_key(egui::Modifiers::CTRL, egui::Key::H))
    {
      self.show_find = true;
      self.show_replace = true;
    }
    self.hardware_keyboard(ctx);
  }

  fn hardware_keyboard(&mut self, ctx: &egui::Context) {
    if self.simulator.is_none() {
      return;
    }
    let events = ctx.input(|input| input.raw.events.clone());
    for event in events {
      let egui::Event::Key {
        key,
        pressed,
        repeat,
        modifiers,
        ..
      } = event
      else {
        continue;
      };
      if repeat || modifiers.command {
        continue;
      }
      if let Some(key) = wqx_key(key, modifiers) {
        self.simulator_key(key, pressed, ctx);
      }
    }
  }

  fn dialogs(&mut self, ctx: &egui::Context) {
    if self.relabel_open {
      let mut open = true;
      egui::Window::new("重排行号")
        .collapsible(false)
        .resizable(false)
        .open(&mut open)
        .show(ctx, |ui| {
          ui.horizontal(|ui| {
            ui.label("起始行号");
            ui.add(
              egui::DragValue::new(&mut self.relabel_start).range(0..=9999),
            );
          });
          ui.horizontal(|ui| {
            ui.label("行号步长");
            ui.add(
              egui::DragValue::new(&mut self.relabel_step).range(1..=9999),
            );
          });
          if ui.button("应用").clicked() {
            self.relabel();
          }
        });
      self.relabel_open &= open;
    }

    if self.emoji_open {
      let mut open = true;
      egui::Window::new("文曲星图形符号")
        .open(&mut open)
        .default_size([480.0, 360.0])
        .show(ctx, |ui| {
          ui.label("点击符号复制到剪贴板");
          egui::ScrollArea::vertical().show(ui, |ui| {
            for row_start in (0..527).step_by(16) {
              ui.horizontal(|ui| {
                for offset in row_start..(row_start + 16).min(527) {
                  let symbol = char::from_u32(0xe000 + offset)
                    .expect("private-use code point");
                  if ui
                    .add_sized(
                      [26.0, 24.0],
                      egui::Button::new(symbol.to_string()),
                    )
                    .clicked()
                  {
                    ctx.copy_text(symbol.to_string());
                    self.status = format!("已复制符号 U+{:04X}", symbol as u32);
                  }
                }
              });
            }
          });
        });
      self.emoji_open &= open;
    }

    if self.help_open {
      let mut open = true;
      egui::Window::new("WQX 工具箱帮助")
        .open(&mut open)
        .default_size([620.0, 480.0])
        .show(ctx, |ui| {
          ui.heading("GVBASIC 编辑器与模拟器");
          ui.label("支持 .bas / .txt 读取、保存、语法诊断、机型选择、行号重排和运行。模拟器提供 160×80 屏幕、文曲星键盘、变量查看与 INPUT 输入。");
          ui.separator();
          ui.strong("快捷键");
          ui.label("Ctrl+N 新建 · Ctrl+O 打开 · Ctrl+S 保存 · Ctrl+F 查找 · Ctrl+H 替换 · F5 运行");
          ui.separator();
          ui.hyperlink_to("项目主页", "https://github.com/arucil/wqxtools");
        });
      self.help_open &= open;
    }
  }
}

impl eframe::App for WqxToolsApp {
  fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
    let ctx = ui.ctx().clone();
    ctx.send_viewport_cmd(egui::ViewportCommand::Title(self.title()));
    self.shortcuts(&ctx);
    self.advance_simulator(&ctx);
    self.top_bar(ui);
    self.problems_panel(ui);
    self.editor_panel(ui);
    self.simulator_window(&ctx);
    self.dialogs(&ctx);

    if self.dirty && ctx.input(|input| input.viewport().close_requested()) {
      ctx.send_viewport_cmd(egui::ViewportCommand::CancelClose);
      self.close_prompt = true;
    }
    if self.close_prompt {
      egui::Window::new("未保存的修改")
        .collapsible(false)
        .resizable(false)
        .anchor(egui::Align2::CENTER_CENTER, egui::Vec2::ZERO)
        .show(&ctx, |ui| {
          ui.label("当前文档尚未保存，仍要退出吗？");
          ui.horizontal(|ui| {
            if ui.button("取消").clicked() {
              self.close_prompt = false;
            }
            if ui.button("保存并退出").clicked() {
              self.save(false);
              if !self.dirty {
                ctx.send_viewport_cmd(egui::ViewportCommand::Close);
              }
            }
            if ui
              .button(
                RichText::new("放弃修改")
                  .color(Color32::from_rgb(248, 113, 113)),
              )
              .clicked()
            {
              self.dirty = false;
              ctx.send_viewport_cmd(egui::ViewportCommand::Close);
            }
          });
        });
    }
  }

  fn on_exit(&mut self, _gl: Option<&eframe::glow::Context>) {
    self.stop_simulator();
  }
}

fn toolbar_icon(
  source: egui::ImageSource<'static>,
  tint: Color32,
) -> egui::Image<'static> {
  egui::Image::new(source)
    .fit_to_exact_size(egui::vec2(16.0, 16.0))
    .tint(tint)
}

fn configure_style(ctx: &egui::Context) {
  let mut fonts = egui::FontDefinitions::default();
  if let Some(system_font) = load_system_cjk_font() {
    fonts
      .font_data
      .insert("System CJK".to_owned(), system_font.into());
    fonts
      .families
      .entry(egui::FontFamily::Proportional)
      .or_default()
      .insert(0, "System CJK".to_owned());
    fonts
      .families
      .entry(egui::FontFamily::Monospace)
      .or_default()
      .push("System CJK".to_owned());
  }
  fonts.font_data.insert(
    "WenQuXing".to_owned(),
    egui::FontData::from_static(include_bytes!("../assets/WenQuXing.ttf"))
      .into(),
  );
  for family in [egui::FontFamily::Proportional, egui::FontFamily::Monospace] {
    fonts
      .families
      .entry(family)
      .or_default()
      .push("WenQuXing".to_owned());
  }
  ctx.set_fonts(fonts);

  let mut visuals = egui::Visuals::dark();
  visuals.panel_fill = theme::BACKGROUND;
  visuals.window_fill = theme::PANEL;
  visuals.window_stroke = egui::Stroke::new(1.0, theme::BORDER);
  visuals.window_corner_radius = egui::CornerRadius::same(10);
  visuals.extreme_bg_color = theme::EDITOR;
  visuals.faint_bg_color = theme::SURFACE;
  visuals.code_bg_color = theme::EDITOR;
  visuals.weak_text_color = Some(theme::MUTED);
  visuals.hyperlink_color = theme::ACCENT;
  visuals.warn_fg_color = theme::WARNING;
  visuals.error_fg_color = theme::ERROR;
  visuals.selection.bg_fill = theme::ACCENT_DARK;
  visuals.selection.stroke.color = theme::ACCENT;
  visuals.widgets.noninteractive.bg_fill = theme::PANEL;
  visuals.widgets.noninteractive.fg_stroke.color = theme::TEXT;
  visuals.widgets.inactive.bg_fill = theme::SURFACE;
  visuals.widgets.inactive.weak_bg_fill = theme::SURFACE;
  visuals.widgets.inactive.bg_stroke.color = theme::BORDER;
  visuals.widgets.inactive.fg_stroke.color = theme::TEXT;
  visuals.widgets.hovered.bg_fill = Color32::from_rgb(34, 53, 62);
  visuals.widgets.hovered.weak_bg_fill = Color32::from_rgb(34, 53, 62);
  visuals.widgets.hovered.bg_stroke.color = theme::ACCENT_DARK;
  visuals.widgets.hovered.fg_stroke.color = theme::TEXT;
  visuals.widgets.active.bg_fill = theme::ACCENT_DARK;
  visuals.widgets.active.weak_bg_fill = theme::ACCENT_DARK;
  visuals.widgets.active.fg_stroke.color = theme::ACCENT;
  visuals.widgets.open.bg_fill = theme::SURFACE;
  visuals.widgets.open.weak_bg_fill = theme::SURFACE;
  visuals.widgets.open.fg_stroke.color = theme::TEXT;
  for widget in [
    &mut visuals.widgets.inactive,
    &mut visuals.widgets.hovered,
    &mut visuals.widgets.active,
    &mut visuals.widgets.open,
  ] {
    widget.corner_radius = egui::CornerRadius::same(6);
  }
  visuals.interact_cursor = Some(egui::CursorIcon::PointingHand);
  ctx.set_visuals(visuals);

  ctx.all_styles_mut(|style| {
    style.spacing.item_spacing = egui::vec2(8.0, 7.0);
    style.spacing.button_padding = egui::vec2(11.0, 6.0);
    style.spacing.window_margin = egui::Margin::same(14);
  });
}

fn load_system_cjk_font() -> Option<egui::FontData> {
  const CANDIDATES: &[&str] = &[
    "Microsoft YaHei UI",
    "Microsoft YaHei",
    "PingFang SC",
    "Hiragino Sans GB",
    "Noto Sans CJK SC",
    "Source Han Sans SC",
    "WenQuanYi Micro Hei",
    "SimHei",
  ];

  let mut database = fontdb::Database::new();
  database.load_system_fonts();
  for family in CANDIDATES {
    let id = database.query(&fontdb::Query {
      families: &[fontdb::Family::Name(family)],
      ..Default::default()
    });
    let Some(id) = id else {
      continue;
    };
    let (_, index) = database.face_source(id)?;
    let bytes = database.with_face_data(id, |data, _| data.to_vec())?;
    return Some(egui::FontData {
      font: bytes.into(),
      index,
      tweak: Default::default(),
    });
  }
  None
}

fn screen_image(device: &DefaultDevice) -> egui::ColorImage {
  let memory = device.graphic_memory();
  let foreground = Color32::from_rgb(42, 48, 45);
  let background = Color32::from_rgb(135, 150, 125);
  let mut image =
    egui::ColorImage::filled([SCREEN_WIDTH, SCREEN_HEIGHT], background);
  for y in 0..SCREEN_HEIGHT {
    for x in 0..SCREEN_WIDTH {
      let byte = memory[y * (SCREEN_WIDTH / 8) + x / 8];
      if byte & (0x80 >> (x & 7)) != 0 {
        image.pixels[y * SCREEN_WIDTH + x] = foreground;
      }
    }
  }
  image
}

fn input_label(field: &KeyboardInputType) -> String {
  match field {
    KeyboardInputType::String => "字符串".to_owned(),
    KeyboardInputType::Integer => "整数".to_owned(),
    KeyboardInputType::Real => "实数".to_owned(),
    KeyboardInputType::Func { name, param } => format!("函数 {name}({param})"),
  }
}

fn parse_inputs(
  vm: &VirtualMachine<DefaultDevice>,
  fields: &[KeyboardInputType],
  values: &[String],
) -> Result<Vec<KeyboardInput>, String> {
  fields
    .iter()
    .zip(values)
    .map(|(field, value)| match field {
      KeyboardInputType::Integer => value
        .parse::<i16>()
        .map(KeyboardInput::Integer)
        .map_err(|error| format!("整数格式错误: {error}")),
      KeyboardInputType::Real => value
        .parse::<f64>()
        .map_err(|error| format!("实数格式错误: {error}"))
        .and_then(|value| {
          value
            .try_into()
            .map(KeyboardInput::Real)
            .map_err(|error| format!("实数超出范围: {error:?}"))
        }),
      KeyboardInputType::String => {
        let utf16 = Utf16String::from(value.as_str());
        let (value, problems) = vm.byte_string_from_utf16str(&utf16);
        if problems.is_empty() {
          Ok(KeyboardInput::String(value))
        } else {
          Err(format!("字符串包含无法编码的字符: {:?}", problems[0]))
        }
      }
      KeyboardInputType::Func { .. } => {
        let utf16 = Utf16String::from(value.as_str());
        let input = Utf16Str::from_slice(utf16.as_slice())
          .map_err(|error| format!("UTF-16 错误: {error}"))?;
        let (body, diagnostics) = vm.compile_fn(input);
        diagnostics
          .iter()
          .find(|item| item.severity == Severity::Error)
          .map_or_else(
            || {
              body
                .map(|body| KeyboardInput::Func { body })
                .ok_or_else(|| "函数表达式为空".to_owned())
            },
            |item| Err(item.message.clone()),
          )
      }
    })
    .collect()
}

fn replace_all_ignore_ascii_case(
  source: &mut String,
  needle: &str,
  replacement: &str,
) -> usize {
  let upper_source = source.to_ascii_uppercase();
  let upper_needle = needle.to_ascii_uppercase();
  let mut matches = upper_source.match_indices(&upper_needle).peekable();
  if matches.peek().is_none() {
    return 0;
  }
  let mut result = String::with_capacity(source.len());
  let mut last = 0;
  let mut count = 0;
  for (start, matched) in matches {
    result.push_str(&source[last..start]);
    result.push_str(replacement);
    last = start + matched.len();
    count += 1;
  }
  result.push_str(&source[last..]);
  *source = result;
  count
}

fn byte_offset_for_utf16(source: &str, utf16_offset: usize) -> usize {
  let mut units = 0;
  for (byte, character) in source.char_indices() {
    if units >= utf16_offset {
      return byte;
    }
    units += character.len_utf16();
  }
  source.len()
}

fn wqx_key(key: egui::Key, modifiers: egui::Modifiers) -> Option<u8> {
  use egui::Key;
  let key = match key {
    Key::F1 => 28,
    Key::F2 => 29,
    Key::F3 => 30,
    Key::F4 => 31,
    Key::Enter => 13,
    Key::PageDown => 14,
    Key::PageUp => 19,
    Key::ArrowUp => 20,
    Key::ArrowDown => 21,
    Key::ArrowRight => 22,
    Key::ArrowLeft => 23,
    Key::Escape => 27,
    Key::Period => 46,
    Key::Space => 32,
    Key::Num0 => 48,
    Key::Num1 => 98,
    Key::Num2 => 110,
    Key::Num3 => 109,
    Key::Num4 => 103,
    Key::Num5 => 104,
    Key::Num6 => 106,
    Key::Num7 => 116,
    Key::Num8 => 121,
    Key::Num9 => 117,
    Key::A => 97,
    Key::B => 98,
    Key::C => 99,
    Key::D => 100,
    Key::E => 101,
    Key::F => 102,
    Key::G => 103,
    Key::H => 104,
    Key::I => 105,
    Key::J => 106,
    Key::K => 107,
    Key::L => 108,
    Key::M => 109,
    Key::N => 110,
    Key::O => 111,
    Key::P => 112,
    Key::Q => 113,
    Key::R => 114,
    Key::S => 115,
    Key::T => 116,
    Key::U => 117,
    Key::V => 118,
    Key::W => 119,
    Key::X => 120,
    Key::Y => 121,
    Key::Z => 122,
    Key::Backtick => 18,
    Key::ControlLeft | Key::ControlRight => 25,
    Key::ShiftLeft | Key::ShiftRight => 26,
    _ => return None,
  };
  if modifiers.ctrl && !matches!(key, 25) {
    return None;
  }
  Some(key)
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn replace_all_ascii_case_insensitive() {
    let mut source = "Print PRINT print".to_owned();
    assert_eq!(
      3,
      replace_all_ignore_ascii_case(&mut source, "print", "输出")
    );
    assert_eq!("输出 输出 输出", source);
  }

  #[test]
  fn utf16_offset_maps_to_utf8_boundary() {
    let source = "A你好𐀀B";
    assert_eq!(0, byte_offset_for_utf16(source, 0));
    assert_eq!(1, byte_offset_for_utf16(source, 1));
    assert_eq!(4, byte_offset_for_utf16(source, 2));
    assert_eq!(7, byte_offset_for_utf16(source, 3));
    assert_eq!(11, byte_offset_for_utf16(source, 5));
  }
}
