use std::{path::PathBuf, time::Duration};

use gpui_kit::RenderImage;
use gvb_interp::device::{Device, default::DefaultDevice};
use gvb_interp::{
  Binding, DimensionValues, Document, ExecInput, ExecResult, KeyboardInput,
  KeyboardInputType, Severity, Value, VirtualMachine,
};
use image::{Frame, RgbaImage};
use std::sync::Arc;
use widestring::{Utf16Str, Utf16String};

pub const SCREEN_WIDTH: usize = 160;
pub const SCREEN_HEIGHT: usize = 80;
pub const SCREEN_SCALE: usize = 2;
const EXEC_STEPS: usize = 4_000;
const FRAME_INTERVAL: Duration = Duration::from_millis(16);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RunState {
  Running,
  Sleeping,
  Waiting,
  Paused,
  Finished,
  Error,
}

impl RunState {
  pub fn label(self) -> &'static str {
    match self {
      Self::Running => "运行中",
      Self::Sleeping => "休眠",
      Self::Waiting => "等待输入",
      Self::Paused => "已暂停",
      Self::Finished => "已结束",
      Self::Error => "运行错误",
    }
  }
}

pub struct Simulator {
  vm: VirtualMachine<DefaultDevice>,
  pub state: RunState,
  pub result: ExecResult,
}

#[derive(Clone, Debug, PartialEq)]
pub enum DebugValue {
  Integer(i16),
  Real(f64),
  String(String),
}

impl DebugValue {
  pub fn display(&self) -> String {
    match self {
      Self::Integer(value) => value.to_string(),
      Self::Real(value) => value.to_string(),
      Self::String(value) => value.clone(),
    }
  }
}

pub enum DebugBinding {
  Variable { name: String, value: DebugValue },
  Array { name: String, dimensions: Vec<u16> },
}

impl Simulator {
  pub fn start(document: &mut Document) -> Result<Self, String> {
    Self::start_in_dir(document, PathBuf::from("dat_files"))
  }

  fn start_in_dir(
    document: &mut Document,
    data_dir: PathBuf,
  ) -> Result<Self, String> {
    std::fs::create_dir_all(&data_dir)
      .map_err(|error| format!("创建模拟器数据目录失败: {error}"))?;
    let device = document.create_device(data_dir);
    let mut vm = document
      .create_vm(device)
      .map_err(|_| "编译失败，请检查问题列表".to_owned())?;
    vm.device_mut().reset();
    vm.start();
    let result = vm.exec(ExecInput::None, EXEC_STEPS);
    Ok(Self {
      vm,
      state: RunState::Running,
      result,
    })
  }

  pub fn next_delay(&mut self) -> Option<Duration> {
    match &self.result {
      ExecResult::Continue => {
        self.state = RunState::Running;
        Some(FRAME_INTERVAL)
      }
      ExecResult::Sleep(duration) => {
        self.state = RunState::Sleeping;
        Some(*duration)
      }
      ExecResult::KeyboardInput { fields, .. } if fields.is_empty() => {
        Some(Duration::ZERO)
      }
      ExecResult::KeyboardInput { .. } | ExecResult::InKey => {
        self.state = RunState::Waiting;
        None
      }
      ExecResult::End => {
        self.state = RunState::Finished;
        None
      }
      ExecResult::Error { .. } => {
        self.state = RunState::Error;
        None
      }
    }
  }

  pub fn error_message(&self) -> Option<String> {
    match &self.result {
      ExecResult::Error { location, message } => {
        Some(format!("第 {} 行：{message}", location.line + 1))
      }
      _ => None,
    }
  }

  pub fn step(&mut self) {
    if self.state != RunState::Paused {
      let input = match &self.result {
        ExecResult::KeyboardInput { fields, .. } if fields.is_empty() => {
          ExecInput::KeyboardInput(Vec::new())
        }
        _ => ExecInput::None,
      };
      self.result = self.vm.exec(input, EXEC_STEPS);
    }
  }

