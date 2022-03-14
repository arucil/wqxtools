use crate::HashMap;
use intmap::IntMap;
use std::collections::BTreeMap;
use std::io;
use std::mem::MaybeUninit;
use std::str::FromStr;
use std::time::Duration;
use util::config;
use yaml_rust::{Yaml, YamlLoader};

pub(crate) mod emoji;

pub(crate) use emoji::*;

#[derive(Debug, Clone)]
pub(crate) struct MachineProps {
  pub name: String,
  pub emoji_version: EmojiVersion,
  pub graphics_base_addr: u16,
  pub sleep_unit: Duration,
  pub text_buffer_base_addr: u16,
  pub key_buffer_addr: u16,
  pub key_mapping_addrs: Vec<u16>,
  pub key_masks: [Option<(u16, u8)>; 256],
  pub key_buffer_quit: bool,
  pub eof_behavior: EofBehavior,
  pub addrs: IntMap<AddrProp>,
  pub extra_symbol_data: Vec<u8>,
  /// symbol code -> index of extra_symbol_data
  pub extra_symbols: IntMap<usize>,
  pub brks: IntMap<BrkKind>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EofBehavior {
  Normal,
  Inverse,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AddrProp {
  Year,
  Month,
  Day,
  WeekDay,
  Hour,
  Minute,
  HalfSecond,
  SecondMult2,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BrkKind {
  Mult,
}

impl Default for MachineProps {
  fn default() -> Self {
    Self {
      name: String::new(),
      emoji_version: EmojiVersion::V2,
      graphics_base_addr: 0,
      sleep_unit: Duration::default(),
      text_buffer_base_addr: 0,
      key_buffer_addr: 0,
      key_mapping_addrs: vec![],
      key_masks: [None; 256],
      key_buffer_quit: false,
      eof_behavior: EofBehavior::Normal,
      addrs: IntMap::new(),
      extra_symbol_data: vec![],
      extra_symbols: IntMap::new(),
      brks: IntMap::new(),
    }
  }
}

impl FromStr for AddrProp {
  type Err = ();
  fn from_str(s: &str) -> Result<Self, Self::Err> {
    match s {
      "year" => Ok(Self::Year),
      "month" => Ok(Self::Month),
      "day" => Ok(Self::Day),
      "weekday" => Ok(Self::WeekDay),
      "hour" => Ok(Self::Hour),
      "minute" => Ok(Self::Minute),
      "halfsecond" => Ok(Self::HalfSecond),
      "second*2" => Ok(Self::SecondMult2),
      _ => Err(()),
    }
  }
}

impl FromStr for BrkKind {
  type Err = ();
  fn from_str(s: &str) -> Result<Self, Self::Err> {
    match s {
      "mult" => Ok(Self::Mult),
      _ => Err(()),
    }
  }
}

pub fn names() -> impl Iterator<Item = &'static str> {
  unsafe { MACHINES.assume_init_ref().keys().map(|s| s.as_str()) }
}

pub(crate) fn machines() -> &'static BTreeMap<String, MachineProps> {
  unsafe { MACHINES.assume_init_ref() }
}

static mut MACHINES: MaybeUninit<BTreeMap<String, MachineProps>> =
  MaybeUninit::uninit();
static mut MACHINES_INITED: bool = false;

pub(crate) static mut DEFAULT_MACHINE_FOR_EMOJI_VERSION_2: String =
  String::new();

pub(crate) static mut DEFAULT_MACHINE_FOR_EMOJI_VERSION_1: String =
  String::new();

#[derive(Debug)]
pub enum InitError {
  Io(io::Error),
  Yaml(yaml_rust::ScanError),
  Other(String),
}

impl From<io::Error> for InitError {
  fn from(err: io::Error) -> Self {
    Self::Io(err)
  }
}

impl From<yaml_rust::ScanError> for InitError {
  fn from(err: yaml_rust::ScanError) -> Self {
    Self::Yaml(err)
  }
}

impl From<String> for InitError {
  fn from(err: String) -> Self {
    Self::Other(err)
  }
}

impl From<&str> for InitError {
  fn from(err: &str) -> Self {
    Self::Other(err.to_owned())
  }
}

pub fn init_machines() -> Result<(), InitError> {
  let content = config::load_config_file("machines.yaml")?;
  let mut docs = YamlLoader::load_from_str(&content)?;
  unsafe {
    if MACHINES_INITED {
      MACHINES.assume_init_drop();
    }
    MACHINES.write(BTreeMap::new());
    MACHINES_INITED = true;
  }
  if docs.is_empty() {
    return Ok(());
  }

  let doc = docs.pop().unwrap();

  let mut obj = doc.into_hash().ok_or_else(|| "toplevel is not object")?;

  // default
  let default = obj
    .remove(&Yaml::String("default".to_owned()))
    .ok_or_else(|| "missing field 'default'")?;
  let mut default =
    default.into_hash().ok_or_else(|| "default is not object")?;

  // default.emoji-v2
  let v2 = default
    .remove(&Yaml::String("emoji-v2".into()))
    .ok_or_else(|| "missing field 'emoji-v2' in 'default'")?;
  let v2 = v2
    .into_string()
    .ok_or_else(|| "default.emoji-v2 is not string")?;
  unsafe {
    DEFAULT_MACHINE_FOR_EMOJI_VERSION_2 = v2.to_ascii_uppercase();
  }

  // default.emoji-v1
  let v1 = default
    .remove(&Yaml::String("emoji-v1".into()))
    .ok_or_else(|| "missing field 'emoji-v1' in 'default'")?;
  let v1 = v1
    .into_string()
    .ok_or_else(|| "default.emoji-v1 is not string")?;
  unsafe {
    DEFAULT_MACHINE_FOR_EMOJI_VERSION_1 = v1.to_ascii_uppercase();
  }

  if let Some((key, _)) = default.pop_front() {
    return Err(
      format!("superfluous field {} in 'default'", yaml_to_string(&key)).into(),
    );
  }

  for (mach_name, obj) in obj {
    let mach_name = mach_name.as_str().ok_or_else(|| {
      format!("key {} is not string", yaml_to_string(&mach_name))
    })?;

    let mut obj = obj
      .into_hash()
      .ok_or_else(|| format!("'{}' is not object", mach_name))?;

    let mut props = MachineProps::default();
    props.name = mach_name.to_ascii_uppercase();

    // emoji-version
    let emoji_version = obj
      .remove(&Yaml::String("emoji-version".into()))
      .ok_or_else(|| {
        format!("missing field 'emoji-version' in '{}'", mach_name)
      })?;
    let emoji_version = emoji_version
      .as_str()
      .ok_or_else(|| format!("{}.emoji-version is not string", mach_name))?;
    if emoji_version.eq_ignore_ascii_case("v2") {
      props.emoji_version = EmojiVersion::V2;
    } else if emoji_version.eq_ignore_ascii_case("v1") {
      props.emoji_version = EmojiVersion::V1;
    } else {
      return Err(
        format!("unrecognized emoji version '{}'", emoji_version).into(),
      );
    }

    // sleep-unit
    let sleep_unit = obj
      .remove(&Yaml::String("sleep-unit".into()))
      .ok_or_else(|| {
        format!("missing field 'sleep-unit' in '{}'", mach_name)
      })?;
    let sleep_unit = sleep_unit
      .as_str()
      .ok_or_else(|| format!("{}.sleep-unit is not string", mach_name))?;
    let i = sleep_unit
      .rfind(|c: char| !c.is_ascii_alphabetic())
      .ok_or_else(|| format!("invalid sleep unit '{}'", sleep_unit))?;
    if i == sleep_unit.len() - 1 {
      return Err(
        format!("missing unit (s/ms/us/ns) in sleep unit '{}'", sleep_unit)
          .into(),
      );
    }

    let value = sleep_unit[..i + 1]
      .parse::<f64>()
      .map_err(|_| format!("invalid sleep unit '{}'", sleep_unit))?;
    if !value.is_normal() || value < 0.0 {
      return Err(format!("invalid sleep unit '{}'", sleep_unit).into());
    }

    let sleep_unit = match &sleep_unit[i + 1..] {
      "s" => Duration::from_millis((value * 1000.0) as u64),
      "ms" => Duration::from_micros((value * 1000.0) as u64),
      "us" => Duration::from_nanos((value * 1000.0) as u64),
      "ns" => Duration::from_nanos(value as u64),
      _ => return Err(format!("invalid sleep unit '{}'", sleep_unit).into()),
    };
    props.sleep_unit = sleep_unit;

    // graphics-base-addr
    let addr = obj
      .remove(&Yaml::String("graphics-base-addr".to_owned()))
      .ok_or_else(|| {
        format!("missing field 'graphics-base-addr' in '{}'", mach_name)
      })?;
    props.graphics_base_addr = get_addr(mach_name, "graphics-base-addr", addr)?;

    // text-buffer-base-addr
    let addr = obj
      .remove(&Yaml::String("text-buffer-base-addr".to_owned()))
      .ok_or_else(|| {
        format!("missing field 'text-buffer-base-addr' in '{}'", mach_name)
      })?;
    props.text_buffer_base_addr =
      get_addr(mach_name, "text-buffer-base-addr", addr)?;

    // key-buffer-base-addr
    let addr = obj
      .remove(&Yaml::String("key-buffer-addr".to_owned()))
      .ok_or_else(|| {
        format!("missing field 'key-buffer-addr' in '{}'", mach_name)
      })?;
    props.key_buffer_addr = get_addr(mach_name, "key-buffer-addr", addr)?;

    // key-mappings
    let key_mappings = obj
      .remove(&Yaml::String("key-mappings".to_owned()))
      .ok_or_else(|| {
        format!("missing field 'key-mappings' in '{}'", mach_name)
      })?;
    let key_mappings = key_mappings
      .into_hash()
      .ok_or_else(|| format!("{}.key-mappings is not object", mach_name))?;

    let mut key_bits = HashMap::<u16, u8>::default();
    for (key, mapping) in key_mappings {
      let key = key.as_i64().ok_or_else(|| {
        format!(
          "key {}.key-mappings.{} is not integer",
          mach_name,
          yaml_to_string(&key)
        )
      })?;
      let key = u8::try_from(key).map_err(|_| {
        format!(
          "key {}.key-mappings.{} is not within the range 0~255",
          mach_name, key,
        )
      })?;

      let mut mapping = mapping.into_hash().ok_or_else(|| {
        format!("{}.key-mappings.{} is not object", mach_name, key)
      })?;
      let addr = mapping
        .remove(&Yaml::String("addr".to_owned()))
        .ok_or_else(|| {
          format!("missing field 'addr' in {}.key-mappings.{}", mach_name, key)
        })?;
      let addr = get_addr(
        format!("{}.key-mappings", mach_name),
        format!("{}", key),
        addr,
      )?;

      let bit =
        mapping
          .remove(&Yaml::String("bit".to_owned()))
          .ok_or_else(|| {
            format!("missing field 'bit' in {}.key-mappings.{}", mach_name, key)
          })?;
      let bit = bit.as_i64().ok_or_else(|| {
        format!("{}.key-mappings.{}.bit is not integer", mach_name, key)
      })?;
      if bit < 0 || bit > 7 {
        return Err(
          format!(
            "{}.key-mappings.{}.bit is not within the range 0~7",
            mach_name, key
          )
          .into(),
        );
      }

      if let Some((k, _)) = mapping.pop_front() {
        return Err(
          format!(
            "superfluous field {} in {}.key-mappings.{}",
            yaml_to_string(&k),
            mach_name,
            key
          )
          .into(),
        );
      }

      if *key_bits.entry(addr).or_insert(0) & (1 << bit) != 0 {
        return Err(
          format!("duplicate {{ addr: {}, bit: {} }}", addr, bit).into(),
        );
      }

      props.key_masks[key as usize] = Some((addr, 1 << bit));
    }

    props.key_mapping_addrs.extend(key_bits.keys());

    // key-buffer-quit
    let key_buffer_quit = obj
      .remove(&Yaml::String("key-buffer-quit".into()))
      .ok_or_else(|| {
      format!("missing field 'key-buffer-quit' in '{}'", mach_name)
    })?;
    props.key_buffer_quit = key_buffer_quit
      .as_bool()
      .ok_or_else(|| format!("{}.key-buffer-quit is not boolean", mach_name))?;

    // eof-behavior
    let eof_behavior = obj
      .remove(&Yaml::String("eof-behavior".into()))
      .ok_or_else(|| {
        format!("missing field 'eof-behavior' in '{}'", mach_name)
      })?;
    let eof_behavior = eof_behavior
      .as_str()
      .ok_or_else(|| format!("{}.eof-behavior is not string", mach_name))?;
    match eof_behavior {
      "normal" => {
        props.eof_behavior = EofBehavior::Normal;
      }
      "inverse" => {
        props.eof_behavior = EofBehavior::Inverse;
      }
      _ => {
        return Err(
          format!("invalid eof-behavior value in '{}'", mach_name).into(),
        );
      }
    }

    // addrs
    let addrs = obj
      .remove(&Yaml::String("addrs".to_owned()))
      .ok_or_else(|| format!("missing field 'addrs' in '{}'", mach_name))?;
    let addrs = addrs
      .into_hash()
      .ok_or_else(|| format!("{}.addrs is not object", mach_name))?;

    for (addr, value) in addrs {
      let addr = addr.as_i64().ok_or_else(|| {
        format!(
          "key {}.addrs.{} is not integer",
          mach_name,
          yaml_to_string(&addr)
        )
      })?;
      let addr = u16::try_from(addr).map_err(|_| {
        format!(
          "key {}.addrs.{} is not within the range 0~65535",
          mach_name, addr
        )
      })?;

      let value = value
        .into_string()
        .ok_or_else(|| format!("{}.addrs.{} is not string", mach_name, addr))?;
      let prop = value.parse().map_err(|_| {
        format!(
          "unrecognized value {} in {}.addrs.{}",
          value, mach_name, addr
        )
      })?;
      props.addrs.insert(addr as _, prop);
    }

    // extra-symbols
    if let Some(extra_symbols) =
      obj.remove(&Yaml::String("extra-symbols".to_owned()))
    {
      let extra_symbols = extra_symbols
        .into_hash()
        .ok_or_else(|| format!("{}.extra-symbols is not object", mach_name))?;

      for (code, pixels) in extra_symbols {
        let code = code.as_i64().ok_or_else(|| {
          format!(
            "key {}.extra-symbols.{} is not integer",
            mach_name,
            yaml_to_string(&code)
          )
        })?;
        let code = u16::try_from(code).map_err(|_| {
          format!(
            "key {}.extra-symbols.{} is not within the range 0~65535",
            mach_name, code
          )
        })?;
        if code < 32768 {
          return Err(
            format!(
              "bit 15 of key {}.extra-symbols.{} is not 1",
              mach_name, code
            )
            .into(),
          );
        }

        let pixels = pixels.into_vec().ok_or_else(|| {
          format!("{}.extra-symbols.{} is not array", mach_name, code)
        })?;
        let start = props.extra_symbol_data.len();
        if pixels.len() != 32 {
          return Err(
            format!("length of {}.extra-symbols.{} is not 32", mach_name, code)
              .into(),
          );
        }
        for b in pixels {
          let b = b.as_i64().ok_or_else(|| {
            format!(
              "{} in {}.extra-symbols.{} is not array",
              yaml_to_string(&b),
              mach_name,
              code
            )
          })?;
          let b = u8::try_from(b).map_err(|_| {
            format!("{} in {}.extra-symbols.{} is not byte", b, mach_name, code)
          })?;
          props.extra_symbol_data.push(b);
        }
        props.extra_symbols.insert(code as _, start);
      }
    }

    // brks
    if let Some(brks) = obj.remove(&Yaml::String("brks".to_owned())) {
      let brks = brks
        .into_hash()
        .ok_or_else(|| format!("{}.brks is not object", mach_name))?;

      for (code, brk) in brks {
        let code = code.as_i64().ok_or_else(|| {
          format!(
            "key {}.brks.{} is not integer",
            mach_name,
            yaml_to_string(&code)
          )
        })?;
        let code = u16::try_from(code).map_err(|_| {
          format!(
            "key {}.brks.{} is not within the range 0~65535",
            mach_name, code
          )
        })?;

        let brk = brk.into_string().ok_or_else(|| {
          format!("{}.brks.{} is not string", mach_name, code)
        })?;
        let brk = brk.parse().map_err(|_| {
          format!("unrecognized brk {} in {}.brks.{}", brk, mach_name, code)
        })?;
        props.brks.insert(code as _, brk);
      }
    }

    if let Some((key, _)) = obj.pop_front() {
      return Err(
        format!(
          "superfluous field {} in '{}'",
          yaml_to_string(&key),
          mach_name
        )
        .into(),
      );
    }

    unsafe {
      MACHINES
        .assume_init_mut()
        .insert(mach_name.to_ascii_uppercase(), props);
    }
  }

  unsafe {
    if !MACHINES
      .assume_init_ref()
      .contains_key(&DEFAULT_MACHINE_FOR_EMOJI_VERSION_2)
    {
      return Err(
        format!("emoji version 2 default machine '{}' not found", v2).into(),
      );
    }

    if !MACHINES
      .assume_init_ref()
      .contains_key(&DEFAULT_MACHINE_FOR_EMOJI_VERSION_1)
    {
      return Err(
        format!("emoji version 1 default machine '{}' not found", v1).into(),
      );
    }
  }

  Ok(())
}

fn get_addr(
  context: impl AsRef<str>,
  name: impl AsRef<str>,
  addr: Yaml,
) -> Result<u16, InitError> {
  let context = context.as_ref();
  let name = name.as_ref();
  let addr = addr
    .as_i64()
    .ok_or_else(|| format!("{}.{} is not integer", context, name))?;
  u16::try_from(addr).map_err(|_| {
    format!("{}.{} is not within the range 0~65535", context, name).into()
  })
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
