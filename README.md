# WQX工具箱

功能：

- [x] GVBASIC编辑器、模拟器：编辑 `.bas`、`.txt` 格式的 GVBASIC 程序，格式转换（`.bas` 转 `.txt`、`.txt` 转 `.bas`），运行 GVBASIC 程序。
- [ ] GVmaker1.0模拟器
- [ ] PAC文件解包
- [ ] EBK阅读器

发版专用仓库：<https://gitlab.com/arucil/wqxtools-release>

## 编译

编译本项目使用 `nightly-2026-09-18`（Rust 1.100 nightly）与 edition 2024。仓库中的 `rust-toolchain.toml` 会由 rustup 自动安装并选择正确的 toolchain；Rust-native GUI 不再依赖 C++、Qt、qmake 或 cbindgen。

运行 GUI：

```shell
cargo run -p wqxtools-gui
```

编译 release：

```shell
cargo build --release -p wqxtools-gui
```

生成的可执行文件为 `target/release/wqxtools`，机型配置已编译进可执行文件。

GPUI 版 GUI 正在迁移中，可用 `cargo run -p wqxtools-gui-gpui` 体验编辑器、实时诊断、文件操作、模拟器和工作台界面。当前默认入口仍是功能较完整的 egui 版；GPUI 迁移状态、未覆盖功能与 Windows 构建边界见 [gui_gpui/README.md](gui_gpui/README.md)。

先前的 Floem 试验版仍可用 `cargo run -p wqxtools-gui-floem` 运行，迁移边界见 [gui_floem/README.md](gui_floem/README.md)。

仓库中的 GVBASIC 示例位于 `examples/bas`，模拟器文件数据位于运行目录下的 `dat_files`。

测试：
```shell
cargo test --all
```
