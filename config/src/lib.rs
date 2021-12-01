use linked_hash_map::LinkedHashMap;
use std::io;
use util::config;
use yaml_rust::{Yaml, YamlLoader};

#[derive(Clone)]
pub struct Config {
  pub gvb: GvbConfig,
}

#[derive(Clone)]
pub struct GvbConfig {
  pub editor: GvbEditorConfig,
  pub simulator: GvbSimulatorConfig,
}

#[derive(Clone)]
pub struct GvbEditorConfig {
  pub font_size: u32,
}

#[derive(Clone)]
pub struct GvbSimulatorConfig {
  pub pixel_scale: u32,
  pub foreground: u32,
  pub background: u32,
}

const DEFAULT_CONFIG: Config = Config {
  gvb: GvbConfig {
    editor: GvbEditorConfig { font_size: 12 },
    simulator: GvbSimulatorConfig {
      pixel_scale: 2,
      foreground: 0x31_31_32,
      background: 0x7a_88_70,
    },
  },
};

#[derive(Debug)]
pub enum ConfigError {
  Io(io::Error),
  Yaml(yaml_rust::ScanError),
  Other(String),
}

impl From<io::Error> for ConfigError {
  fn from(err: io::Error) -> Self {
    Self::Io(err)
  }
}

impl From<yaml_rust::ScanError> for ConfigError {
  fn from(err: yaml_rust::ScanError) -> Self {
    Self::Yaml(err)
  }
}

impl From<String> for ConfigError {
  fn from(err: String) -> Self {
    Self::Other(err)
  }
}

impl From<&str> for ConfigError {
  fn from(err: &str) -> Self {
    Self::Other(err.to_owned())
  }
}

pub fn load_config() -> Result<Config, ConfigError> {
  let content = config::load_config_file("config.yaml")?;
  let mut docs = YamlLoader::load_from_str(&content)?;
  let mut config = DEFAULT_CONFIG.clone();
  if docs.is_empty() {
    return Ok(config);
  }

  let doc = docs.pop().unwrap();
  if doc.is_null() {
    return Ok(config);
  }

  let mut obj = doc.into_hash().ok_or_else(|| "toplevel is not object")?;

  // gvb
  if let Some(gvb) = obj.remove(&Yaml::String("gvb".to_owned())) {
    if !gvb.is_null() {
      let gvb = gvb.into_hash().ok_or_else(|| "gvb is not object")?;
      config.gvb = load_gvb_config(gvb)?;
    }
  }

  if let Some((key, _)) = obj.pop_front() {
    return Err(format!("superfluous field {}", yaml_to_string(&key)).into());
  }

  Ok(config)
}

fn load_gvb_config(
  mut gvb: LinkedHashMap<Yaml, Yaml>,
) -> Result<GvbConfig, ConfigError> {
  let mut gvb_config = DEFAULT_CONFIG.gvb.clone();

  // gvb.editor
  if let Some(editor) = gvb.remove(&Yaml::String("editor".into())) {
    if !editor.is_null() {
      let mut editor = editor
        .into_hash()
        .ok_or_else(|| "gvb.editor is not object")?;

      if let Some(font_size) = editor.remove(&Yaml::String("font-size".into()))
      {
        let font_size = font_size
          .into_i64()
          .ok_or_else(|| "gvb.editor.font-size is not integer")?;
        if font_size <= 0 {
          return Err("gvb.editor.font-size must be positive".into());
        }
        gvb_config.editor.font_size = font_size as u32;
      }

      if let Some((key, _)) = editor.pop_front() {
        return Err(
          format!("superfluous field {} in gvb.editor", yaml_to_string(&key))
            .into(),
        );
      }
    }
  }

  // gvb.simulator
  if let Some(simulator) = gvb.remove(&Yaml::String("simulator".into())) {
    if !simulator.is_null() {
      let mut simulator = simulator
        .into_hash()
        .ok_or_else(|| "gvb.simulator is not object")?;

      if let Some(pixel_scale) =
        simulator.remove(&Yaml::String("pixel-scale".into()))
      {
        let pixel_scale = pixel_scale
          .into_i64()
          .ok_or_else(|| "gvb.simulator.pixel-scale is not integer")?;
        if pixel_scale <= 0 {
          return Err("gvb.simulator.pixel-scale must be positive".into());
        }
        gvb_config.simulator.pixel_scale = pixel_scale as u32;
      }

      if let Some(c) = read_rgb(&mut simulator, "gvb.simulator", "foreground")?
      {
        gvb_config.simulator.foreground = c;
      }

      if let Some(c) = read_rgb(&mut simulator, "gvb.simulator", "background")?
      {
        gvb_config.simulator.background = c;
      }

      if let Some((key, _)) = simulator.pop_front() {
        return Err(
          format!(
            "superfluous field {} in gvb.simulator",
            yaml_to_string(&key)
          )
          .into(),
        );
      }
    }
  }

  if let Some((key, _)) = gvb.pop_front() {
    return Err(
      format!("superfluous field {} in gvb", yaml_to_string(&key)).into(),
    );
  }

  Ok(gvb_config)
}

fn read_rgb(
  obj: &mut LinkedHashMap<Yaml, Yaml>,
  ctx: impl AsRef<str>,
  name: impl ToString,
) -> Result<Option<u32>, ConfigError> {
  let ctx = ctx.as_ref();
  let name = name.to_string();

  if let Some(color) = obj.remove(&Yaml::String(name.clone())) {
    let color = color
      .into_string()
      .ok_or_else(|| format!("{}.{} is not string", ctx, name))?;
    if !color.starts_with('#') {
      return Err(format!("{}.{} is invalid color", ctx, name).into());
    }
    let color = &color[1..];
    if color.len() != 3 && color.len() != 6 {
      return Err(format!("{}.{} is invalid color", ctx, name).into());
    }
    match u32::from_str_radix(color, 16) {
      Ok(mut c) => {
        if color.len() == 3 {
          c = (c & 0xf) * 0x11 | (c & 0xf0) * 0x110 | (c & 0xf00) * 0x1100;
        }
        Ok(Some(c))
      }
      Err(_) => Err(format!("{}.{} is invalid color", ctx, name).into()),
    }
  } else {
    Ok(None)
  }
}

fn yaml_to_string(yaml: &Yaml) -> String {
  match yaml {
    Yaml::Null => "~".to_owned(),
    Yaml::Boolean(true) => "true".to_owned(),
    Yaml::Boolean(false) => "false".to_owned(),
    Yaml::Hash(_) => "<object>".to_owned(),
    Yaml::Array(_) => "<array>".to_owned(),
    Yaml::String(s) => format!("'{}'", s.replace("'", "\\'")),
    Yaml::Integer(n) => n.to_string(),
    Yaml::Real(n) => n.to_string(),
    _ => panic!(),
  }
}
