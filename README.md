# WQX工具箱

功能：

- [x] GVBASIC编辑器、模拟器：编辑 `.bas`、`.txt` 格式的 GVBASIC 程序，格式转换（`.bas` 转 `.txt`、`.txt` 转 `.bas`），运行 GVBASIC 程序。
- [ ] GVmaker1.0模拟器
- [ ] PAC文件解包
- [ ] EBK阅读器

<details>
  <summary>截图</summary>

  GVBASIC编辑器(Linux)：
  ![](./screenshots/linux-gvb-editor.png)

  GVBASIC模拟器(Linux)：
  ![](./screenshots/linux-gvb-sim.png)

</details>

发版专用仓库：<https://gitlab.com/arucil/wqxtools-release>

## 编译

编译本项目需要的环境：`Rust (nightly)`、`C++ 17及以上`、`Qt 6.0 及以上`。在 Windows 下编译时请使用基于 MinGW 的 Rust 和 Qt 版本。

clone 项目到本地，然后运行 `git submodule update --init --recursive` 下载 git submodule。

把 `gui/scintilla515.zip` 解压到 gui 目录。

安装编译辅助工具：

```shell
cargo install cargo-make cbindgen
```

编译：

```shell
cargo make -p release all
```

生成的可执行文件在 `gui/build/release` 目录中。

测试：
```shell
cargo test --all
```
