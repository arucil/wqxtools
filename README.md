# wqxtools


## 编译

编译本项目需要的环境：`Rust (nightly)`、`C++ 17及以上`、`Qt 5.15.0 及以上`，以及`Python`（用于编译 `Scintilla`）

按照 `gui/BUILDING` 中的指引，编译 `Scintilla` 组件。

安装编译辅助工具：

```shell
cargo install cargo-make
cargo install cbindgen
```

编译：

```shell
cargo make all
```