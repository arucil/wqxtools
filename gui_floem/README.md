# Floem GUI 迁移入口

这是 wqxtools 的 Floem 正式迁移目标，已经纳入根 Cargo workspace；目前仍是**功能不完整的迁移版**，默认 `wqxtools` 入口继续使用可运行的 egui GUI。不要用本程序替代日常使用版，直到功能对照与跨平台验证完成。

```sh
cargo run -p wqxtools-gui-floem
```

已接入项目的 `gvb_interp::Document`：可编辑源码中的 BASIC 词法高亮、行号，以及来自真正编译诊断的红/黄波浪线；编辑后诊断实时更新。新建、打开 `.bas`/`.txt`、保存原格式也已接入。未保存时，新建和打开会拒绝覆盖当前内容；尚未实现“保存/放弃/取消”确认框。

仍待迁移：运行模拟器、屏幕与键盘、输入对话框、变量/数组调试器、查找替换、机型切换、重排行号、帮助以及快捷键。关闭窗口前的未保存确认也未实现。现有 egui GUI 保留上述功能；不能把这个迁移版称作完整替代。

Linux 隔离桌面已实测：默认语法错误显示波浪线；把 `10` 补成合法 `PRINT` 后错误与波浪线立即消失；含中文源码保存到 `.bas.txt` 并重新打开后正确恢复。`.bas.txt` 使用项目原有的 GBK 编码，不能用 UTF-8 终端输出判断是否损坏。中文输入法组合输入、Windows/macOS 与长文件性能未验证。Floem 0.2 的依赖链仍有 Cargo 未来 Rust 兼容性警告，需要在最终切换前评估。

```sh
cargo fmt --all --check
cargo clippy -p wqxtools-gui-floem --all-targets -- -D warnings
cargo test -p wqxtools-gui-floem --all-targets
cargo build -p wqxtools-gui-floem
```
