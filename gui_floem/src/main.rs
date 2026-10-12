use std::{
  borrow::Cow,
  cell::{Cell, RefCell},
  io::Cursor,
  ops::Range,
  path::{Path, PathBuf},
  rc::Rc,
  time::Duration,
};

use floem::{
  action::exec_after,
  peniko::Color,
  prelude::*,
  reactive::{RwSignal, SignalGet, SignalUpdate, create_rw_signal},
  style::Style,
  text::{Attrs, AttrsList, FamilyOwned},
  views::editor::{
    EditorStyle,
    core::{
      buffer::rope_text::RopeText, editor::EditType, selection::Selection,
    },
    id::EditorId,
    layout::{LineExtraStyle, TextLayoutLine},
    text::{
      Document as EditorDocument, Styling, WrapMethod, default_dark_color,
    },
  },
  views::{dyn_container, img},
};
use gvb_interp::{
  Document, Edit, EditKind, ExecInput, ExecResult, Severity, VirtualMachine,
};
use gvb_interp::device::default::DefaultDevice;
use image::{DynamicImage, ImageFormat, RgbaImage};
use widestring::Utf16String;

const ACCENT: Color = Color::rgb8(0x66, 0xda, 0xb7);
const TEXT: Color = Color::rgb8(0xe5, 0xee, 0xef);
const MUTED: Color = Color::rgb8(0x9a, 0xac, 0xb7);
const SURFACE: Color = Color::rgb8(0x17, 0x25, 0x31);
const BACKGROUND: Color = Color::rgb8(0x0d, 0x17, 0x21);
const SCREEN_WIDTH: usize = 160;
const SCREEN_HEIGHT: usize = 80;
const EXEC_STEPS: usize = 4_000;
const FRAME_INTERVAL: Duration = Duration::from_millis(16);

const KEYWORDS: &[&str] = &[
  "AND", "BEEP", "BOX", "CALL", "CHECKKEY", "CIRCLE", "CLEAR", "CLOSE", "CLS",
  "COLOR", "DATA", "DEF", "DIM", "DRAW", "ELSE", "END", "FIELD", "FOR",
  "FPUTC", "FREAD", "FSEEK", "FWRITE", "GET", "GOSUB", "GOTO", "IF", "INPUT",
  "LET", "LINE", "LOAD", "LOCATE", "LSET", "NEXT", "NOT", "ON", "OPEN", "OR",
  "PAINT", "PLAY", "POINT", "POKE", "POP", "PRINT", "PUT", "READ", "REM",
  "RESTORE", "RETURN", "RSET", "RUN", "SCREEN", "SET", "SLEEP", "STOP", "SWAP",
  "THEN", "TO", "USING", "WEND", "WHILE", "WRITE", "XOR",
];

#[derive(Clone)]
struct UiDiagnostic {
  line: usize,
  column: usize,
  range: Range<usize>,
  severity: Severity,
  message: String,
}

struct EditorState {
  document: Document,
  source_synced: String,
  path: Option<PathBuf>,
  dirty: bool,
  diagnostics: Vec<UiDiagnostic>,
}

