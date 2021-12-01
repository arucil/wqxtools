use crate::{Either, Unit, Utf8String};
use std::mem::MaybeUninit;

#[repr(C)]
pub struct Config {
  pub gvb: GvbConfig,
}

#[repr(C)]
pub struct GvbConfig {
  pub editor: GvbEditorConfig,
  pub simulator: GvbSimulatorConfig,
}

#[repr(C)]
pub struct GvbEditorConfig {
  pub font_size: u32,
}

#[repr(C)]
pub struct GvbSimulatorConfig {
  pub pixel_scale: u32,
  pub foreground: u32,
  pub background: u32,
}

impl From<::config::Config> for Config {
  fn from(c: ::config::Config) -> Self {
    Self { gvb: c.gvb.into() }
  }
}

impl From<::config::GvbConfig> for GvbConfig {
  fn from(c: ::config::GvbConfig) -> Self {
    Self {
      editor: c.editor.into(),
      simulator: c.simulator.into(),
    }
  }
}

impl From<::config::GvbEditorConfig> for GvbEditorConfig {
  fn from(c: ::config::GvbEditorConfig) -> Self {
    Self {
      font_size: c.font_size,
    }
  }
}

impl From<::config::GvbSimulatorConfig> for GvbSimulatorConfig {
  fn from(c: ::config::GvbSimulatorConfig) -> Self {
    Self {
      pixel_scale: c.pixel_scale,
      foreground: c.foreground,
      background: c.background,
    }
  }
}

static mut CONFIG: MaybeUninit<Config> = MaybeUninit::uninit();

pub type LoadConfigResult = Either<Utf8String, Unit>;

#[no_mangle]
pub extern "C" fn load_config() -> LoadConfigResult {
  use config::ConfigError;
  match config::load_config() {
    Ok(config) => {
      unsafe {
        CONFIG.write(config.into());
      }
      Either::Right(Unit::new())
    }
    Err(err) => match err {
      ConfigError::Io(err) => Either::Left(unsafe {
        Utf8String::new(format!("读取配置文件失败：{}", err))
      }),
      ConfigError::Yaml(err) => Either::Left(unsafe {
        Utf8String::new(format!("解析配置文件失败：{}", err))
      }),
      ConfigError::Other(err) => Either::Left(unsafe {
        Utf8String::new(format!("配置文件错误：{}", err))
      }),
    },
  }
}

#[no_mangle]
pub extern "C" fn config() -> *const Config {
  unsafe { CONFIG.assume_init_ref() as *const _ }
}