  pub fn input_fields(
    &self,
  ) -> Option<(Option<String>, Vec<KeyboardInputType>)> {
    match &self.result {
      ExecResult::KeyboardInput { prompt, fields } if !fields.is_empty() => {
        Some((prompt.clone(), fields.clone()))
      }
      _ => None,
    }
  }

  pub fn submit_inputs(&mut self, values: &[String]) -> Result<(), String> {
    let Some((_, fields)) = self.input_fields() else {
      return Err("程序当前没有等待输入".to_owned());
    };
    if fields.len() != values.len() {
      return Err("输入字段数量不匹配".to_owned());
    }
    let inputs = parse_inputs(&self.vm, &fields, values)?;
    self.result = self.vm.exec(ExecInput::KeyboardInput(inputs), EXEC_STEPS);
    self.state = RunState::Running;
    Ok(())
  }

  pub fn key_down(&mut self, key: u8) {
    self.vm.device_mut().fire_key_down(key);
    if self.state != RunState::Paused
      && matches!(self.result, ExecResult::InKey)
      && let Some(actual) = self.vm.device_mut().key()
    {
      self.result = self.vm.exec(ExecInput::Key(actual), EXEC_STEPS);
      self.state = RunState::Running;
    }
  }

  pub fn key_up(&mut self, key: u8) {
    self.vm.device_mut().fire_key_up(key);
  }

  pub fn screen(&self) -> Arc<RenderImage> {
    screen_image(self.vm.device().graphic_memory())
  }

  pub fn stop(&mut self) -> Result<(), String> {
    self.vm.stop().map_err(|error| format!("{error:?}"))
  }

  pub fn bindings(&self) -> Vec<DebugBinding> {
    let mut bindings: Vec<_> = self
      .vm
      .bindings()
      .into_iter()
      .map(|(name, binding)| match binding {
        Binding::Var { value } => DebugBinding::Variable {
          name,
          value: self.debug_value(&value),
        },
        Binding::Array { dimensions } => {
          DebugBinding::Array { name, dimensions }
        }
      })
      .collect();
    bindings.sort_by(|left, right| {
      let left_name = match left {
        DebugBinding::Variable { name, .. }
        | DebugBinding::Array { name, .. } => name,
      };
      let right_name = match right {
        DebugBinding::Variable { name, .. }
        | DebugBinding::Array { name, .. } => name,
      };
      left_name.cmp(right_name)
    });
    bindings
  }

  pub fn array_values(
    &self,
    name: &str,
    indices: &[u16],
    axis: usize,
  ) -> Result<Vec<DebugValue>, String> {
    self.validate_array_position(name, indices, axis)?;
    let values = self.vm.arr_dimension_values(name, indices, axis);
    Ok(match values {
      DimensionValues::Integer(values) => {
        values.into_iter().map(DebugValue::Integer).collect()
      }
      DimensionValues::Real(values) => values
        .into_iter()
        .map(|value| DebugValue::Real(f64::from(value)))
        .collect(),
      DimensionValues::String(values) => values
        .iter()
        .map(|value| {
          DebugValue::String(
            self.vm.string_from_byte_string_lossy(value.clone()),
          )
        })
        .collect(),
    })
  }

  pub fn set_variable(
    &mut self,
    name: &str,
    input: &str,
  ) -> Result<(), String> {
    self.require_paused()?;
    let binding = self
      .vm
      .bindings()
      .remove(name)
      .ok_or_else(|| format!("变量 {name} 不存在"))?;
    let Binding::Var { value } = binding else {
      return Err(format!("{name} 是数组，不是变量"));
    };
    let next = self.parse_debug_value(&value, input)?;
    self.vm.modify_var(name, next);
    Ok(())
  }