impl EditorState {
  fn new() -> Self {
    let document = Document::new();
    let source_synced = document.text().to_string();
    let mut state = Self {
      document,
      source_synced,
      path: None,
      dirty: false,
      diagnostics: Vec::new(),
    };
    state.refresh_diagnostics();
    state
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

  fn sync_source(&mut self, source: &str) {
    if source == self.source_synced {
      return;
    }
    let old_len = self.document.text().len();
    if old_len > 0 {
      self.document.apply_edit(Edit {
        pos: 0,
        kind: EditKind::Delete(old_len),
      });
    }
    let utf16 = Utf16String::from(source);
    if !utf16.is_empty() {
      self.document.apply_edit(Edit {
        pos: 0,
        kind: EditKind::Insert(&utf16),
      });
    }
    self.source_synced = source.to_owned();
    self.dirty = true;
    self.refresh_diagnostics();
  }

  fn refresh_diagnostics(&mut self) {
    self.diagnostics = self
      .document
      .diagnostics()
      .iter()
      .enumerate()
      .flat_map(|(line, result)| {
        result.diagnostics.iter().map(move |item| UiDiagnostic {
          line,
          column: item.range.start + 1,
          range: item.range.start..item.range.end,
          severity: item.severity,
          message: item.message.clone(),
        })
      })
      .collect();
  }
}

struct Simulator {
  vm: VirtualMachine<DefaultDevice>,
}

#[derive(Clone)]
struct RuntimeUi {
  simulator: Rc<RefCell<Option<Simulator>>>,
  generation: Rc<Cell<u64>>,
  screen_png: RwSignal<Vec<u8>>,
  run_state: RwSignal<String>,
  status: RwSignal<String>,
}

fn screen_rgba(memory: &[u8]) -> Vec<u8> {
  assert_eq!(memory.len(), SCREEN_WIDTH * SCREEN_HEIGHT / 8);
  let mut pixels = Vec::with_capacity(SCREEN_WIDTH * SCREEN_HEIGHT * 4);
  for y in 0..SCREEN_HEIGHT {
    for x in 0..SCREEN_WIDTH {
      let ink = memory[y * SCREEN_WIDTH / 8 + x / 8] & (0x80 >> (x & 7)) != 0;
      let color = if ink {
        [42, 48, 45, 255]
      } else {
        [135, 150, 125, 255]
      };
      pixels.extend_from_slice(&color);
    }
  }
  pixels
}

fn screen_png(memory: &[u8]) -> Result<Vec<u8>, image::ImageError> {
  let image = RgbaImage::from_raw(
    SCREEN_WIDTH as u32,
    SCREEN_HEIGHT as u32,
    screen_rgba(memory),
  )
  .expect("screen dimensions and pixel count must agree");
  let mut output = Cursor::new(Vec::new());
  DynamicImage::ImageRgba8(image).write_to(&mut output, ImageFormat::Png)?;
  Ok(output.into_inner())
}

fn schedule_step(
  ui: RuntimeUi,
  generation: u64,
  input: ExecInput,
  delay: Duration,
) {
  exec_after(delay, move |_| run_step(ui, generation, input));
}

fn run_step(ui: RuntimeUi, generation: u64, input: ExecInput) {
  if ui.generation.get() != generation {
    return;
  }
  let next = {
    let mut current = ui.simulator.borrow_mut();
    let Some(simulator) = current.as_mut() else {
      return;
    };
    let result = simulator.vm.exec(input, EXEC_STEPS);
    if simulator.vm.device_mut().take_dirty_area().is_some() {
      match screen_png(simulator.vm.device().graphic_memory()) {
        Ok(png) => ui.screen_png.set(png),
        Err(error) => {
          ui.status.set(format!("更新模拟器屏幕失败: {error}"));
          ui.run_state.set("屏幕错误".to_owned());
          return;
        }
      }
    }
    match result {
      ExecResult::Continue => {
        ui.run_state.set("运行中".to_owned());
        Some((ExecInput::None, FRAME_INTERVAL))
      }
      ExecResult::Sleep(duration) => {
        ui.run_state.set("休眠".to_owned());
        Some((ExecInput::None, duration))
      }
      ExecResult::KeyboardInput { fields, .. } if fields.is_empty() => {
        Some((ExecInput::KeyboardInput(Vec::new()), Duration::ZERO))
      }
      ExecResult::InKey | ExecResult::KeyboardInput { .. } => {
        ui.run_state.set("等待输入".to_owned());
        ui.status
          .set("程序等待输入；键盘与输入对话框尚未迁移".to_owned());
        None
      }
      ExecResult::End => {
        ui.run_state.set("已结束".to_owned());
        ui.status.set("程序运行结束".to_owned());
        None
      }
      ExecResult::Error { message, .. } => {
        ui.run_state.set("运行错误".to_owned());
        ui.status.set(format!("运行时错误: {message}"));
        None
      }
    }
  };
  if let Some((input, delay)) = next {
    schedule_step(ui, generation, input, delay);
  }
}

fn stop_simulator(ui: &RuntimeUi) -> Result<(), String> {
  ui.generation.set(ui.generation.get().wrapping_add(1));
  if let Some(mut simulator) = ui.simulator.borrow_mut().take() {
    simulator.vm.stop().map_err(|error| format!("{error:?}"))?;
  }
  ui.run_state.set("已停止".to_owned());
  Ok(())
}

fn byte_offset_for_utf16(source: &str, utf16_offset: usize) -> usize {
  let mut seen = 0;
  for (byte, ch) in source.char_indices() {
    if seen + ch.len_utf16() > utf16_offset {
      return byte;
    }
    seen += ch.len_utf16();
  }
  source.len()
}

fn basic_spans(line: &str) -> Vec<(Range<usize>, Color)> {
  let mut spans = Vec::new();
  let mut offset = 0;
  while offset < line.len() {
    let rest = &line[offset..];
    let first = rest.chars().next().expect("non-empty remainder");
    if first == '\'' {
      spans.push((offset..line.len(), MUTED));
      break;
    }
    if first == '"' {
      let mut end = 1;
      let mut chars = rest[1..].char_indices().peekable();
      while let Some((index, ch)) = chars.next() {
        end = index + 1 + ch.len_utf8();
        if ch == '"' {
          if let Some((_, '"')) = chars.peek() {
            let (_, escaped) = chars.next().expect("peeked quote");
            end += escaped.len_utf8();
          } else {
            break;
          }
        }
      }
      spans.push((offset..offset + end, Color::rgb8(0xe6, 0xbd, 0x83)));
      offset += end;
      continue;
    }
    if first.is_ascii_alphabetic() {
      let end = rest
        .char_indices()
        .find(|(_, ch)| !ch.is_ascii_alphanumeric() && *ch != '$' && *ch != '_')
        .map_or(rest.len(), |(index, _)| index);
      let word = rest[..end].to_ascii_uppercase();
      if word == "REM" {
        spans.push((offset..line.len(), MUTED));
        break;
      }
      if KEYWORDS.contains(&word.as_str()) {
        spans.push((offset..offset + end, ACCENT));
      }
      offset += end;
      continue;
    }
    if first.is_ascii_digit() {
      let end = rest
        .char_indices()
        .find(|(_, ch)| !ch.is_ascii_digit() && *ch != '.')
        .map_or(rest.len(), |(index, _)| index);
      spans.push((offset..offset + end, Color::rgb8(0xd5, 0xa5, 0xd7)));
      offset += end;
      continue;
    }
    offset += first.len_utf8();
  }
  spans
}

struct BasicStyling {
  document: Rc<dyn EditorDocument>,
  diagnostics: Rc<RefCell<Vec<UiDiagnostic>>>,
  revision: Rc<Cell<u64>>,
}

impl Styling for BasicStyling {
  fn id(&self) -> u64 {
    self.revision.get()
  }

