pub mod emoji;

pub use emoji::*;

#[derive(Debug, Clone)]
pub struct MachineProps {
  pub emoji_style: EmojiStyle,
  pub graphics_base_addr: u16,
}

include!(concat!(env!("OUT_DIR"), "/machines.rs"));
