mod help;
mod legacy_icons;
mod model;
mod runtime;
mod search;
mod simulator_dialog;
mod syntax;

use std::{
  path::{Path, PathBuf},
  collections::BTreeMap,
  sync::Arc,
  time::Duration,
};

use gpui_kit::assets::IconName;
use gpui_kit::base::Disableable;
use gpui_kit::component::{
  Icon, Root, Sizable, Theme, ThemeMode,
  button::{Button, ButtonVariants},
  input::{
    Copy, Cut, Editor, EditorState, Input, InputEvent, InputState, Paste, Redo,
    SelectAll, Undo,
  },
  menu::{AppMenuBar, DropdownMenu, PopupMenuItem},
  resizable::{ResizableState, resizable_panel, v_resizable},
  scroll::ScrollableElement,
};
use gpui_kit::prelude::FluentBuilder;
use gpui_kit::*;
use gvb_interp::{Document, ExecResult, LabelTarget, ReplaceText, Severity};
use legacy_icons::{LegacyIcon, icon as legacy_icon};
use model::{Analysis, EditorModel, Problem};
use runtime::{
  DebugBinding, DebugValue, RunState, Simulator, input_label, screen_image,
};
use simulator_dialog::SimulatorDialog;

gpui_kit::actions!(
  wqxtools,
  [
    NewFile,
    OpenFile,
    SaveFile,
    SaveFileAs,
    RunProgram,
    StopProgram,
    FindText,
    ReplaceTextAction,
    ExitApp,
    EditUndo,
    EditRedo,
    EditCut,
    EditCopy,
    EditPaste,
    EditSelectAll,
    AddLabelAction,
    AddLabelNextAction,
    AddLabelPrevAction,
    RelabelAction,
    ShowSymbolsAction,
    ShowHelpAction,
    CloseHelpAction,
    LightThemeAction,
    DarkThemeAction,
    ToggleThemeAction
  ]
);

fn app_menus() -> Vec<Menu> {
  vec![
    Menu::new("文件").items([
      MenuItem::action("打开…", OpenFile),
      MenuItem::action("新建", NewFile),
      MenuItem::separator(),
      MenuItem::action("保存", SaveFile),
      MenuItem::action("另存为…", SaveFileAs),
      MenuItem::separator(),
      MenuItem::action("退出", ExitApp),
    ]),
    Menu::new("编辑").items([
      MenuItem::action("撤销", EditUndo),
      MenuItem::action("重做", EditRedo),
      MenuItem::separator(),
      MenuItem::action("复制", EditCopy),
      MenuItem::action("剪切", EditCut),
      MenuItem::action("粘贴", EditPaste),
      MenuItem::action("全选", EditSelectAll),
      MenuItem::separator(),
      MenuItem::action("查找", FindText),
      MenuItem::action("替换", ReplaceTextAction),
      MenuItem::separator(),
      MenuItem::action("为当前行加上行号", AddLabelAction),
      MenuItem::action("在下一行插入行号", AddLabelNextAction),
      MenuItem::action("在上一行插入行号", AddLabelPrevAction),
      MenuItem::action("重置行号", RelabelAction),
    ]),
    Menu::new("程序").items([
      MenuItem::action("运行", RunProgram),
      MenuItem::action("停止", StopProgram),
    ]),
    Menu::new("视图").items([
      MenuItem::action("亮色主题", LightThemeAction),
      MenuItem::action("暗色主题", DarkThemeAction),
    ]),
    Menu::new("帮助").items([MenuItem::action("GVBASIC 文档", ShowHelpAction)]),
  ]
}

fn editor_font(family: SharedString) -> Font {
  let mut font = font(family);
  font.fallbacks = Some(FontFallbacks::from_fonts(vec!["WenQuXing".into()]));
  font
}

