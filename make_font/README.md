# make_font

生成文曲星字体的 WOFF 格式文件，用于网页代码编辑器。

## 依赖

编译本项目需要 Node.js。

## 编译

```shell
npm install
npm run build
```

## 运行

```shell
npm run make-font
```

生成 `WenQuXing-GB2312.woff`、`WenQuXing-Icons-Old.woff`（PC1000A等旧机型的内置图标）和`WenQuXing-Icons-New.woff`（TC808等新机型的内置图标）文件。

新机型和旧机型的内置图标除了编码不一样之外完全相同。