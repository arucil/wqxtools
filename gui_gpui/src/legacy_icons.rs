use std::sync::{Arc, LazyLock};

use gpui_kit::{Image, ImageFormat, Img, Styled, img, px};

#[derive(Clone, Copy)]
#[repr(usize)]
pub(super) enum LegacyIcon {
  Save,
  Find,
  Replace,
  Undo,
  Redo,
  Cut,
  Copy,
  Paste,
  Run,
  Pause,
  Stop,
  Emoji,
  Simulator,
}

fn bytes(name: LegacyIcon) -> &'static [u8] {
  match name {
    LegacyIcon::Save => include_bytes!("../assets/icons/Save.svg"),
    LegacyIcon::Find => include_bytes!("../assets/icons/Find.svg"),
    LegacyIcon::Replace => include_bytes!("../assets/icons/Replace.svg"),
    LegacyIcon::Undo => include_bytes!("../assets/icons/Undo.svg"),
    LegacyIcon::Redo => include_bytes!("../assets/icons/Redo.svg"),
    LegacyIcon::Cut => include_bytes!("../assets/icons/Cut.svg"),
    LegacyIcon::Copy => include_bytes!("../assets/icons/Copy.svg"),
    LegacyIcon::Paste => include_bytes!("../assets/icons/Paste.svg"),
    LegacyIcon::Run => include_bytes!("../assets/icons/Run.svg"),
    LegacyIcon::Pause => include_bytes!("../assets/icons/Pause.svg"),
    LegacyIcon::Stop => include_bytes!("../assets/icons/Stop.svg"),
    LegacyIcon::Emoji => include_bytes!("../assets/icons/Emoji.svg"),
    LegacyIcon::Simulator => include_bytes!("../assets/icons/Simulator.svg"),
  }
}

pub(super) fn icon(name: LegacyIcon) -> Img {
  static IMAGES: LazyLock<Vec<Arc<Image>>> = LazyLock::new(|| {
    [
      LegacyIcon::Save,
      LegacyIcon::Find,
      LegacyIcon::Replace,
      LegacyIcon::Undo,
      LegacyIcon::Redo,
      LegacyIcon::Cut,
      LegacyIcon::Copy,
      LegacyIcon::Paste,
      LegacyIcon::Run,
      LegacyIcon::Pause,
      LegacyIcon::Stop,
      LegacyIcon::Emoji,
      LegacyIcon::Simulator,
    ]
    .into_iter()
    .map(|icon| {
      Arc::new(Image::from_bytes(ImageFormat::Svg, bytes(icon).to_vec()))
    })
    .collect()
  });
  img(IMAGES[name as usize].clone()).w(px(20.)).h(px(20.))
}
