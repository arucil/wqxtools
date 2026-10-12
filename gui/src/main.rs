mod app;

use app::WqxToolsApp;
use eframe::egui;

fn main() -> eframe::Result {
  gvb_interp::machine::init_machines_from_str(include_str!(
    "../../gvb_interp/machines.yaml"
  ))
  .map_err(|error| {
    eframe::Error::AppCreation(Box::new(std::io::Error::other(format!(
      "加载 machines.yaml 失败: {error:?}"
    ))))
  })?;

  let options = eframe::NativeOptions {
    viewport: egui::ViewportBuilder::default()
      .with_title("WQX 工具箱")
      .with_inner_size([1180.0, 760.0])
      .with_min_inner_size([900.0, 620.0]),
    ..Default::default()
  };

  eframe::run_native(
    "WQX 工具箱",
    options,
    Box::new(|cc| Ok(Box::new(WqxToolsApp::new(cc)))),
  )
}
