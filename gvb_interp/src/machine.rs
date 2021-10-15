use std::time::Duration;

pub mod emoji;

pub use emoji::*;

#[derive(Debug, Clone)]
pub struct MachineProps {
  pub name: &'static str,
  pub emoji_style: EmojiStyle,
  pub graphics_base_addr: u16,
  pub sleep_unit: Duration,
}

include!(concat!(env!("OUT_DIR"), "/machines.rs"));