#[cfg(target_os = "linux")]
fn zenity_initial_folder(directory: &Path) -> Option<String> {
  directory.to_str().map(|directory| format!("{directory}/"))
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Palette {
  bg: u32,
  panel: u32,
  surface: u32,
  text: u32,
  muted: u32,
  accent: u32,
  border: u32,
  error: u32,
  warning: u32,
  key_hover: u32,
  editor_bg: u32,
  overlay: u32,
}

impl Palette {
  fn for_mode(mode: ThemeMode) -> Self {
    match mode {
      ThemeMode::Dark => Self {
        bg: 0x11161d,
        panel: 0x1b222c,
        surface: 0x27313e,
        text: 0xe8edf3,
        muted: 0x9aa8b8,
        accent: 0x81b4ff,
        border: 0x344150,
        error: 0xf28b82,
        warning: 0xf3bd72,
        key_hover: 0x405064,
        editor_bg: 0x101820,
        overlay: 0x071018cc,
      },
      ThemeMode::Light => Self {
        bg: 0xf7f9fc,
        panel: 0xeaf0f6,
        surface: 0xffffff,
        text: 0x202b38,
        muted: 0x536475,
        accent: 0x1459a8,
        border: 0xcbd6e2,
        error: 0xb53030,
        warning: 0x916000,
        key_hover: 0xdbe9f8,
        editor_bg: 0xffffff,
        overlay: 0x21324799,
      },
    }
  }
}

fn configure_theme(mode: ThemeMode, cx: &mut App) {
  Theme::change(mode, None, cx);
  let p = Palette::for_mode(mode);
  let highlight = Arc::make_mut(&mut cx.global_mut::<Theme>().highlight_theme);
  highlight.style.editor_background = Some(rgb(p.editor_bg).into());
  highlight.style.editor_gutter_background = Some(rgb(p.panel).into());
}

fn alternate_theme(mode: ThemeMode) -> ThemeMode {
  match mode {
    ThemeMode::Dark => ThemeMode::Light,
    ThemeMode::Light => ThemeMode::Dark,
  }
}

fn theme_icon(mode: ThemeMode) -> IconName {
  match mode {
    ThemeMode::Dark => IconName::Sun,
    ThemeMode::Light => IconName::Moon,
  }
}

fn theme_toggle_label(mode: ThemeMode) -> &'static str {
  match mode {
    ThemeMode::Dark => "切换为亮色主题",
    ThemeMode::Light => "切换为暗色主题",
  }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum SimulatorPrimary {
  Run,
  Pause,
  Continue,
}

impl SimulatorPrimary {
  fn icon(self) -> LegacyIcon {
    match self {
      Self::Pause => LegacyIcon::Pause,
      Self::Run | Self::Continue => LegacyIcon::Run,
    }
  }

  fn label(self) -> &'static str {
    match self {
      Self::Run => "运行",
      Self::Pause => "暂停",
      Self::Continue => "继续",
    }
  }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct SimulatorControls {
  primary: SimulatorPrimary,
  stop_enabled: bool,
}

impl SimulatorControls {
  fn for_state(state: Option<RunState>) -> Self {
    match state {
      Some(RunState::Running | RunState::Sleeping | RunState::Waiting) => {
        Self {
          primary: SimulatorPrimary::Pause,
          stop_enabled: true,
        }
      }
      Some(RunState::Paused) => Self {
        primary: SimulatorPrimary::Continue,
        stop_enabled: true,
      },
      None | Some(RunState::Finished | RunState::Error) => Self {
        primary: SimulatorPrimary::Run,
        stop_enabled: false,
      },
    }
  }
}

fn key_labels(label: &str) -> (&str, Option<&str>) {
  match label.rsplit_once('/') {
    Some((primary, digit))
      if digit.len() == 1 && digit.as_bytes()[0].is_ascii_digit() =>
    {
      (primary, Some(digit))
    }
    _ => (label, None),
  }
}

const KEYS: &[&[(&str, u8)]] = &[
  &[("F1", 28), ("F2", 29), ("F3", 30), ("F4", 31)],
  &[
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
  &[
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
  &[
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
  &[
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

#[derive(Clone, Copy)]
enum PendingAction {
  New,
  Open,
  Close,
}

struct ArrayInspector {
  name: String,
  dimensions: Vec<u16>,
  axis: usize,
  indices: Vec<u16>,
  page: usize,
}

enum DebugTarget {
  Variable(String),
  ArrayCell { name: String, indices: Vec<u16> },
}

#[derive(Clone, Copy)]
enum EditorCommand {
  Undo,
  Redo,
  Cut,
  Copy,
  Paste,
  SelectAll,
}

struct DebugEdit {
  target: DebugTarget,
  input: Entity<InputState>,
  error: Option<String>,
}

struct Studio {
  menu_bar: Entity<AppMenuBar>,
  editor: Entity<EditorState>,
  theme_mode: ThemeMode,
  model: EditorModel,
  path: Option<PathBuf>,
  dirty: bool,
  errors: usize,
  warnings: usize,
  problems: Vec<Problem>,
  show_problems: bool,
  problem_split: Entity<ResizableState>,
  status: String,
  pending: Option<PendingAction>,
  show_relabel: bool,
  show_symbols: bool,
  show_help: bool,
  help: help::HelpBrowser,
  search: search::SearchPanel,
  relabel_start: Entity<InputState>,
  relabel_step: Entity<InputState>,
  simulator: Option<Simulator>,
  simulator_window: Option<WindowHandle<Root>>,
  simulator_close_subscription: Option<Subscription>,
  simulator_opening: bool,
  array_inspector: Option<ArrayInspector>,
  debug_edit: Option<DebugEdit>,
  simulator_focus: FocusHandle,
  screen: Arc<RenderImage>,
  input_fields: Vec<Entity<InputState>>,
  input_subscriptions: Vec<Subscription>,
  input_error: Option<String>,
  simulator_message: Option<String>,
  held_keys: BTreeMap<String, u8>,
  run_generation: u64,
  machine_names: Vec<String>,
  _subscription: Subscription,
  _editor_observation: Subscription,
  _editor_blur: Subscription,
}

impl Studio {
  fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
    let mut model = EditorModel::new();
    let initial = model.source().to_owned();
    let analysis = model.analyze(&initial);
    let editor = cx.new(|cx| {
      let mut state = EditorState::new(window, cx)
        .language("gvbasic")
        .searchable(false)
        .default_value(initial);
      state.set_highlighter_factory(syntax::factory(ThemeMode::Dark), cx);
      state
    });
    let subscription =
      cx.subscribe(&editor, |this, editor, event: &InputEvent, cx| {
        if matches!(event, InputEvent::Change) {
          let source = editor.read(cx).value().to_string();
          if source == this.model.source() {
            return;
          }
          let analysis = this.model.analyze(&source);
          this.dirty = true;
          this.status = "文档已修改".to_owned();
          this.apply_analysis(analysis, cx);
        }
      });
    let editor_observation = cx.observe(&editor, |_, _, cx| cx.notify());
    let editor_blur =
      cx.on_blur(&editor.read(cx).focus_handle(cx), window, |view, _, cx| {
        view
          .editor
          .update(cx, |editor, cx| editor.clear_diagnostic_popover(cx));
      });
    let problem_split = cx.new(|_| ResizableState::default());
    let relabel_start =
      cx.new(|cx| InputState::new(window, cx).default_value("10"));
    let relabel_step =
      cx.new(|cx| InputState::new(window, cx).default_value("10"));
    let help = help::HelpBrowser::new(cx);
    let search = search::SearchPanel::new(window, cx);
    let mut this = Self {
      menu_bar: AppMenuBar::new(cx),
      editor,
      theme_mode: ThemeMode::Dark,
      model,
      path: None,
      dirty: false,
      errors: 0,
      warnings: 0,
      problems: Vec::new(),
      show_problems: false,
      problem_split,
      status: "准备就绪".to_owned(),
      pending: None,
      show_relabel: false,
      show_symbols: false,
      show_help: false,
      help,
      search,
      relabel_start,
      relabel_step,
      simulator: None,
      simulator_window: None,
      simulator_close_subscription: None,
      simulator_opening: false,
      array_inspector: None,
      debug_edit: None,
      simulator_focus: cx.focus_handle(),
      screen: screen_image(
        &[0; runtime::SCREEN_WIDTH * runtime::SCREEN_HEIGHT / 8],
      ),
      input_fields: Vec::new(),
      input_subscriptions: Vec::new(),
      input_error: None,
      simulator_message: None,
      held_keys: BTreeMap::new(),
      run_generation: 0,
      machine_names: gvb_interp::machine::names()
        .map(ToString::to_string)
        .collect(),
      _subscription: subscription,
      _editor_observation: editor_observation,
      _editor_blur: editor_blur,
    };
    this.apply_analysis(analysis, cx);
    this
  }

  fn filename(&self) -> String {
    self
      .path
      .as_deref()
      .and_then(Path::file_name)
      .and_then(|name| name.to_str())
      .unwrap_or("未命名.bas.txt")
      .to_owned()
  }

  fn palette(&self) -> Palette {
    Palette::for_mode(self.theme_mode)
  }

  fn set_theme(&mut self, mode: ThemeMode, cx: &mut Context<Self>) {
    if self.theme_mode == mode {
      return;
    }
    self.theme_mode = mode;
    configure_theme(mode, cx);
    self.editor.update(cx, |editor, cx| {
      editor.set_highlighter_factory(syntax::factory(mode), cx);
    });
    cx.refresh_windows();
    cx.notify();
  }

  fn toggle_theme(&mut self, cx: &mut Context<Self>) {
    self.set_theme(alternate_theme(self.theme_mode), cx);
  }

  fn apply_analysis(&mut self, analysis: Analysis, cx: &mut Context<Self>) {
    self.errors = analysis.errors;
    self.warnings = analysis.warnings;
    self.problems = analysis.problems;
    self.editor.update(cx, |state, cx| {
      let diagnostics = state
        .diagnostics_mut()
        .expect("editor diagnostics initialized");
      diagnostics.clear();
      diagnostics.extend(analysis.diagnostics);
      cx.notify();
    });
    cx.notify();
  }

  fn replace_document(
    &mut self,
    document: Document,
    path: Option<PathBuf>,
    window: &mut Window,
    cx: &mut Context<Self>,
  ) {
    self.stop_simulator(cx);
    let analysis = self.model.replace_document(document);
    let source = self.model.source().to_owned();
    self
      .editor
      .update(cx, |editor, cx| editor.set_value(source, window, cx));
    self.path = path;
    self.dirty = false;
    self.pending = None;
    self.apply_analysis(analysis, cx);
    self
      .editor
      .update(cx, |editor, cx| editor.focus(window, cx));
  }

  fn request_new(&mut self, window: &mut Window, cx: &mut Context<Self>) {
    if self.dirty {
      self.pending = Some(PendingAction::New);
      cx.notify();
      return;
    }
    self.replace_document(Document::new(), None, window, cx);
    self.status = "已新建 GVBASIC 文档".to_owned();
  }

  fn request_open(&mut self, window: &mut Window, cx: &mut Context<Self>) {
    if self.dirty {
      self.pending = Some(PendingAction::Open);
      cx.notify();
      return;
    }
    self.open_document(window, cx);
  }

  fn open_document(&mut self, window: &mut Window, cx: &mut Context<Self>) {
    let directory = match std::env::current_dir() {
      Ok(directory) => directory,
      Err(error) => {
        self.status = format!("无法读取当前目录: {error}");
        cx.notify();
        return;
      }
    };
    let dialog = rfd::FileDialog::new()
      .add_filter("GVBASIC", &["bas", "txt"])
      .set_directory(&directory);
    // rfd's Linux zenity fallback ignores set_directory but passes file_name
    // through to zenity --filename. A trailing slash selects the folder.
    #[cfg(target_os = "linux")]
    let dialog = {
      let Some(initial_folder) = zenity_initial_folder(&directory) else {
        self.status = "当前目录路径不是 UTF-8，无法初始化文件对话框".to_owned();
        cx.notify();
        return;
      };
      dialog.set_file_name(initial_folder)
    };
    let Some(path) = dialog.pick_file() else {
      return;
    };
    match Document::load_file(&path) {
      Ok(document) => {
        self.replace_document(document, Some(path.clone()), window, cx);
        self.status = format!("已打开 {}", path.display());
      }
      Err(error) => {
        self.status = format!("打开失败: {error:?}");
        cx.notify();
      }
    }
  }

  fn save(&mut self, save_as: bool, cx: &mut Context<Self>) -> bool {
    let path = if save_as || self.path.is_none() {
      let Some(path) = rfd::FileDialog::new()
        .add_filter("GVBASIC 文本", &["txt"])
        .add_filter("GVBASIC 二进制", &["bas"])
        .set_file_name(self.filename())
        .save_file()
      else {
        return false;
      };
      path
    } else {
      self.path.clone().expect("existing path")
    };
    match self.model.document().save(&path) {
      Ok(()) => {
        self.path = Some(path.clone());
        self.dirty = false;
        self.status = format!("已保存 {}", path.display());
        cx.notify();
        true
      }
      Err(error) => {
        self.status = format!("保存失败: {error:?}");
        cx.notify();
        false
      }
    }
  }

  fn finish_pending(
    &mut self,
    save_first: bool,
    window: &mut Window,
    cx: &mut Context<Self>,
  ) {
    if save_first && !self.save(false, cx) {
      return;
    }
    match self.pending.take() {
      Some(PendingAction::New) => {
        self.replace_document(Document::new(), None, window, cx);
        self.status = "已新建 GVBASIC 文档".to_owned();
      }
      Some(PendingAction::Open) => self.open_document(window, cx),
      Some(PendingAction::Close) => {
        self.dirty = false;
        self.close_simulator_dialog(cx);
        window.remove_window();
      }
      None => {}
    }
    cx.notify();
  }

  fn request_exit(&mut self, window: &mut Window, cx: &mut Context<Self>) {
    if self.dirty {
      self.pending = Some(PendingAction::Close);
      cx.notify();
    } else {
      self.close_simulator_dialog(cx);
      window.remove_window();
    }
  }

  fn run(&mut self, window: &mut Window, cx: &mut Context<Self>) {
    if self.errors > 0 {
      self.status = "源码含错误，请先修复问题列表中的错误".to_owned();
      cx.notify();
      return;
    }
    self.stop_simulator(cx);
    match Simulator::start(self.model.document_mut()) {
      Ok(simulator) => {
        self.simulator_message = None;
        self.screen = simulator.screen();
        self.simulator = Some(simulator);
        self.status = "模拟器已启动".to_owned();
        if self.simulator_window.is_some() {
          self.refresh_runtime(window, cx);
        } else if !self.simulator_opening {
          self.open_simulator_dialog(window, cx);
        }
      }
      Err(error) => {
        if self.simulator_window.is_some() {
          self.simulator_message = Some(error);
        } else {
          self.status = error;
        }
        cx.notify();
      }
    }
  }

  fn open_simulator_dialog(
    &mut self,
    window: &mut Window,
    cx: &mut Context<Self>,
  ) {
    self.simulator_opening = true;
    let studio = cx.entity();
    window.defer(cx, move |_, cx| {
      let options = WindowOptions {
        kind: WindowKind::Dialog,
        window_bounds: Some(WindowBounds::Windowed(Bounds {
          origin: point(px(170.), px(65.)),
          size: size(px(740.), px(440.)),
        })),
        titlebar: Some(TitlebarOptions {
          title: Some("GVBASIC 模拟器".into()),
          ..Default::default()
        }),
        app_id: Some("wqxtools-gpui".to_owned()),
        ..Default::default()
      };
      let close_studio = studio.clone();
      let result = cx.open_window(options, |dialog_window, cx| {
        let dialog =
          cx.new(|cx| SimulatorDialog::new(studio.clone(), dialog_window, cx));
        dialog_window.on_window_should_close(cx, move |_, cx| {
          close_studio.update(cx, |view, cx| {
            view.simulator_window = None;
            view.simulator_close_subscription = None;
            view.simulator_opening = false;
            view.stop_simulator(cx);
          });
          true
        });
        cx.new(|cx| Root::new(dialog, dialog_window, cx))
      });
      match result {
        Ok(handle) => {
          let dialog_id = handle.window_id();
          let closed_studio = studio.clone();
          let close_subscription = cx.on_window_closed(move |cx, closed_id| {
            if closed_id == dialog_id {
              closed_studio.update(cx, |view, cx| {
                view.simulator_window = None;
                view.simulator_close_subscription = None;
                view.simulator_opening = false;
                view.stop_simulator(cx);
              });
            }
          });
          studio.update(cx, |view, cx| {
            view.simulator_window = Some(handle);
            view.simulator_close_subscription = Some(close_subscription);
            view.simulator_opening = false;
            cx.notify();
          });
          _ = handle.update(cx, |_, dialog_window, cx| {
            let focus = studio.read(cx).simulator_focus.clone();
            dialog_window.focus(&focus, cx);
            studio.update(cx, |view, cx| {
              view.refresh_runtime(dialog_window, cx);
            });
          });
        }
        Err(error) => studio.update(cx, |view, cx| {
          view.simulator_opening = false;
          view.stop_simulator(cx);
          view.status = format!("打开模拟器窗口失败: {error}");
          cx.notify();
        }),
      }
    });
  }

  fn close_simulator_dialog(&mut self, cx: &mut Context<Self>) {
    self.simulator_close_subscription = None;
    if let Some(handle) = self.simulator_window.take() {
      _ = handle.update(cx, |_, window, _| window.remove_window());
    }
    self.simulator_opening = false;
    self.stop_simulator(cx);
  }

  fn simulator_primary(&mut self, window: &mut Window, cx: &mut Context<Self>) {
    let state = self.simulator.as_ref().map(|simulator| simulator.state);
    match SimulatorControls::for_state(state).primary {
      SimulatorPrimary::Run => self.run(window, cx),
      SimulatorPrimary::Pause | SimulatorPrimary::Continue => {
        self.toggle_pause(window, cx)
      }
    }
  }

  fn stop_simulator_action(&mut self, cx: &mut Context<Self>) {
    let state = self.simulator.as_ref().map(|simulator| simulator.state);
    if SimulatorControls::for_state(state).stop_enabled {
      self.stop_simulator(cx);
    }
  }

  fn stop_simulator(&mut self, cx: &mut Context<Self>) {
    self.run_generation = self.run_generation.wrapping_add(1);
    if let Some(mut simulator) = self.simulator.take() {
      self.simulator_message = Some(match simulator.stop() {
        Ok(()) => "模拟器已停止".to_owned(),
        Err(error) => format!("停止模拟器失败: {error}"),
      });
    }
    self.input_fields.clear();
    self.input_subscriptions.clear();
    self.input_error = None;
    self.held_keys.clear();
    self.array_inspector = None;
    self.debug_edit = None;
    cx.notify();
  }

  fn refresh_runtime(&mut self, window: &mut Window, cx: &mut Context<Self>) {
    let Some(simulator) = self.simulator.as_mut() else {
      return;
    };
    self.screen = simulator.screen();
    let delay = if simulator.state == RunState::Paused {
      None
    } else {
      simulator.next_delay()
    };
    if let Some((_, fields)) = simulator.input_fields() {
      if self.input_fields.is_empty() {
        self.input_fields = fields
          .iter()
          .map(|_| cx.new(|cx| InputState::new(window, cx)))
          .collect();
        self.input_error = None;
        self.input_subscriptions = self
          .input_fields
          .iter()
          .map(|field| {
            cx.subscribe_in(
              field,
              window,
              |view, _, event: &InputEvent, window, cx| {
                if matches!(event, InputEvent::PressEnter { .. }) {
                  view.submit_input(window, cx);
                }
              },
            )
          })
          .collect();
        if let Some(first) = self.input_fields.first() {
          first.update(cx, |input, cx| input.focus(window, cx));
        }
      }
    } else {
      self.input_fields.clear();
      self.input_subscriptions.clear();
    }
    if let Some(delay) = delay {
      self.schedule_tick(delay, window, cx);
    }
    cx.notify();
  }

  fn schedule_tick(
    &self,
    delay: Duration,
    window: &mut Window,
    cx: &mut Context<Self>,
  ) {
    let generation = self.run_generation;
    cx.spawn_in(window, async move |this, cx| {
      cx.background_executor().timer(delay).await;
      _ = this.update_in(cx, |view, window, cx| {
        if view.run_generation != generation {
          return;
        }
        if let Some(simulator) = view.simulator.as_mut() {
          simulator.step();
          view.refresh_runtime(window, cx);
        }
      });
    })
    .detach();
  }

  fn simulator_key_down(
    &mut self,
    source: String,
    key: u8,
    window: &mut Window,
    cx: &mut Context<Self>,
  ) {
    if !self.input_fields.is_empty() || self.debug_edit.is_some() {
      return;
    }
    if self.held_keys.contains_key(&source) {
      return;
    }
    let already_down = self.held_keys.values().any(|&value| value == key);
    self.held_keys.insert(source, key);
    if let Some(simulator) = self.simulator.as_mut() {
      if already_down {
        return;
      }
      let was_in_key = matches!(simulator.result, ExecResult::InKey);
      simulator.key_down(key);
      if was_in_key {
        self.refresh_runtime(window, cx);
      }
    }
  }

  fn simulator_key_up(&mut self, source: &str) {
    if let Some(key) = self.held_keys.remove(source)
      && !self.held_keys.values().any(|&value| value == key)
      && let Some(simulator) = self.simulator.as_mut()
    {
      simulator.key_up(key);
    }
  }

  fn release_simulator_keys(&mut self) {
    if let Some(simulator) = self.simulator.as_mut() {
      for &key in self.held_keys.values() {
        simulator.key_up(key);
      }
    }
    self.held_keys.clear();
  }

  fn toggle_pause(&mut self, window: &mut Window, cx: &mut Context<Self>) {
    let Some(simulator) = self.simulator.as_mut() else {
      return;
    };
    if simulator.state == RunState::Paused {
      simulator.state = RunState::Running;
      self.simulator_message = None;
      self.refresh_runtime(window, cx);
    } else if matches!(
      simulator.state,
      RunState::Running | RunState::Sleeping | RunState::Waiting
    ) {
      self.run_generation = self.run_generation.wrapping_add(1);
      simulator.state = RunState::Paused;
      self.release_simulator_keys();
      cx.notify();
    }
  }

  fn submit_input(&mut self, window: &mut Window, cx: &mut Context<Self>) {
    let values: Vec<_> = self
      .input_fields
      .iter()
      .map(|field| field.read(cx).value().to_string())
      .collect();
    let Some(simulator) = self.simulator.as_mut() else {
      return;
    };
    match simulator.submit_inputs(&values) {
      Ok(()) => {
        self.input_fields.clear();
        self.input_subscriptions.clear();
        self.input_error = None;
        window.focus(&self.simulator_focus, cx);
        self.refresh_runtime(window, cx);
      }
      Err(error) => {
        self.input_error = Some(error);
        cx.notify();
      }
    }
  }

  fn inspect_array(
    &mut self,
    name: String,
    dimensions: Vec<u16>,
    cx: &mut Context<Self>,
  ) {
    self.array_inspector = Some(ArrayInspector {
      name,
      indices: vec![0; dimensions.len()],
      dimensions,
      axis: 0,
      page: 0,
    });
    cx.notify();
  }

  fn begin_debug_edit(
    &mut self,
    target: DebugTarget,
    value: DebugValue,
    window: &mut Window,
    cx: &mut Context<Self>,
  ) {
    if !matches!(
      self.simulator.as_ref().map(|sim| sim.state),
      Some(RunState::Paused)
    ) {
      self.simulator_message =
        Some("暂停模拟器后才能修改变量或数组".to_owned());
      cx.notify();
      return;
    }
    let input =
      cx.new(|cx| InputState::new(window, cx).default_value(value.display()));
    input.update(cx, |input, cx| input.focus(window, cx));
    self.debug_edit = Some(DebugEdit {
      target,
      input,
      error: None,
    });
    cx.notify();
  }

  fn apply_debug_edit(&mut self, cx: &mut Context<Self>) {
    let Some(edit) = &self.debug_edit else {
      return;
    };
    let value = edit.input.read(cx).value().to_string();
    let Some(simulator) = self.simulator.as_mut() else {
      self.debug_edit = None;
      return;
    };
    let result = match &edit.target {
      DebugTarget::Variable(name) => simulator.set_variable(name, &value),
      DebugTarget::ArrayCell { name, indices } => {
        simulator.set_array_value(name, indices, &value)
      }
    };
    match result {
      Ok(()) => {
        self.debug_edit = None;
        self.simulator_message = Some("调试值已更新".to_owned());
      }
      Err(error) => {
        if let Some(edit) = self.debug_edit.as_mut() {
          edit.error = Some(error);
        }
      }
    }
    cx.notify();
  }

  fn show_search(
    &mut self,
    replace: bool,
    window: &mut Window,
    cx: &mut Context<Self>,
  ) {
    self.search.open(replace, &self.editor, window, cx);
  }

  fn editor_command(
    &mut self,
    command: EditorCommand,
    window: &mut Window,
    cx: &mut Context<Self>,
  ) {
    self
      .editor
      .update(cx, |editor, cx| editor.focus(window, cx));
    match command {
      EditorCommand::Undo => window.dispatch_action(Box::new(Undo), cx),
      EditorCommand::Redo => window.dispatch_action(Box::new(Redo), cx),
      EditorCommand::Cut => window.dispatch_action(Box::new(Cut), cx),
      EditorCommand::Copy => window.dispatch_action(Box::new(Copy), cx),
      EditorCommand::Paste => window.dispatch_action(Box::new(Paste), cx),
      EditorCommand::SelectAll => {
        window.dispatch_action(Box::new(SelectAll), cx)
      }
    }
  }

  fn apply_edits(
    &mut self,
    edits: &[ReplaceText],
    window: &mut Window,
    cx: &mut Context<Self>,
  ) {
    let analysis = self.model.apply_replacements(edits);
    let source = self.model.source().to_owned();
    self
      .editor
      .update(cx, |editor, cx| editor.replace_all(source, window, cx));
    self.apply_analysis(analysis, cx);
    self.dirty = true;
    cx.notify();
  }

  fn add_label(
    &mut self,
    target: LabelTarget,
    window: &mut Window,
    cx: &mut Context<Self>,
  ) {
    let position = self.editor.read(cx).cursor_position();
    let offset = cursor_utf16_offset(self.model.source(), position);
    match self
      .model
      .document_mut()
      .compute_add_label_edit(target, offset)
    {
      Ok(result) => {
        self.apply_edits(&[result.edit], window, cx);
        if let Some(offset) = result.goto {
          let byte = utf16_to_byte(self.model.source(), offset);
          self.editor.update(cx, |editor, cx| {
            editor.set_selected_range(byte..byte, cx);
            editor.focus(window, cx);
          });
        }
        self.status = "已添加行号".to_owned();
      }
      Err(error) => {
        self.status = format!("添加行号失败: {error:?}");
        cx.notify();
      }
    }
  }

  fn relabel(&mut self, window: &mut Window, cx: &mut Context<Self>) {
    let start = self.relabel_start.read(cx).value().parse::<u16>();
    let step = self.relabel_step.read(cx).value().parse::<u16>();
    let (Ok(start), Ok(step)) = (start, step) else {
      self.status = "起始行号和步长必须是 0–65535 的整数".to_owned();
      cx.notify();
      return;
    };
    match self.model.document_mut().compute_relabel_edits(start, step) {
      Ok(edits) => {
        self.apply_edits(&edits, window, cx);
        self.show_relabel = false;
        self.status = "已重排行号及跳转引用".to_owned();
      }
      Err(error) => {
        self.status = format!("重排行号失败: {error:?}");
        cx.notify();
      }
    }
  }

  fn set_machine(
    &mut self,
    name: &str,
    window: &mut Window,
    cx: &mut Context<Self>,
  ) {
    let current = self.model.document().machine_name();
    if name == current {
      return;
    }
    match self.model.set_machine_name(name) {
      Ok(analysis) => {
        let source = self.model.source().to_owned();
        let first_line =
          source.lines().next().expect("document has a first line");
        let marker = first_line
          .rfind("{type:")
          .expect("machine selection writes a first-line type marker");
        let marker_column = first_line[..marker].chars().count() as u32;
        self.editor.update(cx, |editor, cx| {
          editor.replace_all(source, window, cx);
          editor.focus(window, cx);
          editor.set_cursor_position(
            gpui_kit::component::input::Position::new(0, marker_column),
            window,
            cx,
          );
        });
        self.apply_analysis(analysis, cx);
        self.dirty = true;
        self.status = format!("已切换机型为 {name}（注解在首行末尾）");
        cx.notify();
      }
      Err(error) => {
        self.status = format!("切换机型失败: {error:?}");
        cx.notify();
      }
    }
  }

  fn sync_machine(&mut self, window: &mut Window, cx: &mut Context<Self>) {
    let previous_source = self.model.source().to_owned();
    match self.model.document_mut().sync_machine_name() {
      Ok(_) => {
        let analysis = self.model.refresh_from_document();
        let source = self.model.source().to_owned();
        if source != previous_source {
          self.editor.update(cx, |editor, cx| {
            editor.replace_all(source, window, cx);
          });
          self.dirty = true;
        }
        self.apply_analysis(analysis, cx);
        self.status =
          format!("已从源码同步机型：{}", self.model.document().machine_name());
        cx.notify();
      }
      Err(error) => {
        self.status = format!("同步机型失败: {error:?}");
        cx.notify();
      }
    }
  }

  fn editor_button(
    &self,
    id: &'static str,
    label: &'static str,
    icon: LegacyIcon,
    command: EditorCommand,
    cx: &Context<Self>,
  ) -> Button {
    let view = cx.entity();
    Button::new(id)
      .ghost()
      .compact()
      .child(legacy_icon(icon))
      .tooltip(label)
      .accessibility_label(label)
      .on_click(move |_, window, cx| {
        view.update(cx, |view, cx| view.editor_command(command, window, cx));
      })
  }

  fn key_row(&self, row: usize, studio: &Entity<Self>) -> Div {
    let p = self.palette();
    let mut line = div().flex().gap(px(3.));
    if row == 0 {
      line = line.child(div().w(px(6. * 45. - 3.)).flex_shrink_0());
    }
    for (column, (label, code)) in KEYS[row].iter().enumerate() {
      let view = studio.clone();
      let release_view = studio.clone();
      let outside_view = studio.clone();
      let code = *code;
      let (primary, digit) = key_labels(label);
      let mut key = div()
        .id(format!("key-{row}-{column}"))
        .w(px(42.))
        .flex_shrink_0()
        .h(px(25.))
        .relative()
        .flex()
        .items_center()
        .justify_center()
        .rounded_sm()
        .bg(rgb(p.surface))
        .border_1()
        .border_color(rgb(p.border))
        .text_xs()
        .cursor_pointer()
        .hover(move |style| style.bg(rgb(p.key_hover)))
        .on_mouse_down(MouseButton::Left, move |_, window, cx| {
          view.update(cx, |view, cx| {
            window.focus(&view.simulator_focus, cx);
            view.simulator_key_down("pointer".to_owned(), code, window, cx);
          });
        })
        .on_mouse_up(MouseButton::Left, move |_, _, cx| {
          release_view.update(cx, |view, _| view.simulator_key_up("pointer"));
        })
        .on_mouse_up_out(MouseButton::Left, move |_, _, cx| {
          outside_view.update(cx, |view, _| view.simulator_key_up("pointer"));
        })
        .child(primary.to_owned());
      if let Some(digit) = digit {
        key = key.child(
          div()
            .absolute()
            .top(px(1.))
            .right(px(3.))
            .text_size(px(9.))
            .font_weight(FontWeight::SEMIBOLD)
            .text_color(rgb(p.accent))
            .child(digit.to_owned()),
        );
      }
      line = line.child(key);
    }
    line
  }
}

impl Render for Studio {
  fn render(
    &mut self,
    window: &mut Window,
    cx: &mut Context<Self>,
  ) -> impl IntoElement {
    let p = self.palette();
    let simulator_state =
      self.simulator.as_ref().map(|simulator| simulator.state);
    let simulator_controls = SimulatorControls::for_state(simulator_state);
    window.set_window_title(&format!(
      "WQX 工具箱 - {}{}",
      self.filename(),
      if self.dirty { "*" } else { "" }
    ));
    let mut toolbar = div()
      .h(px(42.))
      .flex()
      .items_center()
      .gap(px(4.))
      .px(px(7.))
      .bg(rgb(p.panel))
      .border_b_1()
      .border_color(rgb(p.border));
    let save_view = cx.entity();
    toolbar = toolbar.child(
      Button::new("save")
        .ghost()
        .compact()
        .child(legacy_icon(LegacyIcon::Save))
        .tooltip("保存 · Ctrl+S")
        .accessibility_label("保存")
        .on_click(move |_, _, cx| {
          save_view.update(cx, |view, cx| {
            view.save(false, cx);
          });
        }),
    );
    toolbar =
      toolbar.child(div().w(px(1.)).h(px(22.)).bg(rgb(p.border)).mx(px(4.)));
    let search_view = cx.entity();
    toolbar = toolbar.child(
      Button::new("find")
        .ghost()
        .compact()
        .child(legacy_icon(LegacyIcon::Find))
        .tooltip("查找 · Ctrl+F")
        .accessibility_label("查找")
        .on_click(move |_, window, cx| {
          search_view
            .update(cx, |view, cx| view.show_search(false, window, cx));
        }),
    );
    let replace_view = cx.entity();
    toolbar = toolbar.child(
      Button::new("replace")
        .ghost()
        .compact()
        .child(legacy_icon(LegacyIcon::Replace))
        .tooltip("替换 · Ctrl+R")
        .accessibility_label("替换")
        .on_click(move |_, window, cx| {
          replace_view
            .update(cx, |view, cx| view.show_search(true, window, cx));
        }),
    );
    toolbar = toolbar
      .child(div().w(px(1.)).h(px(22.)).bg(rgb(p.border)).mx(px(4.)))
      .child(self.editor_button(
        "undo",
        "撤销 · Ctrl+Z",
        LegacyIcon::Undo,
        EditorCommand::Undo,
        cx,
      ))
      .child(self.editor_button(
        "redo",
        "重做 · Ctrl+Y",
        LegacyIcon::Redo,
        EditorCommand::Redo,
        cx,
      ))
      .child(div().w(px(1.)).h(px(22.)).bg(rgb(p.border)).mx(px(4.)))
      .child(self.editor_button(
        "cut",
        "剪切 · Ctrl+X",
        LegacyIcon::Cut,
        EditorCommand::Cut,
        cx,
      ))
      .child(self.editor_button(
        "copy",
        "复制 · Ctrl+C",
        LegacyIcon::Copy,
        EditorCommand::Copy,
        cx,
      ))
      .child(self.editor_button(
        "paste",
        "粘贴 · Ctrl+V",
        LegacyIcon::Paste,
        EditorCommand::Paste,
        cx,
      ))
      .child(div().w(px(1.)).h(px(22.)).bg(rgb(p.border)).mx(px(4.)))
      .child(
        Button::new("run")
          .ghost()
          .compact()
          .child(legacy_icon(LegacyIcon::Simulator))
          .tooltip("启动模拟器 · F5")
          .accessibility_label("启动模拟器")
          .on_click(cx.listener(|view, _, window, cx| view.run(window, cx))),
      )
      .child(div().w(px(20.)))
      .child(
        Button::new("stop")
          .ghost()
          .compact()
          .disabled(!simulator_controls.stop_enabled)
          .child(legacy_icon(LegacyIcon::Stop).opacity(
            if simulator_controls.stop_enabled {
              1.0
            } else {
              0.3
            },
          ))
          .tooltip("停止模拟器")
          .accessibility_label("停止模拟器")
          .on_click(
            cx.listener(|view, _, _, cx| view.stop_simulator_action(cx)),
          ),
      )
      .child(div().flex_1());
    let machine_view = cx.entity();
    let machine_names = self.machine_names.clone();
    let current_machine = self.model.document().machine_name().to_owned();
    toolbar = toolbar.child(
      Button::new("symbols")
        .ghost()
        .compact()
        .child(legacy_icon(LegacyIcon::Emoji))
        .tooltip("文曲星符号")
        .accessibility_label("文曲星符号")
        .on_click(cx.listener(|view, _, _, cx| {
          view.show_symbols = true;
          cx.notify();
        })),
    );
    toolbar = toolbar.child(
      Button::new("theme-toggle")
        .ghost()
        .compact()
        .icon(theme_icon(self.theme_mode))
        .tooltip(theme_toggle_label(self.theme_mode))
        .accessibility_label(theme_toggle_label(self.theme_mode))
        .on_click(cx.listener(|view, _, _, cx| view.toggle_theme(cx))),
    );
    toolbar = toolbar
      .child(div().w(px(1.)).h(px(22.)).bg(rgb(p.border)).mx(px(4.)))
      .child(
        Button::new("machine")
          .ghost()
          .label(current_machine.clone())
          .dropdown_caret(true)
          .accessibility_label("选择机型")
          .dropdown_menu_with_anchor(Anchor::TopRight, move |menu, _, _| {
            machine_names.iter().fold(menu, |menu, name| {
              let view = machine_view.clone();
              let selected = name == &current_machine;
              let name = name.clone();
              menu.item(
                PopupMenuItem::new(name.clone()).checked(selected).on_click(
                  move |_, window, cx| {
                    view.update(cx, |view, cx| {
                      view.set_machine(&name, window, cx);
                    });
                  },
                ),
              )
            })
          }),
      )
      .child(
        Button::new("sync-machine")
          .ghost()
          .compact()
          .icon(IconName::RefreshCw)
          .tooltip("从源码同步机型")
          .accessibility_label("从源码同步机型")
          .on_click(cx.listener(|view, _, window, cx| {
            view.sync_machine(window, cx);
          })),
      );
    let editor_view = cx.entity();
    let editor_panel = div()
      .id("editor-panel")
      .size_full()
      .min_w_0()
      .min_h_0()
      .flex()
      .flex_col()
      .overflow_hidden()
      .on_hover(move |hovered, _, cx| {
        if !hovered {
          editor_view.update(cx, |view, cx| {
            view.editor.update(cx, |editor, cx| {
              editor.clear_diagnostic_popover(cx);
            });
          });
        }
      })
      .child(
        div().flex_1().min_h_0().child(
          Editor::new(&self.editor)
            .h_full()
            // The diagnostic popover is an empty flex sibling until painted.
            // The input's default 6px gap would shorten the search panel.
            .gap_0()
            .font(editor_font(cx.global::<Theme>().mono_font_family.clone())),
        ),
      );

    let mut problems = div()
      .size_full()
      .min_h_0()
      .flex()
      .flex_col()
      .bg(rgb(p.bg))
      .border_t_1()
      .border_color(rgb(p.border))
      .child(
        div()
          .h(px(27.))
          .flex()
          .items_center()
          .bg(rgb(p.surface))
          .border_b_1()
          .border_color(rgb(p.border))
          .text_xs()
          .text_color(rgb(p.muted))
          .child(div().w(px(38.)))
          .child(div().flex_1().child("问题"))
          .child(div().w(px(115.)).child("位置")),
      );
    let mut list = div().flex_1().min_h_0().overflow_y_scrollbar();
    if self.problems.is_empty() {
      list = list.child(
        div()
          .p(px(12.))
          .text_sm()
          .text_color(rgb(p.muted))
          .child("当前文档没有问题"),
      );
    }
    for (index, problem) in self.problems.iter().enumerate() {
      let view = cx.entity();
      let line = problem.line;
      let column = problem.column;
      let color = if problem.severity == Severity::Error {
        p.error
      } else {
        p.warning
      };
      let icon = if problem.severity == Severity::Error {
        IconName::CircleX
      } else {
        IconName::TriangleAlert
      };
      list = list.child(
        div()
          .id(format!("problem-{index}"))
          .h(px(32.))
          .flex()
          .items_center()
          .border_b_1()
          .border_color(rgb(p.border))
          .hover(|style| style.bg(rgb(p.surface)))
          .cursor_pointer()
          .on_click(move |_, window, cx| {
            view.update(cx, |view, cx| {
              view.editor.update(cx, |editor, cx| {
                editor.focus(window, cx);
                editor.set_cursor_position(
                  gpui_kit::component::input::Position::new(
                    (line - 1) as u32,
                    (column - 1) as u32,
                  ),
                  window,
                  cx,
                );
              });
            });
          })
          .child(
            div()
              .w(px(38.))
              .flex()
              .justify_center()
              .child(Icon::new(icon).text_color(rgb(color))),
          )
          .child(
            div()
              .flex_1()
              .min_w_0()
              .text_sm()
              .text_ellipsis()
              .child(problem.message.clone()),
          )
          .child(
            div()
              .w(px(115.))
              .text_xs()
              .text_color(rgb(p.muted))
              .child(format!("第 {} 行", problem.line)),
          ),
      );
    }
    problems = problems.child(list);

    // Keep the splitter mounted while the problem pane is hidden so a drag is
    // retained when the user expands it again.
    let work_area = v_resizable("source-problems")
      .with_state(&self.problem_split)
      .child(resizable_panel().child(editor_panel))
      .child(
        resizable_panel()
          .visible(self.show_problems)
          .size(px(220.))
          .size_range(px(120.)..px(450.))
          .child(problems),
      );

    let cursor = self.editor.read(cx).cursor_position();
    let toggle_problems = cx.entity();
    let problem_button = Button::new("toggle-problems")
      .ghost()
      .compact()
      .xsmall()
      .icon(if self.show_problems {
        IconName::ChevronDown
      } else {
        IconName::ChevronUp
      })
      .label(format!("{} 错误  /  {} 警告", self.errors, self.warnings))
      .tooltip(if self.show_problems {
        "收起问题列表"
      } else {
        "展开问题列表"
      })
      .on_click(move |_, _, cx| {
        toggle_problems.update(cx, |view, cx| {
          view.show_problems = !view.show_problems;
          cx.notify();
        });
      });

    let mut root =
      div()
        .size_full()
        .key_context(if self.show_help {
          "WqxStudioBlocked"
        } else {
          "WqxStudio"
        })
        .on_action(cx.listener(|view, _: &NewFile, window, cx| {
          view.request_new(window, cx)
        }))
        .on_action(cx.listener(|view, _: &OpenFile, window, cx| {
          view.request_open(window, cx)
        }))
        .on_action(cx.listener(|view, _: &SaveFile, _, cx| {
          view.save(false, cx);
        }))
        .on_action(cx.listener(|view, _: &SaveFileAs, _, cx| {
          view.save(true, cx);
        }))
        .on_action(
          cx.listener(|view, _: &RunProgram, window, cx| view.run(window, cx)),
        )
        .on_action(cx.listener(|view, _: &StopProgram, _, cx| {
          view.stop_simulator_action(cx)
        }))
        .on_action(cx.listener(|view, _: &FindText, window, cx| {
          view.show_search(false, window, cx)
        }))
        .on_action(cx.listener(|view, _: &ReplaceTextAction, window, cx| {
          view.show_search(true, window, cx)
        }))
        .on_action(cx.listener(|view, _: &ExitApp, window, cx| {
          view.request_exit(window, cx)
        }))
        .on_action(cx.listener(|view, _: &EditUndo, window, cx| {
          view.editor_command(EditorCommand::Undo, window, cx)
        }))
        .on_action(cx.listener(|view, _: &EditRedo, window, cx| {
          view.editor_command(EditorCommand::Redo, window, cx)
        }))
        .on_action(cx.listener(|view, _: &EditCut, window, cx| {
          view.editor_command(EditorCommand::Cut, window, cx)
        }))
        .on_action(cx.listener(|view, _: &EditCopy, window, cx| {
          view.editor_command(EditorCommand::Copy, window, cx)
        }))
        .on_action(cx.listener(|view, _: &EditPaste, window, cx| {
          view.editor_command(EditorCommand::Paste, window, cx)
        }))
        .on_action(cx.listener(|view, _: &EditSelectAll, window, cx| {
          view.editor_command(EditorCommand::SelectAll, window, cx)
        }))
        .on_action(cx.listener(|view, _: &AddLabelAction, window, cx| {
          view.add_label(LabelTarget::CurLine, window, cx)
        }))
        .on_action(cx.listener(|view, _: &AddLabelNextAction, window, cx| {
          view.add_label(LabelTarget::NextLine, window, cx)
        }))
        .on_action(cx.listener(|view, _: &AddLabelPrevAction, window, cx| {
          view.add_label(LabelTarget::PrevLine, window, cx)
        }))
        .on_action(cx.listener(|view, _: &RelabelAction, _, cx| {
          view.show_relabel = true;
          cx.notify();
        }))
        .on_action(cx.listener(|view, _: &ShowSymbolsAction, _, cx| {
          view.show_symbols = true;
          cx.notify();
        }))
        .on_action(cx.listener(|view, _: &ShowHelpAction, window, cx| {
          view.show_help = true;
          window.focus(&view.help.focus, cx);
          view
            .editor
            .update(cx, |editor, cx| editor.clear_diagnostic_popover(cx));
          cx.notify();
        }))
        .on_action(cx.listener(|view, _: &LightThemeAction, _, cx| {
          view.set_theme(ThemeMode::Light, cx);
        }))
        .on_action(cx.listener(|view, _: &DarkThemeAction, _, cx| {
          view.set_theme(ThemeMode::Dark, cx);
        }))
        .on_action(cx.listener(|view, _: &ToggleThemeAction, _, cx| {
          view.toggle_theme(cx);
        }))
        .flex()
        .flex_col()
        .bg(rgb(p.bg))
        .text_color(rgb(p.text))
        .child(
          div()
            .h(px(30.))
            .px(px(7.))
            .bg(rgb(p.panel))
            .border_b_1()
            .border_color(rgb(p.border))
            .child(self.menu_bar.clone()),
        )
        .child(toolbar)
        .when(self.search.visible, |root| {
          root.child(self.search.render(
            cx.entity(),
            p,
            self.model.source(),
            self.editor.read(cx).selected_range(),
            cx,
          ))
        })
        .child(div().flex_1().min_h_0().overflow_hidden().child(work_area))
        .child(
          div()
            .h(px(29.))
            .flex()
            .items_center()
            .px(px(12.))
            .bg(rgb(p.panel))
            .border_t_1()
            .border_color(rgb(p.border))
            .text_xs()
            .text_color(rgb(p.muted))
            .child(
              div()
                .flex_1()
                .min_w_0()
                .overflow_hidden()
                .whitespace_nowrap()
                .text_ellipsis()
                .child(self.status.clone()),
            )
            .child(problem_button)
            .child(div().w(px(1.)).h(px(17.)).bg(rgb(p.border)).mx(px(12.)))
            .child(format!(
              "第 {} 行，第 {} 列",
              cursor.line + 1,
              cursor.character + 1
            )),
        );

    if let Some(pending) = self.pending {
      let title = match pending {
        PendingAction::New => "新建文档前保存更改？",
        PendingAction::Open => "打开其他文件前保存更改？",
        PendingAction::Close => "关闭窗口前保存更改？",
      };
      let save_view = cx.entity();
      let discard_view = cx.entity();
      let cancel_view = cx.entity();
      root = root.child(
        div()
          .absolute()
          .size_full()
          .flex()
          .items_center()
          .justify_center()
          .bg(rgba(p.overlay))
          .child(
            div()
              .w(px(420.))
              .p(px(24.))
              .rounded_lg()
              .bg(rgb(p.surface))
              .border_1()
              .border_color(rgb(p.border))
              .flex()
              .flex_col()
              .gap(px(16.))
              .child(
                div()
                  .text_lg()
                  .font_weight(FontWeight::SEMIBOLD)
                  .child(title),
              )
              .child(
                div()
                  .text_sm()
                  .text_color(rgb(p.muted))
                  .child("当前编辑内容尚未保存。请选择保存、放弃更改或取消。"),
              )
              .child(
                div()
                  .flex()
                  .justify_end()
                  .gap(px(8.))
                  .child(
                    Button::new("cancel-pending")
                      .ghost()
                      .label("取消")
                      .on_click(move |_, _, cx| {
                        cancel_view.update(cx, |view, cx| {
                          view.pending = None;
                          cx.notify();
                        });
                      }),
                  )
                  .child(
                    Button::new("discard-pending")
                      .danger()
                      .label("放弃更改")
                      .on_click(move |_, window, cx| {
                        discard_view.update(cx, |view, cx| {
                          view.finish_pending(false, window, cx)
                        });
                      }),
                  )
                  .child(
                    Button::new("save-pending")
                      .primary()
                      .label("保存并继续")
                      .on_click(move |_, window, cx| {
                        save_view.update(cx, |view, cx| {
                          view.finish_pending(true, window, cx)
                        });
                      }),
                  ),
              ),
          ),
      );
    }
    if self.show_relabel {
      let cancel_view = cx.entity();
      let apply_view = cx.entity();
      root = root.child(
        div()
          .absolute()
          .size_full()
          .flex()
          .items_center()
          .justify_center()
          .bg(rgba(p.overlay))
          .child(
            div()
              .w(px(390.))
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
                  .text_lg()
                  .font_weight(FontWeight::SEMIBOLD)
                  .child("重排行号"),
              )
              .child(
                div()
                  .text_sm()
                  .text_color(rgb(p.muted))
                  .child("行号和 GOTO/GOSUB 引用会一起更新。"),
              )
              .child(
                div()
                  .flex()
                  .items_center()
                  .gap(px(10.))
                  .child(div().w(px(85.)).text_sm().child("起始行号"))
                  .child(div().flex_1().child(Input::new(&self.relabel_start))),
              )
              .child(
                div()
                  .flex()
                  .items_center()
                  .gap(px(10.))
                  .child(div().w(px(85.)).text_sm().child("步长"))
                  .child(div().flex_1().child(Input::new(&self.relabel_step))),
              )
              .child(
                div()
                  .flex()
                  .justify_end()
                  .gap(px(8.))
                  .child(
                    Button::new("cancel-relabel")
                      .ghost()
                      .label("取消")
                      .on_click(move |_, _, cx| {
                        cancel_view.update(cx, |view, cx| {
                          view.show_relabel = false;
                          cx.notify();
                        });
                      }),
                  )
                  .child(
                    Button::new("apply-relabel")
                      .primary()
                      .label("应用")
                      .on_click(move |_, window, cx| {
                        apply_view
                          .update(cx, |view, cx| view.relabel(window, cx));
                      }),
                  ),
              ),
          ),
      );
    }
    if self.show_symbols {
      let close_view = cx.entity();
      let mut grid = div()
        .flex_1()
        .min_h_0()
        .overflow_y_scrollbar()
        .flex()
        .flex_col()
        .gap(px(4.));
      for row_start in (0..527).step_by(16) {
        let mut row = div().flex().gap(px(3.));
        for offset in row_start..(row_start + 16).min(527) {
          let symbol =
            char::from_u32(0xe000 + offset).expect("private-use code point");
          let view = cx.entity();
          row = row.child(
            Button::new(format!("symbol-{offset}"))
              .secondary()
              .compact()
              .font_family("WenQuXing")
              .label(symbol.to_string())
              .on_click(move |_, _, cx| {
                cx.write_to_clipboard(ClipboardItem::new_string(
                  symbol.to_string(),
                ));
                view.update(cx, |view, cx| {
                  view.status =
                    format!("已复制文曲星符号 U+{:04X}", symbol as u32);
                  cx.notify();
                });
              }),
          );
        }
        grid = grid.child(row);
      }
      root = root.child(
        div()
          .absolute()
          .size_full()
          .flex()
          .items_center()
          .justify_center()
          .bg(rgba(p.overlay))
          .child(
            div()
              .w(px(650.))
              .h(px(540.))
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
                  .justify_between()
                  .items_center()
                  .child(
                    div()
                      .text_lg()
                      .font_weight(FontWeight::SEMIBOLD)
                      .child("文曲星图形符号"),
                  )
                  .child(
                    Button::new("close-symbols")
                      .ghost()
                      .label("关闭")
                      .on_click(move |_, _, cx| {
                        close_view.update(cx, |view, cx| {
                          view.show_symbols = false;
                          cx.notify();
                        });
                      }),
                  ),
              )
              .child(
                div()
                  .text_sm()
                  .text_color(rgb(p.muted))
                  .child("点击符号复制到剪贴板，可在编辑器中粘贴。"),
              )
              .child(grid),
          ),
      );
    }
    if self.show_help {
      root = root.child(self.help.render(cx.entity(), p));
    }
    root
  }
}

fn cursor_utf16_offset(
  source: &str,
  position: gpui_kit::component::input::Position,
) -> usize {
  let mut offset = 0;
  for (line_number, line) in source.split('\n').enumerate() {
    if line_number == position.line as usize {
      offset += line
        .chars()
        .take(position.character as usize)
        .map(char::len_utf16)
        .sum::<usize>();
      return offset;
    }
    offset += line.encode_utf16().count() + 1;
  }
  source.encode_utf16().count()
}

fn utf16_to_byte(source: &str, offset: usize) -> usize {
  let mut units = 0;
  for (byte, character) in source.char_indices() {
    if units >= offset {
      return byte;
    }
    units += character.len_utf16();
  }
  source.len()
}

fn hardware_key(key: &str) -> Option<u8> {
  match key {
    "f1" => Some(28),
    "f2" => Some(29),
    "f3" => Some(30),
    "f4" => Some(31),
    "enter" => Some(13),
    "escape" => Some(27),
    "capslock" | "caps_lock" | "~" | "`" => Some(18),
    "pageup" => Some(19),
    "pagedown" => Some(14),
    "up" => Some(20),
    "down" => Some(21),
    "right" => Some(22),
    "left" => Some(23),
    "space" => Some(32),
    "." => Some(46),
    "0" => Some(48),
    "1" => Some(98),
    "2" => Some(110),
    "3" => Some(109),
    "4" => Some(103),
    "5" => Some(104),
    "6" => Some(106),
    "7" => Some(116),
    "8" => Some(121),
    "9" => Some(117),
    _ if key.len() == 1 && key.as_bytes()[0].is_ascii_alphabetic() => {
      Some(key.as_bytes()[0].to_ascii_lowercase())
    }
    _ => None,
  }
}

fn main() {
  gvb_interp::machine::init_machines_from_str(include_str!(
    "../../gvb_interp/machines.yaml"
  ))
  .expect("bundled machines.yaml must be valid");
  gpui_kit::application()
    .with_assets(gpui_kit::assets::AllAssets)
    .run(|cx| {
      gpui_kit::init(cx);
      cx.set_menus(app_menus());
      gpui_kit::base::GlobalState::global_mut(cx)
        .set_app_menus(app_menus().into_iter().map(Menu::owned).collect());
      cx.text_system()
        .add_fonts(vec![std::borrow::Cow::Borrowed(
          include_bytes!("../../gui/assets/WenQuXing.ttf").as_slice(),
        )])
        .expect("bundled WenQuXing font must load");
      cx.bind_keys([
        KeyBinding::new("secondary-n", NewFile, Some("WqxStudio")),
        KeyBinding::new("secondary-o", OpenFile, Some("WqxStudio")),
        KeyBinding::new("secondary-s", SaveFile, Some("WqxStudio")),
        KeyBinding::new("secondary-shift-s", SaveFileAs, Some("WqxStudio")),
        KeyBinding::new("f5", RunProgram, Some("WqxStudio")),
        KeyBinding::new("shift-f5", StopProgram, Some("WqxStudio")),
        KeyBinding::new("f5", RunProgram, Some("WqxSimulator")),
        KeyBinding::new("f7", StopProgram, Some("WqxSimulator")),
        KeyBinding::new(
          "secondary-shift-t",
          ToggleThemeAction,
          Some("WqxStudio"),
        ),
        KeyBinding::new(
          "secondary-shift-t",
          ToggleThemeAction,
          Some("WqxSimulator"),
        ),
        KeyBinding::new("secondary-f", FindText, Some("WqxStudio")),
        KeyBinding::new("secondary-r", ReplaceTextAction, Some("WqxStudio")),
        KeyBinding::new("secondary-h", AddLabelAction, Some("WqxStudio")),
        KeyBinding::new("secondary-j", AddLabelNextAction, Some("WqxStudio")),
        KeyBinding::new("secondary-k", AddLabelPrevAction, Some("WqxStudio")),
        KeyBinding::new("f1", ShowHelpAction, Some("WqxStudio")),
        KeyBinding::new("escape", CloseHelpAction, Some("WqxHelp")),
      ]);
      configure_theme(ThemeMode::Dark, cx);
      cx.spawn(async move |cx| {
        let options = WindowOptions {
          window_bounds: Some(WindowBounds::Windowed(Bounds {
            origin: point(px(50.), px(35.)),
            size: size(px(900.), px(680.)),
          })),
          titlebar: Some(TitlebarOptions {
            title: Some("WQX 工具箱".into()),
            ..Default::default()
          }),
          app_id: Some("wqxtools-gpui".to_owned()),
          ..Default::default()
        };
        cx.open_window(options, |window, cx| {
          let view = cx.new(|cx| Studio::new(window, cx));
          let close_view = view.clone();
          window.on_window_should_close(cx, move |_, cx| {
            let dirty = close_view.read(cx).dirty;
            if dirty {
              close_view.update(cx, |view, cx| {
                view.pending = Some(PendingAction::Close);
                cx.notify();
              });
            } else {
              close_view.update(cx, |view, cx| {
                view.close_simulator_dialog(cx);
              });
            }
            !dirty
          });
          cx.new(|cx| Root::new(view, window, cx))
        })
        .expect("failed to open GPUI window");
      })
      .detach();
    });
}

#[cfg(test)]
mod tests {
  use super::{
    Palette, SimulatorControls, SimulatorPrimary, app_menus,
    cursor_utf16_offset, key_labels, hardware_key, utf16_to_byte,
  };
  use gpui_kit::component::ThemeMode;
  use gpui_kit::component::input::Position;
  use crate::runtime::RunState;

  #[test]
  fn simulator_hardware_keys_include_qt_input_method_keys() {
    assert_eq!(hardware_key("capslock"), Some(18));
    assert_eq!(hardware_key("caps_lock"), Some(18));
    assert_eq!(hardware_key("~"), Some(18));
    assert_eq!(hardware_key("`"), Some(18));
    assert_eq!(hardware_key("A"), hardware_key("a"));
    assert_eq!(hardware_key("1"), hardware_key("b"));
    assert_eq!(hardware_key("f4"), Some(31));
  }

  #[test]
  fn document_label_cursor_maps_back_to_utf8() {
    let source = "10 REM 中文😀\n20 ";
    assert_eq!(
      utf16_to_byte(source, "10 REM 中文😀\n".encode_utf16().count()),
      "10 REM 中文😀\n".len()
    );
    assert_eq!(
      utf16_to_byte(source, source.encode_utf16().count()),
      source.len()
    );
  }

  #[test]
  fn editor_scalar_cursor_maps_to_document_utf16_offset() {
    let source = "10 PRINT \"你😀\"\n20 END";
    assert_eq!(cursor_utf16_offset(source, Position::new(0, 12)), 13);
    assert_eq!(cursor_utf16_offset(source, Position::new(1, 2)), 17);
  }

  #[test]
  fn menu_bar_keeps_the_main_qt_sections() {
    let names: Vec<_> = app_menus().into_iter().map(|menu| menu.name).collect();
    assert_eq!(names, ["文件", "编辑", "程序", "视图", "帮助"]);
  }

  #[test]
  fn numeric_keyboard_legends_are_separate_from_primary_keys() {
    assert_eq!(key_labels("T/7"), ("T", Some("7")));
    assert_eq!(key_labels("符号/0"), ("符号", Some("0")));
    assert_eq!(key_labels("."), (".", None));
    assert_eq!(key_labels("上翻"), ("上翻", None));
  }

  #[test]
  fn both_themes_have_distinct_surfaces_and_legible_key_accents() {
    let dark = Palette::for_mode(ThemeMode::Dark);
    let light = Palette::for_mode(ThemeMode::Light);
    for p in [dark, light] {
      assert_ne!(p.bg, p.panel);
      assert_ne!(p.panel, p.surface);
      assert_ne!(p.text, p.muted);
      assert_ne!(p.accent, p.text);
    }
    assert_ne!(dark.editor_bg, light.editor_bg);
  }

  #[test]
  fn simulator_buttons_follow_program_state() {
    use RunState::*;
    let cases = [
      (None, SimulatorPrimary::Run, false),
      (Some(Running), SimulatorPrimary::Pause, true),
      (Some(Sleeping), SimulatorPrimary::Pause, true),
      (Some(Waiting), SimulatorPrimary::Pause, true),
      (Some(Paused), SimulatorPrimary::Continue, true),
      (Some(Finished), SimulatorPrimary::Run, false),
      (Some(Error), SimulatorPrimary::Run, false),
    ];
    for (state, primary, stop_enabled) in cases {
      assert_eq!(
        SimulatorControls::for_state(state),
        SimulatorControls {
          primary,
          stop_enabled,
        }
      );
    }
  }

  #[cfg(target_os = "linux")]
  #[test]
  fn zenity_open_dialog_starts_in_the_requested_folder() {
    assert_eq!(
      super::zenity_initial_folder(std::path::Path::new("/tmp/wqxtools")),
      Some("/tmp/wqxtools/".to_owned())
    );
  }
}
