# WQX工具箱

- [x] GVBASIC编辑器、模拟器
- [ ] GVmaker1.0模拟器
- [ ] PAC文件解包

## 编译

编译本项目需要的环境：`Rust (nightly)`、`C++ 17及以上`、`Qt 5.15.0 及以上`，以及`Python`（用于编译 `Scintilla`，`Python 2`或`Python 3`皆可）。

clone 项目到本地，然后运行 `git submodule update --init` 下载 git submodule。

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