  pub fn set_array_value(
    &mut self,
    name: &str,
    indices: &[u16],
    input: &str,
  ) -> Result<(), String> {
    self.require_paused()?;
    self.validate_array_position(name, indices, 0)?;
    let current = self.vm.arr_dimension_values(name, indices, 0);
    let template = match current {
      DimensionValues::Integer(_) => Value::Integer(0),
      DimensionValues::Real(_) => {
        Value::Real(0.0_f64.try_into().expect("zero is representable"))
      }
      DimensionValues::String(_) => Value::String(Default::default()),
    };
    let next = self.parse_debug_value(&template, input)?;
    self.vm.modify_arr(name, indices, next);
    Ok(())
  }

  fn validate_array_position(
    &self,
    name: &str,
    indices: &[u16],
    axis: usize,
  ) -> Result<(), String> {
    let binding = self
      .vm
      .bindings()
      .remove(name)
      .ok_or_else(|| format!("数组 {name} 不存在"))?;
    let Binding::Array { dimensions } = binding else {
      return Err(format!("{name} 不是数组"));
    };
    if indices.len() != dimensions.len() || axis >= dimensions.len() {
      return Err("数组维度不匹配".to_owned());
    }
    for (&index, &bound) in indices.iter().zip(&dimensions) {
      if index > bound {
        return Err(format!("数组索引 {index} 超出上界 {bound}"));
      }
    }
    Ok(())
  }

  fn require_paused(&self) -> Result<(), String> {
    if self.state == RunState::Paused {
      Ok(())
    } else {
      Err("暂停模拟器后才能修改变量或数组".to_owned())
    }
  }

  fn debug_value(&self, value: &Value) -> DebugValue {
    match value {
      Value::Integer(value) => DebugValue::Integer(*value),
      Value::Real(value) => DebugValue::Real(f64::from(*value)),
      Value::String(value) => {
        DebugValue::String(self.vm.string_from_byte_string_lossy(value.clone()))
      }
    }
  }

  fn parse_debug_value(
    &self,
    template: &Value,
    input: &str,
  ) -> Result<Value, String> {
    match template {
      Value::Integer(_) => input
        .parse::<i16>()
        .map(Value::Integer)
        .map_err(|error| format!("整数无效: {error}")),
      Value::Real(_) => {
        let number = input
          .parse::<f64>()
          .map_err(|error| format!("实数无效: {error}"))?;
        let number = number
          .try_into()
          .map_err(|error| format!("实数超出文曲星范围: {error:?}"))?;
        Ok(Value::Real(number))
      }
      Value::String(_) => {
        let utf16 = Utf16String::from(input);
        let (value, problems) = self.vm.byte_string_from_utf16str(&utf16);
        if let Some(problem) = problems.first() {
          return Err(format!("字符串包含无法编码的字符: {problem:?}"));
        }
        if value.len() > 255 {
          return Err("字符串超过 255 字节".to_owned());
        }
        Ok(Value::String(value))
      }
    }
  }
}

pub fn screen_image(memory: &[u8]) -> Arc<RenderImage> {
  assert_eq!(memory.len(), SCREEN_WIDTH * SCREEN_HEIGHT / 8);
  let width = SCREEN_WIDTH * SCREEN_SCALE;
  let height = SCREEN_HEIGHT * SCREEN_SCALE;
  let mut pixels = Vec::with_capacity(width * height * 4);
  // GPUI samples images linearly. Generate the exact display resolution with
  // nearest-neighbour pixels so the 2x LCD view has no blurred edges.
  for y in 0..height {
    for x in 0..width {
      let source_x = x / SCREEN_SCALE;
      let source_y = y / SCREEN_SCALE;
      let ink = memory[source_y * SCREEN_WIDTH / 8 + source_x / 8]
        & (0x80 >> (source_x & 7))
        != 0;
      pixels.extend_from_slice(if ink {
        &[42, 48, 45, 255]
      } else {
        &[135, 150, 125, 255]
      });
    }
  }
  let buffer = RgbaImage::from_raw(width as u32, height as u32, pixels)
    .expect("screen dimensions match pixel buffer");
  Arc::new(RenderImage::new(vec![Frame::new(buffer)]))
}