  fn font_size(&self, _editor: EditorId, _line: usize) -> usize {
    15
  }

  fn line_height(&self, _editor: EditorId, _line: usize) -> f32 {
    24.0
  }

  fn font_family(
    &self,
    _editor: EditorId,
    _line: usize,
  ) -> Cow<'_, [FamilyOwned]> {
    Cow::Owned(vec![
      FamilyOwned::Name("Noto Sans Mono".into()),
      FamilyOwned::Monospace,
    ])
  }

  fn apply_attr_styles(
    &self,
    _editor: EditorId,
    _style: &EditorStyle,
    line: usize,
    default: Attrs,
    attrs: &mut AttrsList,
  ) {
    attrs.clear_spans();
    let content = self.document.rope_text().line_content(line).to_string();
    for (range, color) in basic_spans(&content) {
      attrs.add_span(range, default.color(color));
    }
  }

  fn apply_layout_styles(
    &self,
    _editor: EditorId,
    _style: &EditorStyle,
    line: usize,
    layout: &mut TextLayoutLine,
  ) {
    let content = self.document.rope_text().line_content(line).to_string();
    for diagnostic in self
      .diagnostics
      .borrow()
      .iter()
      .filter(|item| item.line == line)
    {
      let start = byte_offset_for_utf16(&content, diagnostic.range.start);
      let end = byte_offset_for_utf16(&content, diagnostic.range.end);
      let start_position = layout.text.hit_position(start);
      let end_position = layout.text.hit_position(end);
      let color = match diagnostic.severity {
        Severity::Error => Color::rgb8(0xf7, 0x7f, 0x7d),
        Severity::Warning => Color::rgb8(0xed, 0xbe, 0x67),
      };
      layout.extra_style.push(LineExtraStyle {
        x: start_position.point.x,
        y: start_position.point.y,
        width: Some((end_position.point.x - start_position.point.x).max(5.0)),
        height: 18.0,
        bg_color: None,
        under_line: None,
        wave_line: Some(color),
      });
    }
  }
}

