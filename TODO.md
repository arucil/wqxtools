- [x] PRINT、WRITE、READ、INPUT、DIM、NEXT 不能以 ELSE 结束？
  NO
- [x] 同名的数组只能定义一个？
  YES
- [x] 所有 parse_expr （所有extend_symbol）前面要 reset first_symbols
- [x] match多个token时 first_symbols 不对（match_token只能处理单个token的情况）
- [x] READ、键盘 INPUT 输入实数，忽略实数后面的字符串？文件 INPUT 输入实数，实数后面不行？
  NO
- [x] FOR、FN的函数名和参数只能是实数？
  YES
- [x] WRITE 输出0x00后面的引号？剔除 0x1f？
  NO, YES
- [x] OPEN 文件名以0x00结尾？
  NO
- [x] editor 一行文本最长多长？包含中文呢？
  94字符；中文不考虑0x1f
- [x] tokenize 关键字只识别前缀？（IFA 会当成 IF A）
  NO
- [x] 如果光标在中文的第二个字节，光标在什么位置闪烁？在中文的位置闪烁还是实际的光标位置闪烁？
  实际的光标位置
- [x] 当光标在第一列时，无论调用多少次空的 PRINT，都不会换行？
  yes
- [x] 输入的内容不会导致后面的字符变成 NUL ？
  会把输入的内容末尾后的一个字节变成 NUL，然后再把后续的连续非 NUL 字节变成 NUL
- [x] LOCATE 5,20:PRINT 0; 会不会换行？
  yes
- ~~看看TC808和PC1000A的键值和映射~~
- [x] 检查一下：emoji 的第二个字节小于161也合法，怎么对应字符？pc1000a是否一样支持？校对一下tc808和pc1000a的emoji的编码
  tc808 不支持小于 161
- [x] 按键映射一个地址只能响应一个键？
  NO


- [ ] QLineEdit 会把 control character 显示成空格，wqx字体的 control character glyph无法显示
- 关联 .bas  .lav  .pac 文件
- [ ] 在运行模拟器时不能加载其他文件
- 检查内存泄漏
    + [x] gvb
    + [x] sync & set machine name
- ~~改成GBK（有些汉字会和emoji重叠，优先使用emoji）~~
- [x] drag and drop (初始界面支持所有文件，gvbeditor界面支持bas/txt文件)
- [x] 初始界面显示功能列表
- 如果打开gvbsim后打开其他类型文件，gvbsim窗口是否能正常关闭？
- 搜索变量内容
- ~~移除 scintilla 依赖。替代物要有以下api：~~
    + get cursor pos, cursor line, cursor position
    + set font, font size
    + line number
    + highlight current line
    + error/warning squiggles
    + show error message tooltip when hovering on error/warning squiggles
    + undo/redo
    + copy/cut/paste
    + can undo/can redo
- [x] 自动重排行号，可以设置行号间隔和起始行号
- ~~scintilla antialiasing~~
- [x] 移除 QThread
- [x] if goto 后面只能跟上行号（默认是0）
- [x] C-h 当前行插入行号，C-j 下一行插入行号并跳转，C-k 上一行插入行号并跳转

- [x] scintilla 行号宽度实时变动（监测 lines_added）
- 扩展gvb:
    + [x] POINT(x,y)
    + [x] checkkey(key-code)
    + ~~@hour, @minute, @second~~
    + ~~text 支持小字模式：text 0（大字）, text 1（小字）。~~


- document:
    + [x] create new: () => (document, string)
    + ~~get path: () => option<path>~~
    + [x] load bas/txt: path -> (document, string)
    + [x] save: option<path> -> ()
    + [x] diagnostics: () -> vec
    + [ ] auto completion: () -> completion list
    + [x] compile to vm: device -> vm
    + [x] set machine name: string -> ()
    + [x] get machine name: () -> string
    + [x] edit: edit list -> ()
    + [ ] get semantic token: () -> vec
    + [ ] symbol highlight: () -> vec
    + [x] create device: () -> device
- vm:
    - [x] compile expr to InputFuncBody: string -> body
    - [x] exec: input -> exec result
    - [x] start
    - [x] stop
- mbf5:
    - [x] parse string: string -> result<mbf5>
- device:
    - [x] get graphics data
    - [x] get key
    - [x] fire key down
    - [x] fire key up
    - [x] blink cursor: () -> ()
    - [x] get screen update rect
    - [x] reset

- 设置：
    - [x] gvb editor: 字体大小
    - [x] gvb: 屏幕放大倍数

- 状态：
    - 窗口大小、窗口位置
    - 上一次打开的文件
    - 上一次的文件光标位置
    - 上一次加载文件的目录