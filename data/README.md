# 数据文件列表

- `ascii_16.dat` 从 TC808 的 NAND 中提取的 8x16 大小的 ASCII 字体。
- `gb2312_symbol_16.dat` 从 TC808 的 NAND 中提取的 16x16 大小的 GB2312 的 01~09 区的特殊符号字体，共 846 个符号。
- `unicode1.1_16.dat` 从 TC808 的 NAND 中提取的 16x16 大小的 Unicode 1.1 汉字字体，Unicode 码点范围是 `[U+4E00, U+9FA5]`，共 20902 个汉字。
- `gb2312_16.dat` 由 `unicode1.1_16.dat` 和 `gb2312_symbol_16.dat` 生成的 16x16 GB2312 字体。
- `icon_16.dat` 从 TC808 的 NAND 中提取的 16x16 文曲星内置图标字体。
- `nc3000-gvb+.decrypted_bin` 解密后的 NC3000 GVBASIC++.bin 文件，可以用 6502 反汇编器反汇编。