fn replace_editor_text(document: &Rc<dyn EditorDocument>, text: &str) {
  document.edit_single(
    Selection::region(0, document.text().len()),
    text,
    EditType::Other,
  );
}

fn toolbar_button_style(style: Style) -> Style {
  style
    .background(Color::rgb8(0x24, 0x3a, 0x46))
    .color(TEXT)
    .border(1.0)
    .border_color(Color::rgb8(0x38, 0x54, 0x5e))
    .border_radius(6.0)
    .padding_horiz(12.0)
    .padding_vert(7.0)
    .hover(|style| style.background(Color::rgb8(0x2e, 0x50, 0x59)))
    .active(|style| style.background(Color::rgb8(0x1c, 0x6d, 0x63)))
    .focus(|style| {
      style
        .background(Color::rgb8(0x24, 0x3a, 0x46))
        .color(TEXT)
        .hover(|style| style.background(Color::rgb8(0x2e, 0x50, 0x59)))
    })
}

fn app_view() -> impl IntoView {
  let state = Rc::new(RefCell::new(EditorState::new()));
  let diagnostics = Rc::new(RefCell::new(state.borrow().diagnostics.clone()));
  let revision = Rc::new(Cell::new(1_u64));
  let filename = create_rw_signal(state.borrow().filename());
  let dirty = create_rw_signal(false);
  let has_path = create_rw_signal(false);
  let status = create_rw_signal("准备就绪".to_owned());
  let runtime = RuntimeUi {
    simulator: Rc::new(RefCell::new(None)),
    generation: Rc::new(Cell::new(0)),
    screen_png: create_rw_signal(
      screen_png(&[0; SCREEN_WIDTH * SCREEN_HEIGHT / 8])
        .expect("blank screen must encode as PNG"),
    ),
    run_state: create_rw_signal("未启动".to_owned()),
    status,
  };
  let problem_count = create_rw_signal(state.borrow().diagnostics.len());
  let first_problem =
    create_rw_signal(state.borrow().diagnostics.first().map(|item| {
      format!("{}:{}  {}", item.line + 1, item.column, item.message)
    }));

  let editor = text_editor(state.borrow().source_synced.clone());
  let document = editor.doc();
  let styling = BasicStyling {
    document: document.clone(),
    diagnostics: diagnostics.clone(),
    revision: revision.clone(),
  };

  let update_state = state.clone();
  let update_document = document.clone();
  let update_diagnostics = diagnostics.clone();
  let update_revision = revision.clone();
  let editor = editor
    .styling(styling)
    .update(move |_| {
      let source = update_document.text().to_string();
      let mut current = update_state.borrow_mut();
      if source == current.source_synced {
        return;
      }
      current.sync_source(&source);
      update_diagnostics.replace(current.diagnostics.clone());
      update_revision.set(update_revision.get().wrapping_add(1));
      dirty.set(current.dirty);
      problem_count.set(current.diagnostics.len());
      first_problem.set(current.diagnostics.first().map(|item| {
        format!("{}:{}  {}", item.line + 1, item.column, item.message)
      }));
      status.set("文档已修改".to_owned());
    })
    .editor_style(|style| {
      default_dark_color(style).wrap_method(WrapMethod::None)
    })
    .style(|style| style.height_full().flex_grow(1.0_f32).min_width(0.0));

  let new_state = state.clone();
  let new_document = document.clone();
  let new_diagnostics = diagnostics.clone();
  let new_revision = revision.clone();
  let new_button = button(label(|| "新建"))
    .on_click_stop(move |_| {
      if new_state.borrow().dirty
        || new_document.text().to_string() != new_state.borrow().source_synced
      {
        status.set("当前文档未保存；请先保存，再新建".to_owned());
        return;
      }
      let next = EditorState::new();
      let source = next.source_synced.clone();
      new_diagnostics.replace(next.diagnostics.clone());
      problem_count.set(next.diagnostics.len());
      first_problem.set(next.diagnostics.first().map(|item| {
        format!("{}:{}  {}", item.line + 1, item.column, item.message)
      }));
      *new_state.borrow_mut() = next;
      replace_editor_text(&new_document, &source);
      new_revision.set(new_revision.get().wrapping_add(1));
      filename.set("未命名.bas.txt".to_owned());
      dirty.set(false);
      has_path.set(false);
      status.set("已新建 GVBASIC 文档".to_owned());
    })
    .style(toolbar_button_style);

  let open_state = state.clone();
  let open_document = document.clone();
  let open_diagnostics = diagnostics.clone();
  let open_revision = revision.clone();
  let open_button = button(label(|| "打开…"))
    .on_click_stop(move |_| {
      if open_state.borrow().dirty
        || open_document.text().to_string() != open_state.borrow().source_synced
      {
        status.set("当前文档未保存；请先保存，再打开其他文件".to_owned());
        return;
      }
      let Some(path) = rfd::FileDialog::new()
        .add_filter("GVBASIC", &["bas", "txt"])
        .pick_file()
      else {
        return;
      };
      match Document::load_file(&path) {
        Ok(loaded) => {
          let source = loaded.text().to_string();
          let mut next = EditorState {
            document: loaded,
            source_synced: source.clone(),
            path: Some(path.clone()),
            dirty: false,
            diagnostics: Vec::new(),
          };
          next.refresh_diagnostics();
          open_diagnostics.replace(next.diagnostics.clone());
          problem_count.set(next.diagnostics.len());
          first_problem.set(next.diagnostics.first().map(|item| {
            format!("{}:{}  {}", item.line + 1, item.column, item.message)
          }));
          filename.set(next.filename());
          *open_state.borrow_mut() = next;
          replace_editor_text(&open_document, &source);
          open_revision.set(open_revision.get().wrapping_add(1));
          dirty.set(false);
          has_path.set(true);
          status.set(format!("已打开 {}", path.display()));
        }
        Err(error) => status.set(format!("打开失败: {error:?}")),
      }
    })
    .style(toolbar_button_style);

  let save_state = state.clone();
  let save_document = document.clone();
  let save_button = button(label(|| "保存"))
    .on_click_stop(move |_| {
      let source = save_document.text().to_string();
      let mut current = save_state.borrow_mut();
      current.sync_source(&source);
      let path = match current.path.clone() {
        Some(path) => path,
        None => {
          let Some(path) = rfd::FileDialog::new()
            .add_filter("GVBASIC 文本", &["txt"])
            .add_filter("GVBASIC 二进制", &["bas"])
            .set_file_name("program.bas.txt")
            .save_file()
          else {
            return;
          };
          path
        }
      };
      match current.document.save(&path) {
        Ok(()) => {
          current.path = Some(path.clone());
          current.dirty = false;
          filename.set(current.filename());
          dirty.set(false);
          has_path.set(true);
          status.set(format!("已保存 {}", path.display()));
        }
        Err(error) => status.set(format!("保存失败: {error:?}")),
      }
    })
    .style(toolbar_button_style);

  let run_state = state.clone();
  let run_document = document.clone();
  let run_runtime = runtime.clone();
  let run_button = button(label(|| "▶ 运行"))
    .on_click_stop(move |_| {
      let source = run_document.text().to_string();
      let mut current = run_state.borrow_mut();
      current.sync_source(&source);
      current.refresh_diagnostics();
      if current
        .diagnostics
        .iter()
        .any(|item| item.severity == Severity::Error)
      {
        status.set("源码含错误，修复后才能运行".to_owned());
        return;
      }
      let data_dir = PathBuf::from("dat_files");
      if let Err(error) = std::fs::create_dir_all(&data_dir) {
        status.set(format!("创建数据目录失败: {error}"));
        return;
      }
      let device = current.document.create_device(data_dir);
      let Ok(mut vm) = current.document.create_vm(device) else {
        status.set("编译失败，请检查问题列表".to_owned());
        return;
      };
      if let Err(error) = stop_simulator(&run_runtime) {
        status.set(format!("停止旧程序失败: {error}"));
        return;
      }
      vm.device_mut().reset();
      vm.start();
      match screen_png(vm.device().graphic_memory()) {
        Ok(png) => run_runtime.screen_png.set(png),
        Err(error) => {
          status.set(format!("初始化模拟器屏幕失败: {error}"));
          return;
        }
      }
      *run_runtime.simulator.borrow_mut() = Some(Simulator { vm });
      let generation = run_runtime.generation.get().wrapping_add(1);
      run_runtime.generation.set(generation);
      run_runtime.run_state.set("运行中".to_owned());
      status.set("模拟器已启动".to_owned());
      schedule_step(
        run_runtime.clone(),
        generation,
        ExecInput::None,
        Duration::ZERO,
      );
    })
    .style(|style| {
      toolbar_button_style(style)
        .background(Color::rgb8(0x1c, 0x6d, 0x63))
        .border_color(ACCENT)
    });

  let stop_runtime = runtime.clone();
  let stop_button = button(label(|| "■ 停止"))
    .on_click_stop(move |_| match stop_simulator(&stop_runtime) {
      Ok(()) => status.set("模拟器已停止".to_owned()),
      Err(error) => status.set(format!("停止模拟器失败: {error}")),
    })
    .style(toolbar_button_style);

  let screen = dyn_container(
    move || runtime.screen_png.get(),
    |png| {
      img(move || png.clone()).style(|style| {
        style
          .width(480.0)
          .height(240.0)
          .border(2.0)
          .border_color(Color::rgb8(0x39, 0x4c, 0x4a))
      })
    },
  );

  v_stack((
    h_stack((
      label(|| "WQX")
        .style(|style| style.font_size(20.0).font_bold().color(ACCENT)),
      label(|| "TOOLS")
        .style(|style| style.font_size(12.0).font_bold().color(MUTED)),
      new_button,
      open_button,
      save_button,
      run_button,
      stop_button,
      label(move || filename.get())
        .style(|style| style.font_size(12.0).color(MUTED)),
    ))
    .style(|style| {
      style
        .width_full()
        .gap(12.0)
        .padding(14.0)
        .background(SURFACE)
    }),
    h_stack((
      label(|| "GVBASIC 编辑器")
        .style(|style| style.font_size(16.0).font_bold().color(TEXT)),
      label(move || {
        if dirty.get() {
          "● 未保存"
        } else if has_path.get() {
          "● 已保存"
        } else {
          "● 未修改"
        }
      })
      .style(|style| style.font_size(12.0).color(MUTED)),
    ))
    .style(|style| {
      style
        .width_full()
        .gap(12.0)
        .padding(12.0)
        .background(BACKGROUND)
    }),
    h_stack((
      editor,
      v_stack((
        h_stack((
          label(|| "文曲星模拟器")
            .style(|style| style.font_size(15.0).font_bold().color(TEXT)),
          label(move || runtime.run_state.get())
            .style(|style| style.font_size(12.0).color(ACCENT)),
        ))
        .style(|style| style.gap(10.0)),
        screen,
        label(|| "160 × 80 · GVBASIC")
          .style(|style| style.font_size(11.0).color(MUTED)),
      ))
      .style(|style| {
        style
          .width(512.0)
          .gap(12.0)
          .padding(16.0)
          .background(SURFACE)
      }),
    ))
    .style(|style| style.width_full().height_full().background(BACKGROUND)),
    v_stack((
      label(move || format!("问题  {}", problem_count.get()))
        .style(|style| style.font_size(13.0).font_bold().color(TEXT)),
      label(move || {
        first_problem
          .get()
          .unwrap_or_else(|| "当前文档没有诊断问题".to_owned())
      })
      .style(|style| style.font_size(12.0).color(MUTED)),
    ))
    .style(|style| {
      style
        .width_full()
        .gap(4.0)
        .padding(12.0)
        .background(SURFACE)
    }),
    label(move || status.get()).style(|style| {
      style
        .width_full()
        .font_size(12.0)
        .color(MUTED)
        .padding(8.0)
        .background(BACKGROUND)
    }),
  ))
  .style(|style| style.size_full().background(BACKGROUND))
}

