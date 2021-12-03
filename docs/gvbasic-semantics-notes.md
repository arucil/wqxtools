以下语句、函数的语义来自 NC3000 的 GVBASIC+。

以下的语义说明只包含 GVBASIC 中比较鲜为人知的细节部分，对 GVBASIC 入门教程中基本涵盖的基础部分不作额外说明。

# 语句

| 语句 | 语义 |
|:---:|:---|
| AUTO    | 和 REM 一样 |
| BEEP    | beep |
| BOX `<X0 expr>` , `<Y0 expr>` , `<X1 expr>` , `<Y1 expr>` [ , `<fill mode expr>` [ , `<draw mode expr>` ] ] | 画矩形。X0、Y0、X1、Y1、fill mode、draw mode 必须在 0~255 之间。<br>如果 fill mode 的 bit0 为 1，则画实心矩形，否则画空心矩形。<br>draw mode 的值在下面的注解中说明。 |
| CALL `<expr>` | 调用 `<expr>` 地址的机器码。<br>`<expr>` 在 -65535 ~ 65535 之间，如果是负数则取补码。 |
| CIRCLE `<X expr>` , `<Y expr>` , `<radius expr>` [ , `<fill mode expr>` [ , `<draw mode expr>` ] ] | 画圆。X、Y、radius、fill mode、draw mode 必须在 0~255 之间。<br>如果 fill mode 的 bit0 为 1，则画实心圆，否则画空心圆。<br>draw mode 的值在下面的注解中说明。 |
| CLEAR   | 关闭所有文件、清空所有变量、重置 DATA 指针、清空所有循环和子程序 |
| CLOSE [ # ] `<file number expr>` | 关闭文件。file number 的结果必须在 1~3 之间。 |
| CLS     | 清空屏幕和文字缓冲区、清除所有文字的 INVERSE 属性。 |
| CONT    | 不做任何操作 |
| COPY    | 和 REM 一样 |
| DATA    | 忽略其后的所有字符，直到行尾，或者遇到没有被双引号括起来的 `:`。 |
| DEF FN `<name var>`( `<parameter var>` ) = `<body expr>` | 定义函数。name 和 parameter 必须是实数类型。可以重定义之前定义的同名函数。 |
| DEL     | 和 REM 一样 |
| DIM `<lvalue>` ( , `<another lvalue>` )* | 定义变量或数组。如果定义的变量已存在，则保留变量原有的值，不会重置变量。如果定义的数组已存在，则报错。不能定义名称相同（如果名称的后缀 `$`、`%` 不同，也算不同名称）但维度不同的数组，例如 `DIM A(1), A(1, 2)` 会报错，但是 `DIM A(1), A$(2)` 没有问题。数组下标的范围是 0~32767。 |
| DRAW `<X expr>` , `<Y expr>` [ , `<draw mode expr>` ] | 画点。X、Y、draw mode 必须在 0~255 之间。<br>draw mode 的值在下面的注解中说明。 |
| EDIT    | 和 REM 一样 |
| ELLIPSE `<X expr>` , `<Y expr>` , `<X radius expr>` , `<Y radius expr>` [ , `<fill mode expr>` [ , `<draw mode expr>` ] ] | 画椭圆。X、Y、X radius、Y radius、fill mode、draw mode 必须在 0~255 之间。<br>如果 fill mode 的 bit0 为 1，则画实心椭圆，否则画空心椭圆。<br>draw mode 的值在下面的注解中说明。 |
| END     | 结束程序 |
| FIELD [ # ] `<file number expr>` ( , `<field len expr>` AS `<field name lvalue>` )+ | 为打开的 RANDOM 模式的文件分配记录（record）的成员变量（field）。file number 必须在 1~3 之间。<br>field len 必须在 0~255 之间。field name 必须是字符串变量，可以是数组。<br>AS 中间可以有空格，不需要和后面的变量名用空格分隔。<br>所有 field len 加起来不能超过打开文件时设置的 LEN。<br>执行该语句后，所有 field name 的内容都是长度为对应的 field  len、所有字节都为 `0x00` 的字符串。<br>在执行该语句后，如果对某个 field name 重新赋值，则会导致原有的字符串丢失；如果要修改原字符串，则要使用 LSET / RSET 语句。 |
| FILES   | 和 REM 一样 |
| FLASH   | 和 INVERSE 一样，但是先设置 INVERSE 再设置 FLASH 的话二者的效果会互相抵消，使得后续打印的字符没有反显效果；先设置 FLASH 后设置 INVERSE 不会发生这种情况。 |
| FOR `<var>`=`<from expr>` TO `<to expr>` [ STEP `<step expr>` ] | FOR 循环。`<var>` 必须是实数类型（即不能有 `$` 或 `%` 后缀），并且不能是数组。<br>`<from expr>`、`<to expr>` 和 `<step expr>` 在循环之前就会计算出结果，在后续的循环中不会重新计算。<br>如果省略 STEP，则步长默认为 1。如果步长为正数，则当 `<var>` 大于 `<to expr>` 时循环结束；如果步长为负数，则当 `<var>` 小于 `<to expr>` 时循环结束；如果步长为 0，则当 `<var>` 等于 `<to expr>` 时循环结束。<br>循环体至少会执行一次。<br>如果目前正在执行一个 `<var>` 相同的 FOR 循环，则会覆盖此 FOR 循环。 |
| GET [ # ] `<file number expr>` , `<record number expr>` | 从 RANDOM 文件读取一条记录（record）。file number 在 1~3 之间。<br>record number 在 -32768~32767 之间，不能为 0，如果是负数则取补码，因此最终得到的 record number 在 1~65535 之间。<br>读取的记录不能超出文件长度。 |
| GOSUB [ `<integer>` ] | 跳转子程序。如果后面有跟上行号（行号的数字中间没有空格），则跳转到行号，<br>否则跳转到行号为 `0` 的行；如果没有行号为 `0` 的行，则报错 `UNDEF'D STATEMENT`。<br>执行 RETURN 返回到 GOSUB 语句后，行号后面的字符会被忽略，和 DATA 语句一样（这是为了把处理 GOSUB 的代码重用于 ON ... GOSUB 语句中）。 |
| GOTO [ `<integer>` ] | <p>无条件跳转。如果后面有跟上行号（行号的数字中间没有空格），则跳转到行号，<br>否则跳转到行号为 `0` 的行；如果没有行号为 `0` 的行，则报错 `UNDEF'D STATEMENT`。</p><p>由于在判断行号之后就立即跳转到目标行号继续执行，因此 GOTO 语句后面的内容不会被检查。<br>例：`10 GOTO 20 something wrong` 这个 GOTO 语句后面的 `something wrong` 不会被检查。</p> |
| GRAPH   | 设置为 GRAPH 模式，隐藏光标，然后执行 CLS。在 GRAPH 模式中打印文字时文本缓冲区中为 `0x00` 的部分不会刷新到屏幕，因此这些部分之前绘制的图形可以得到保留。 |
| IF `<cond expr>` ( THEN \| GOTO ) `<then statements>` ( ELSE `<else statements>` )* | 当 cond 不为 0 时，执行 THEN / GOTO 后面的语句（THEN 和 GOTO 等价，GOTO 后面不一定要跟上行号），否则如果有 ELSE 的话，执行 ELSE 后面的语句。<br>不会判断 cond 是否是数字，如果是字符串，该语句的行为未知。<br>可以有多个 ELSE，这是为了处理多个 IF 语句嵌套的情况，但是 GVBASIC 并没有判断 IF 和 ELSE 是否匹配，ELSE 的个数可以多于 IF。在按顺序执行 then 或 else 中的语句时，如果碰到 ELSE，则直接跳过这一行剩下的内容，继续执行下一行。<br>then 开头不能是冒号，结尾可以有一个冒号；语句之间只能用一个冒号分隔。<br/>else 开头不能是冒号，如果不是最后一个 ELSE，则结尾可以有一个冒号；语句之间只能用一个冒号分隔。<br>如果 then 和 else 其中的语句是一个行号，则跳转到指定的行号。<br>在 then 或 else 中如果出现 GOSUB 或 ON ... GOSUB 语句，在使用 RETURN 回到这个 IF 语句中继续执行时，如果 GOSUB 后面的第一个冒号之后遇到 ELSE 或者行号语句就会报错 syntax error。 |
| INKEY$  | 等待按键。按键值（长度为 1 的字符串）会保存到用于表达式计算的字符串操作数栈中，这个栈只能保存 3 个元素，因此在执行 4 次 INKEY$ 语句后就栈溢出了，发生 `formula too complex` 错误。 ||
| INPUT  [ # `<file number expr>` , \| `<input prompt string>` ; ] `<lvalue>` ( , `<another lvalue>` )* | 从键盘或文件读取数据。如果有 file number（前面的 `#` 符号不能省略），则是从文件读取数据，否则从键盘读取数据。<br><br>从文件读取数据：<br>* EOF 被认为是 `0xff` 字符。<br>* 如果要读数字，但文件中数据不是合法的数字，或者数字没有以逗号或 `0xff` 字符结尾，则报错 type mismatch；<br>* 如果要读字符串，可以读有引号或无引号的字符串；有引号的字符串的引号必须闭合，否则报错 file read error；无引号的字符串读取到逗号或 `0xff`为止；<br>* 文件中每个数据（数字/字符串）必须以 `0xff` 字符或逗号结尾。<br><br>从键盘读取数据：<br>* 输出的 input prompt 会剔除其中汉字的 `0x1f` 前缀。<br>* input prompt 和输入的内容会受 INVERSE / FLASH 影响。<br>* 输入的多个数据可以用逗号分隔。<br>* 如果要求输入字符串，可以输入有引号或无引号的字符串：有引号的字符串的引号不需要闭合；无引号的字符串读取到逗号、冒号或末尾为止，如果字符串以冒号结尾，后面的内容会被忽略。<br>* 如果要求输入数字，但输入的不是合法的数字或者数字没有以逗号、冒号、输入末尾结尾，则打印 `?REENTER`，然后显示 input prompt，要求用户从头输入所有数据。如果数字以冒号结尾，则冒号后面的内容会被忽略。<br>* 如果输入的数据不足以赋值给所有 INPUT 语句的变量，则继续从键盘输入，此时 input prompt 变成 `?` 符号。 |
| INVERSE | 设置 INVERSE 模式，后续打印的字符有反显效果。反显效果只能在 TEXT 模式中起作用，在 GRAPH 模式中不起作用。 |
| KILL    | 和 REM 一样 |
| [LET] `<lvalue>` = `<expr>` | 赋值。LET 关键字可以省略 |
| LINE `<X0 expr>` , `<Y0 expr>` , `<X1 expr>` , `<Y1 expr>` [ , `<draw mode expr>` ] | 画线。X0、Y0、X1、Y1、draw mode 必须在 0~255 之间。<br>draw mode 的值在下面的注解中说明。 |
| LIST    | 和 REM 一样 |
| LOAD    | 和 REM 一样 |
| LOCATE [ `<row expr>` ] [ , `<column expr>` ] | 改变光标位置。row 必须在 1~5 之间，column 必须在 1~20 之间。<br>如果省略 row，则不改变光标纵坐标。如果省略 column，则不改变光标横坐标。<br>row 和 column 不能同时省略。 |
| LSET `<lvalue>` = `<expr>` | 把等号右边表达式的结果（必须是字符串）复制到等号左边的 lvalue 中。<br>如果 lvalue 原有的字符串比新的字符串长，则超出的部分字符串不变；如果 lvalue 原有的字符串比新的字符串短，则超出的部分字符串会覆盖掉其他变量的字符串的空间（这是 bug）。 |
| NEW     | 和 REM 一样 |
| NEXT [ `<var>` ( , `<another var>` )* ] | 继续执行 FOR 循环。如果有 `<var>`，则继续执行最近的 `<var>` 相同的 FOR 循环。<br>在 `<var>` 对应的 FOR 循环结束后，继续执行 `<another var>` 对应的循环，以此类推。 |
| NORMAL  | 取消 INVERSE 模式，后续打印的字符没有反显效果。 |
| NOTRACE | 关闭 tracing |
| ON `<expr>` ( GOTO \| GOSUB ) [ `<integer>` ( , [ `<integer>` ] )* ] | 根据 `<expr>` 的结果跳转到对应的行号。如果结果取整之后为 1，则跳转到第一个行号；为 2 则跳转到第二个行号，以此类推。如果没有对应的行号则往后面继续执行。<br>`<expr>` 的结果必须在 0~255 之间。<br>行号可以省略，如果省略某个行号，则默认为 `0`。甚至所有行号都能省略，例如 `ON <expr> GOTO` 等价于 `ON <expr> GOTO 0`。 |
| OPEN `<filename expr>` [ FOR ] [ INPUT \| OUTPUT \| APPEND \| RANDOM ] AS [ # ] `<file number>` [ LEN = `<len expr>` ] | 打开文件。filename 结果必须是字符串，不能为空，不能包含`/`字符，不能包含中文，可以包含 `0x00` 字符。filename 中的 `0x1F` 字符会被删除，经过处理的 filename 最长 14 字节，超出的部分将被截断。<br>如果省略 INPUT / OUTPUT / APPEND / RANDOM，则要用一个任意的非空格字符代替，在这种情况下默认为 RANDOM，例如 `OPEN A$ FOR @ AS 1`。<br>OUTPUT、APPEND、RANDOM 不是关键字。<br>AS 中间可以有空格，并且可以和前面的文件打开模式连起来，例如 `APPENDA  S`；不需要和后面的变量名用空格分隔。<br>file number 必须在 1~3 之间。<br>LEN 只能用于 RANDOM 模式，len 必须在 0~255 之间，如果 len 等于 0 或大于 128，则改为 32。如果省略 LEN 则 len 默认为 32。 |
| PLAY    | 和 REM 一样 |
| POKE `<addr expr>` , `<value expr>` | 把 addr 地址的字节设置为 value。<br>addr 在 -65535 ~ 65535 之间，如果是负数则取补码。<br>value 必须在 0~255 之间。 |
| POP     | 最近的 GOSUB 记录出栈，然后从 POP 语句之后继续执行。 |
| PRINT ( `<expr>` \| `,` \| `;` \| SPC(`<spc expr`) \| TAB(`<tab expr>`) )* | 打印文字。<br>* `;` 不做操作<br>* `,` 可能换行<br>* `<expr>` 打印表达式的结果。如果 `<expr>` 之后 PRINT 语句结束，则可能换行；否则如果 `<expr>` 后面跟上的不是`;` 和 `,`，则打印一个空格。<br>* `SPC` 打印 spc 个空格。spc 必须在 0~255 之间。`SPC` 后面不一定要跟上左括号，只要是一个非空格的字符就行，例如 `SPC A 1 )` 也是合法的。如果 `SPC` 表达式之后 PRINT 语句结束，则可能换行。<br>* `TAB` 把光标向右移动到 tab 列，同时用空格填充间隙。如果光标当前横坐标大于 tab，则先用空格填充到换行为止，再移动到 tab 列，同时用空格填充间隙。tab 必须在 1~20 之间。TAB 后面不一定要跟上左括号，只要是一个非空格的字符就行。如果 `TAB` 表达式之后 PRINT 语句结束，则可能换行。<br>**注**：上面 `可能换行` 的意思是，如果光标当前横坐标不在第一列，则换行。<br>在打印汉字时，如果光标当前横坐标在最后一列，此时必须换行才有足够的屏幕空间打印出完整的汉字，先打印一个空格（这个空格受 INVERSE / FLASH 影响），换行，然后打印汉字。<br>在每次打印 `<expr>`、SPC 或 TAB 之后，会把从光标当前位置直到其后出现的第一个 `0x00` 为止的字符设置为 `0x00`。<br>对于字符串，如果字符串中间出现了 `0x00`，则从此位置截断，只打印前面的部分。<br>如果字符串中出现了 `0x1F`，则忽略这个字符，并且把其后的两个字节直接输出，而不管其中是否有 `0x1F` 字节；如果其后不足两个字节，则有多少字节就打印多少字节。<br>在打印时如果发生滚屏，在 GRAPH 模式下，屏幕上原先绘制的图形也会滚动。 |
| PUT [ # ] `<file number expr>` , `<record number expr>` | 向 RANDOM 文件写入一条记录（record）。file number 在 1~3 之间。<br/>record number 在 -32768~32767 之间，不能为 0，如果是负数则取补码，因此最终得到的 record number 在 1~65535 之间。<br/>可以在文件末尾追加记录，除此之外写入的记录不能超过文件长度。<br>写入文件之后文件长度不能超过 65535。 |
| READ `<lvalue>` ( , `<another lvalue>` )* | 从 DATA 指针指向的位置读取数据。READ 语句会确保 DATA 语句后面的数据是用逗号隔开的字符串（可能用引号括起来，或者没用引号括起来。没有用引号括起来的字符串可以为空，并且其中不能出现冒号或者逗号。没有引号的字符串开头的空格会被去除，末尾的空格保留）。<br>READ 语句后面的每个变量会接收 DATA 中一个字符串（有引号/无引号），如果是字符串变量，则可以接收有引号或无引号的字符串，把有引号的字符串去掉引号。<br>如果是数字（整数/浮点数）变量，则只能接收无引号的字符串，并且字符串必须是合法的浮点数；对于整数变量，会把接收到的浮点数转换为整数。如果接收到的字符串为空，则接收到的浮点数为 0。 |
| REM     | 注释。忽略其后到行尾的所有字符 |
| RENAME  | 和 REM 一样 |
| RESTORE [ `<integer>` ] | 重置 DATA 指针。如果后面跟上行号（行号的数字中间没有空格），则把 DATA 指针重置到指定的行号。<br>如果指定的行号不存在，则重置 DATA 指针到程序开头。 |
| RETURN  | 返回最近的 GOSUB 位置继续执行 |
| RSET `<lvalue>` = `<expr>` | 把等号右边表达式的结果（必须是字符串）复制到等号左边的 lvalue 中。<br>如果 lvalue 原有的字符串比新的字符串长，则把新字符串在原字符串中右对齐，左边填上空格；如果 lvalue 原有的字符串比新的字符串短，则新字符串末尾超出的部分直接丢弃。 |
| RUN     | 清空屏幕和文字缓冲、设置为 TEXT 模式，执行 CLEAR，然后跳转到第一行执行。<br>不会检查 RUN 后面是否有参数。 |
| SAVE    | 和 REM 一样 |
| STOP    | 和 REM 一样 |
| SWAP `<lvalue 1>` , `<lvalue 2>` | 交换两个 lvalue 的值。两个 lvalue 的类型必须相同。 |
| SYSTEM  | 在 GVBASIC 交互模式（PC1000时代的模式）中退出到系统。<br>在新机器上的 GVBASIC 移除了这个模式，执行这个语句时直接报错 syntax error。 |
| TEXT    | 设置为文字模式，显示光标；然后执行 CLS。在 TEXT 模式中每次打印字符后，之前绘制到屏幕的图形都会被清除。 |
| TRACE   | 启用 tracing。启用 tracing 后，每执行一条语句之前，都会打印出当前的行号，执行完一条语句之后等待按键。 |
| WEND    | 跳转到最近的 WHILE 循环的位置后继续执行。<br>注意，如果 WHILE 循环结束，则会从和 WHILE 语句匹配的 WEND 语句后面继续执行，而不一定是当前的这个 WEND 语句。具体请看下面的注解。 |
| WHILE `<expr>` | 当前 `<expr>` 不为 0 时，执行循环。<br>不会检查 `<expr>` 的结果是否是数字；如果结果是字符串，该语句的行为未知。<br>当循环结束时，查找和这个 WHILE 语句匹配的 WEND 语句，然后从 WEND 语句后面继续执行。<br>查找匹配的 WEND 语句的具体方法请看下面的注解。 |
| WRITE [ # `<file number expr>` , ] `<datum expr>` ( [ , ] `<datum expr>` )* [ , ] | 输出数据到屏幕或文件。如果有 file number 则是输出到文件。<br>file number 在 1~3 之间。<br>如果 datum 是字符串则加上引号；如果字符串中有 `0x00` 则只输出 `0x00` 前面的部分（包括开头的引号，不包括末尾的引号），例如 `WRITE "ABC" + CHR$(0) + "DEF"` 输出 `"ABC`。<br>datum 之间的逗号可以省略，在这种情况下，忽略前面的 datum，只输出后面的 datum；例如 `WRITE "ABC" "DEF" "GHJ"` 输出 `"GHJ"`。<br>如果不省略 datum 之间的逗号则输出的结果中也有逗号，例如 `WRITE "ABC", "DEF"` 输出 `"ABC","DEF"`。<br>语句末尾额外的逗号不会输出。<br>在执行完该语句后，如果是输出到屏幕，则**不会**换行；如果是输出到文件，则输出一个额外的 `0xff` 字节。<br>WRITE语句会剔除字符串中的 0x1f 字符，规则和 PRINT 语句一样。 |

注：
- 以上的语句如果没有特别说明，都不能跟上参数。
- 上面的 `lvalue` 表示变量或数组。
- FOR 循环、WHILE 循环、GOSUB 共用一个栈。执行 FOR、NEXT、WEND、RETURN、POP 时，从栈顶到栈底查找对应的循环/子程序记录，如果找到了，则把找到的记录以及其上的所有记录弹出。  
    例：
    ```
    10 FOR I=1 TO 0
    20 GOSUB 30
    30 NEXT
    40 RETURN
    ```
    `10` 行的 FOR 循环首先入栈，然后 `20` 行的 GOSUB 入栈。`30` 行的 NEXT 从栈中弹出 FOR 循环，并且由于 GOSUB 子程序在 FOR 循环上面，也被弹出栈。  
    在执行到 `40` 行时，由于 GOSUB 已经被弹出，发生 `RETURN WITHOUT GOSUB` 错误。
- 查找和 WHILE 语句匹配的 WEND 语句的方法：
    ```
    nested-WHILEs = 0
    loop:
        move to next token
        if current token is WHILE
            nested-WHILEs = nested-WHILEs + 1
        if current token is WEND
            if nested-WHILEs == 0
                found the matching WEND and return
            nested-WHILEs = nested-WHILEs - 1
    ```
    查找 WEND 语句的过程中忽略所有跳转语句。例：
    ```
    10 WHILE I < 1
    20 GOTO 40
    30 WEND:END
    40 I=I+1:WEND
    ```
    首先进入 WHILE 循环的循环体，跳转到 `40` 行，`I` 加 1 之后回到 `10` 行；然后循环结束，查找和 WHILE 语句匹配的 WEND 语句，由于查找过程会忽略 GOTO 语句，所以找到的 WEND 语句是 `30` 行的 WEND 语句。  
- DRAW、LINE、BOX、CIRCLE、ELLIPSE 的 draw mode 取 bit0~bit2，如果为 6 则改为 1。draw mode 的值如下：
    + `0` erase
    + `1` copy
    + `2` not
    + `3`、`4`、`5` 未知

# 表达式

- 变量名最长 16 字节（不包括 `$` / `%` 后缀），超出的部分直接忽略。
- 变量名可以用空格分隔，但出现在第一个空格之后的部分将被忽略（不包括 `$` / `%` 后缀），例如 `AB CDE$` 会被认为和 `AB XY$` 是同一个变量。
- 字符串字面量的引号不必闭合。在引号不闭合的情况下，字符串的内容到行尾为止。
- 整数类型的变量在参与表达式计算时会自动转换为实数。
- 字符串最长 255 字节。
- 比较运算符 `>=`、`<=`、`<>` 中间可以出现空格。
- 实数字面值、READ 语句读取的实数、键盘 INPUT 读取的实数支持中间出现空格；文件 INPUT 读取的实数不支持中间出现空格。
- 表达式中所有转换为整数的操作都是截断小数部分。
- 匹配合法的实数的正则表达式是 `[-+]?\d*(\.\d*)?(E[-+]?\d*)?`。
- 数组下标的范围是 0~32767。

## 函数

| 函数 | 说明 |
|:---:|:----|
| ABS(`<expr>`) | 取绝对值。 |
| ASC(`<expr>`) | 取字符串的第一个字节（0~255）转换为数字。参数必须是字符串。如果字符串为空，则报错 illegal quantity。 |
| ATN(`<expr>`) | 反正切 |
| CHR$(`<expr>`) | 把实数转换为 1 个字节长的字符串。参数必须在 0~255 之间。 |
| COS(`<expr>`) | 余弦 |
| CVI$(`<expr>`) | 把保存 16 位整数的二进制数据的字符串（2字节）转换为实数。参数必须是字符串。如果字符串长度不等于 2 字节，则报错 syntax error。 |
| CVS$(`<expr>`) | 把保存实数的二进制数据的字符串（5字节）转换为实数。参数必须是字符串。如果字符串长度不等于 5 字节，则报错 syntax error。 |
| EOF(`<expr>`) | 判断 INPUT 模式文件是否已经读取到文件末尾。参数必须是实数，在 1~3 之间，对应的文件必须以 INPUT 模式打开。 |
| EXP(`<expr>`) | e 的幂 |
| INT(`<expr>`) | 参数截断小数部分。 |
| LEFT$(`<string expr>`, `<len expr>`) | 取 string 字符串左边的 len 个字符。string 必须是字符串，len 必须在 1~255 之间。如果 len 超过字符串长度，则取整个字符串。 |
| LEN(`<expr>`) | 取字符串的长度。参数必须是字符串类型。 |
| LOF(`<expr>`) | 获取 RANDOM 文件的大小。参数必须是实数，必须在 1~3 之间，对应的文件必须以 RANDOM 模式打开。 |
| LOG(`<expr>`) | 自然对数 |
| MID$(`<string expr>`, `<pos expr>` [ , `<len expr>` ] ) | 从 string 字符串的第 pos 个字符开始取 len 个字符。string 必须是字符串，pos 必须在 1~255 之间，len 必须在 0~255 之间。如果省略 len 或 len 超过剩余的字符数，则默认取剩余的所有字符。如果 pos 超过字符串长度，则总是返回空字符串。 |
| MKI$(`<expr>`) | 把实数转换为 16 位整数，然后二进制数据转换为字符串（2字节）。参数必须是实数。 |
| MKS$(`<expr>`) | 把实数的二进制数据转换为字符串（5字节）。参数必须是实数。 |
| PEEK(`<expr>`) | 获取指定地址的值（0~255）。参数在 -65535 ~ 65535 之间，如果是负数则取补码。 |
| POS(`<expr>`) | 获取光标横坐标（从 0 开始）。参数没有作用 |
| RIGHT$(`<string expr>`, `<len expr>`) | 取 string 字符串右边的 len 个字符。string 必须是字符串，len 必须在 1~255 之间。如果 len 超过字符串长度，则取整个字符串。 |
| RND(`<expr>`) | 产生0~1之间（包含0，不包含1）的随机数。如果参数大于 0，则产生新的随机数；如果参数为 0，则返回上次产生的随机数；如果参数小于 0，则用参数作为随机数种子产生随机数。 |
| SGN(`<expr>`) | 判断数字正负号，如果参数为正数，则返回 1；如果参数为 0，则返回 0；如果参数为负数，则返回 -1。 |
| SIN(`<expr>`) | 正弦 |
| SQR(`<expr>`) | 平方根 |
| STR$(`<expr>`) | 把数字转换为字符串。参数必须是数字类型。 |
| TAN(`<expr>`) | 正切 |
| VAL(`<expr>`) | 把字符串转换为实数。参数必须是字符串。如果字符串为空或者不是合法的实数，则返回 0。字符串所表示的实数后面可以不是合法的实数部分，只要前面的部分是合法的实数就行（例如 `VAL("13ABC")` 返回 `13`）。实数中间可以有空格，例如 `1.6 7  E 3` |

注：

- 以上的函数参数如果没有特别说明，都不会检查类型。如果类型不匹配，函数的结果不确定。