pub fn input_label(field: &KeyboardInputType) -> String {
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

#[cfg(test)]
mod tests {
  use super::*;

  fn program(source: &[u8]) -> Simulator {
    gvb_interp::machine::init_machines_from_str(include_str!(
      "../../gvb_interp/machines.yaml"
    ))
    .expect("bundled machines");
    let mut document = Document::load(source, false).expect("test source");
    Simulator::start(&mut document).expect("test program compiles")
  }

  #[test]
  fn checkkey_observes_hold_until_release() {
    let mut sim = program(b"10 B=CHECKKEY(20)\n20 GOTO 10");
    sim.key_down(20);
    sim.step();
    let Binding::Var { value } = &sim.vm.bindings()["B"] else {
      panic!("B is scalar");
    };
    assert_ne!(sim.debug_value(value), DebugValue::Real(0.0));
    sim.key_up(20);
    sim.step();
    let Binding::Var { value } = &sim.vm.bindings()["B"] else {
      panic!("B is scalar");
    };
    assert_eq!(sim.debug_value(value), DebugValue::Real(0.0));
  }

  #[test]
  fn invalid_input_keeps_waiting_and_can_be_corrected() {
    let mut sim = program(b"10 INPUT A\n20 INPUT B\n30 END");
    sim.next_delay();
    assert!(sim.submit_inputs(&[String::new()]).is_err());
    assert_eq!(sim.state, RunState::Waiting);
    assert!(sim.input_fields().is_some());
    sim.submit_inputs(&["12".to_owned()]).expect("first input");
    assert!(sim.input_fields().is_some());
    sim.submit_inputs(&["34".to_owned()]).expect("second input");
    assert!(matches!(sim.result, ExecResult::End));
  }

  #[test]
  fn key_input_does_not_resume_a_paused_program() {
    let mut sim = program(b"10 K$=INKEY$\n20 END");
    assert!(matches!(sim.result, ExecResult::InKey));
    sim.state = RunState::Paused;
    sim.key_down(20);
    sim.key_up(20);
    assert_eq!(sim.state, RunState::Paused);
    assert!(matches!(sim.result, ExecResult::InKey));
  }

  fn advance_until_input_or_end(simulator: &mut Simulator, program: &str) {
    for _ in 0..20_000 {
      match simulator.result {
        ExecResult::Continue | ExecResult::Sleep(_) => simulator.step(),
        ExecResult::KeyboardInput { ref fields, .. } if fields.is_empty() => {
          simulator.step()
        }
        _ => return,
      }
    }
    panic!(
      "{program} did not reach input, end, or error: {:?}",
      simulator.result
    );
  }

  #[test]
  fn empire_requires_its_installation_data_and_boots_after_installing() {
    gvb_interp::machine::init_machines_from_str(include_str!(
      "../../gvb_interp/machines.yaml"
    ))
    .expect("bundled machines must be valid");
    let examples = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
      .join("../examples/bas/远古帝国");
    let data_dir = tempfile::tempdir().expect("isolated simulator data");
    let mut game = Document::load_file(examples.join("远古帝国.bas"))
      .expect("bundled game must load");
    let mut missing =
      Simulator::start_in_dir(&mut game, data_dir.path().to_path_buf())
        .expect("bundled game must compile");
    advance_until_input_or_end(&mut missing, "game without data");
    let error = missing.error_message().expect("missing setup must fail");
    assert!(error.contains("EmpireDATA.DAT"), "{error}");
    assert!(error.contains("文件不存在"), "{error}");

    for installer in ["远古帝国-安装A.txt", "远古帝国-安装B.txt"] {
      let mut document = Document::load_file(examples.join(installer))
        .expect("bundled installer must load");
      let mut simulator =
        Simulator::start_in_dir(&mut document, data_dir.path().to_path_buf())
          .unwrap_or_else(|error| {
            panic!(
              "{installer} must compile: {error}; diagnostics: {:?}",
              document
                .diagnostics()
                .iter()
                .enumerate()
                .filter(|(_, line)| !line.diagnostics.is_empty())
                .collect::<Vec<_>>()
            )
          });
      advance_until_input_or_end(&mut simulator, installer);
      assert!(
        matches!(simulator.result, ExecResult::End),
        "{installer} did not finish: {:?}",
        simulator.result
      );
    }
    assert!(data_dir.path().join("EmpireDATA.DAT").exists());
    assert!(data_dir.path().join("EmpireDATB.DAT").exists());

    let mut game = Document::load_file(examples.join("远古帝国.bas"))
      .expect("bundled game must reload");
    let mut simulator =
      Simulator::start_in_dir(&mut game, data_dir.path().to_path_buf())
        .expect("bundled game must compile after installing");
    for _ in 0..100 {
      assert!(
        simulator.error_message().is_none(),
        "game boot failed: {:?}",
        simulator.result
      );
      simulator.step();
    }
    assert!(
      matches!(simulator.result, ExecResult::Sleep(_)),
      "game did not reach its idle loop: {:?}",
      simulator.result
    );
  }

  fn debug_simulator() -> Simulator {
    gvb_interp::machine::init_machines_from_str(include_str!(
      "../../gvb_interp/machines.yaml"
    ))
    .expect("bundled machines must be valid");
    let mut document = Document::load(
      b"10 A%=4:R=1.5:S$=\"abc\":DIM B%(2,1):B%(1,1)=8\n20 END",
      false,
    )
    .expect("test program must load");
    let simulator =
      Simulator::start(&mut document).expect("test program must compile");
    assert!(matches!(simulator.result, ExecResult::End));
    simulator
  }

  #[test]
  fn screen_pixels_follow_device_bitmap() {
    let mut bits = [0; SCREEN_WIDTH * SCREEN_HEIGHT / 8];
    bits[0] = 0x80;
    let image = screen_image(&bits);
    let bytes = image.as_bytes(0).expect("one frame");
    assert_eq!(&bytes[..4], &[42, 48, 45, 255]);
    assert_eq!(&bytes[4..8], &[42, 48, 45, 255]);
    assert_eq!(&bytes[8..12], &[135, 150, 125, 255]);
    let second_row = SCREEN_WIDTH * SCREEN_SCALE * 4;
    assert_eq!(&bytes[second_row..second_row + 4], &[42, 48, 45, 255]);
  }

  #[test]
  fn debugger_edits_require_pause_and_validate_array_bounds() {
    let mut simulator = debug_simulator();
    assert!(simulator.set_variable("A%", "9").is_err());
    simulator.state = RunState::Paused;
    simulator.set_variable("A%", "9").expect("integer edit");
    assert!(simulator.set_variable("A%", "99999").is_err());
    simulator.set_variable("R", "2.5").expect("real edit");
    assert!(simulator.set_variable("R", "NaN").is_err());
    simulator.set_variable("S$", "new").expect("string edit");
    assert!(simulator.set_variable("S$", "😀").is_err());
    assert!(simulator.set_variable("missing", "1").is_err());
    assert!(simulator.array_values("B%", &[0], 0).is_err());
    assert!(simulator.array_values("B%", &[3, 0], 0).is_err());
    assert!(simulator.array_values("B%", &[0, 0], 2).is_err());
    simulator
      .set_array_value("B%", &[1, 1], "12")
      .expect("array edit");
    let values = simulator.array_values("B%", &[0, 1], 0).expect("array row");
    assert_eq!(values[1], DebugValue::Integer(12));
    assert!(simulator.set_array_value("B%", &[3, 0], "1").is_err());
  }
}