fn main() {
  gvb_interp::machine::init_machines_from_str(include_str!(
    "../../gvb_interp/machines.yaml"
  ))
  .expect("bundled machines.yaml must be valid");
  floem::launch(app_view);
}

#[cfg(test)]
mod tests {
  use super::{
    EditorState, SCREEN_HEIGHT, SCREEN_WIDTH, basic_spans,
    byte_offset_for_utf16, screen_png, screen_rgba,
  };
  use gvb_interp::{ExecInput, ExecResult};

  #[test]
  fn chinese_utf16_offsets_map_to_utf8_boundaries() {
    let line = "PRINT \"你好😀\"";
    assert_eq!(byte_offset_for_utf16(line, 7), 7);
    assert_eq!(byte_offset_for_utf16(line, 9), 13);
    assert_eq!(byte_offset_for_utf16(line, 10), 13);
    assert_eq!(byte_offset_for_utf16(line, 11), 17);
  }

  #[test]
  fn quote_and_comment_spans_do_not_overlap() {
    let source = "PRINT \"REM ' \"\"你好\"\"\" ' 注释";
    let spans = basic_spans(source);
    assert_eq!(&source[spans[0].0.clone()], "PRINT");
    assert_eq!(&source[spans[1].0.clone()], "\"REM ' \"\"你好\"\"\"");
    assert_eq!(&source[spans[2].0.clone()], "' 注释");
  }

  #[test]
  fn editing_source_refreshes_real_gvb_diagnostics() {
    gvb_interp::machine::init_machines_from_str(include_str!(
      "../../gvb_interp/machines.yaml"
    ))
    .expect("bundled machine definitions");
    let mut state = EditorState::new();
    assert!(!state.diagnostics.is_empty());
    state.sync_source("10 PRINT \"你好\"\n20 END");
    assert!(state.diagnostics.is_empty());
    assert!(state.dirty);
  }

  #[test]
  fn screen_bits_are_rendered_and_encoded_at_native_resolution() {
    let mut memory = vec![0; SCREEN_WIDTH * SCREEN_HEIGHT / 8];
    memory[0] = 0b1000_0001;
    let pixels = screen_rgba(&memory);
    assert_eq!(&pixels[0..4], &[42, 48, 45, 255]);
    assert_eq!(&pixels[4..8], &[135, 150, 125, 255]);
    assert_eq!(&pixels[28..32], &[42, 48, 45, 255]);
    let png = screen_png(&memory).expect("screen PNG");
    let image = image::load_from_memory(&png).expect("decode screen PNG");
    assert_eq!((image.width(), image.height()), (160, 80));
  }

  #[test]
  fn real_vm_prints_to_graphic_memory() {
    gvb_interp::machine::init_machines_from_str(include_str!(
      "../../gvb_interp/machines.yaml"
    ))
    .expect("bundled machine definitions");
    let mut state = EditorState::new();
    state.sync_source("10 PRINT \"HELLO\"\n20 END");
    assert!(state.diagnostics.is_empty());
    let device = state.document.create_device("dat_files");
    let Ok(mut vm) = state.document.create_vm(device) else {
      panic!("valid program must compile");
    };
    vm.device_mut().reset();
    vm.start();
    let result = vm.exec(ExecInput::None, 4_000);
    assert!(matches!(result, ExecResult::End | ExecResult::Continue));
    assert!(vm.device().graphic_memory().iter().any(|&byte| byte != 0));
  }
}
