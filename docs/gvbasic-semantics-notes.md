以下语句、函数的语义来自 NC3000 的 GVBASIC+。

# 语句

| 语句 | 语义 |
|:---:|:---|
| AUTO    | 和 REM 一样 |
| BEEP    | beep |
| BOX `<X0 expr>` , `<Y0 expr>` , `<X1 expr>` , `<Y1 expr>` [ , `<fill mode expr>` [ , `<draw mode expr>` ] ] | 画矩形。X0、Y0、X1、Y1、fill mode、draw mode 必须在 0~255 之间。<br>如果 fill mode 的 bit0 为 1，则画实心矩形，否则画空心矩形。<br>draw mode 的值在下面的注解中说明。 |
| CALL `<expr>` | 调用 `<expr>` 地址的机器码。<br>`<expr>` 转换为整数，如果是负数则取补码；地址必须小于 65536 |
| CIRCLE `<X expr>` , `<Y expr>` , `<radius expr>` [ , `<fill mode expr>` [ , `<draw mode expr>` ] ] | 画圆。X、Y、radius、fill mode、draw mode 必须在 0~255 之间。<br>如果 fill mode 的 bit0 为 1，则画实心圆，否则画空心圆。<br>draw mode 的值在下面的注解中说明。 |
| CLEAR   | 关闭所有文件、清空所有变量、重置 DATA 指针、清空所有循环和子程序 |
| CLOSE [ # ] `<file number expr>` | 关闭文件。file number 的结果必须在 1~3 之间。 |
| CLS     | 清空屏幕和文字缓冲区 |
| CONT    | 不做任何操作 |
| COPY    | 和 REM 一样 |
| DATA    | 忽略其后的所有字符，直到行尾，或者遇到没有被双引号括起来的 `:`。 |
| DEL     | 和 REM 一样 |
| DIM `<lvalue>` [ , `<another lvalue>` ]... | 定义变量或数组。 |
| DRAW `<X expr>` , `<Y expr>` [ , `<draw mode expr>` ] | 画点。X、Y、draw mode 必须在 0~255 之间。<br>draw mode 的值在下面的注解中说明。 |
| EDIT    | 和 REM 一样 |
| ELLIPSE `<X expr>` , `<Y expr>` , `<X radius expr>` , `<Y radius expr>` [ , `<fill mode expr>` [ , `<draw mode expr>` ] ] | 画椭圆。X、Y、X radius、Y radius、fill mode、draw mode 必须在 0~255 之间。<br>如果 fill mode 的 bit0 为 1，则画实心椭圆，否则画空心椭圆。<br>draw mode 的值在下面的注解中说明。 |
| END     | 结束程序 |
| FILES   | 和 REM 一样 |
| FOR `<var>`=`<from expr>` TO `<to expr>` [ STEP `<step expr>` ] | FOR 循环。`<var>` 必须是实数类型（即不能有 `$` 或 `%` 后缀），并且不能有下标。<br>`<from expr>`、`<to expr>` 和 `<step expr>` 在循环之前就会计算出结果，在后续的循环中不会重新计算。<br>如果省略 STEP，则步长默认为 1。如果步长为正数，则当 `<var>` 大于 `<to expr>` 时循环结束；如果步长为负数，则当 `<var>` 小于 `<to expr>` 时循环结束；如果步长为 0，则当 `<var>` 等于 `<to expr>` 时循环结束。<br>循环体至少会执行一次。<br>如果目前正在执行一个 `<var>` 相同的 FOR 循环，则会覆盖此 FOR 循环。 |
| GOSUB [ `<integer>` ] | 跳转子程序。如果后面有跟上行号（行号的数字中间没有空格），则跳转到行号，<br>否则跳转到行号为 `0` 的行；如果没有行号为 `0` 的行，则报错 `UNDEF'D STATEMENT`。<br>执行 RETURN 返回到 GOSUB 语句后，行号后面的字符会被忽略，和 DATA 语句一样（这是为了把处理 GOSUB 的代码重用于 ON ... GOSUB 语句中）。 |
| GOTO [ `<integer>` ] | <p>无条件跳转。如果后面有跟上行号（行号的数字中间没有空格），则跳转到行号，<br>否则跳转到行号为 `0` 的行；如果没有行号为 `0` 的行，则报错 `UNDEF'D STATEMENT`。</p><p>由于在判断行号之后就立即跳转到目标行号继续执行，因此 GOTO 语句后面的内容不会被检查。<br>例：`10 GOTO 20 something wrong` 这个 GOTO 语句后面的 `something wrong` 不会被检查。</p> |
| GRAPH   | 设置为图形模式，隐藏光标。然后清空屏幕和文字缓冲 |
| IF `<cond expr>` ( THEN \| GOTO ) `<then statements>` [ ELSE `<else statements>` ]... | 当 cond 不为 0 时，执行 THEN / GOTO 后面的语句（THEN 和 GOTO 等价，GOTO 后面不一定要跟上行号），否则如果有 ELSE 的话，执行 ELSE 后面的语句。<br>不会判断 cond 是否是数字，如果是字符串，该语句的行为未知。<br>可以有多个 ELSE，这是为了处理多个 IF 语句嵌套的情况，但是 GVBASIC 并没有判断 IF 和 ELSE 是否匹配，ELSE 的个数可以多于 IF。在按顺序执行 then 或 else 中的语句时，如果碰到 ELSE，则直接跳过这一行剩下的内容，继续执行下一行。<br>then 开头不能是冒号，结尾可以有一个冒号；语句之间只能用一个冒号分隔。<br/>else 开头不能是冒号，如果不是最后一个 ELSE，则结尾可以有一个冒号；语句之间只能用一个冒号分隔。<br>如果 then 和 else 其中的语句是一个行号，则跳转到指定的行号。<br>在 then 或 else 中如果出现 GOSUB 或 ON ... GOSUB 语句，在使用 RETURN 回到这个 IF 语句中继续执行时，如果碰到 ELSE 或者行号语句就会报错 syntax error。 |
| INKEY$  | 等待按键。按键值（长度为 1 的字符串）会保存到用于表达式计算的字符串操作数栈中，这个栈只能保存 3 个元素，因此在连续执行 4 次 INKEY$ 语句后就栈溢出了，发生 `formula too complex` 错误。<br>如果要避免这个错误，要使用赋值语句把按键值赋值给某个变量，这样就会把字符串操作数栈的内容消耗掉。 |
| KILL    | 和 REM 一样 |
| [LET] `<lvalue>` = `<expr>` | 赋值。LET 关键字可以省略 |
| LINE `<X0 expr>` , `<Y0 expr>` , `<X1 expr>` , `<Y1 expr>` [ , `<draw mode expr>` ] | 画线。X0、Y0、X1、Y1、draw mode 必须在 0~255 之间。<br>draw mode 的值在下面的注解中说明。 |
| LIST    | 和 REM 一样 |
| LOAD    | 和 REM 一样 |
| LOCATE [ `<row expr>` ] [ , `<column expr>` ] | 改变光标位置。row 必须在 1~5 之间，column 必须在 1~20 之间。<br>如果省略 row，则不改变光标纵坐标。如果省略 column，则不改变光标横坐标。<br>row 和 column 不能同时省略。 |
| LSET `<lvalue>` = `<expr>` | 把等号右边表达式的结果（必须是字符串）复制到等号左边的 lvalue 中。<br>如果 lvalue 原有的字符串比新的字符串长，则超出的部分字符串不变；如果 lvalue 原有的字符串比新的字符串短，则超出的部分字符串会覆盖掉其他变量的字符串的空间（这是 bug）。 |
| NEW     | 和 REM 一样 |
| NEXT [ `<var>` [ , `<another var>` ]... ] | 继续执行 FOR 循环。如果有 `<var>`，则继续执行最近的 `<var>` 相同的 FOR 循环。<br>在 `<var>` 对应的 FOR 循环结束后，继续执行 `<another var>` 对应的循环，以此类推。 |
| NOTRACE | 关闭 tracing |
| ON `<expr>` ( GOTO \| GOSUB ) [ `<integer>` [ , [ `<integer>` ] ]... ] | 根据 `<expr>` 的结果跳转到对应的行号。如果结果取整之后为 1，则跳转到第一个行号；为 2 则跳转到第二个行号，以此类推。如果没有对应的行号则往后面继续执行。<br>`<expr>` 的结果必须在 0~255 之间。<br>行号可以省略，如果省略某个行号，则默认为 `0`。甚至所有行号都能省略，例如 `ON <expr> GOTO` 等价于 `ON <expr> GOTO 0`。 |
| OPEN `<filename expr>` [ FOR ] [ INPUT \| OUTPUT \| APPEND \| RANDOM ] AS [ # ] `<file number>` [ LEN = `<len expr>` ] | 打开文件。filename 结果必须是字符串，不能为空，不能包含`/`字符。filename 中的 `U+001F` 字符会被删除，经过处理的 filename 最长 14 字节，超出的部分将被截断。<br>如果省略 INPUT / OUTPUT / APPEND / RANDOM，则默认为 RANDOM。<br>OUTPUT、APPEND、RANDOM 不是关键字。<br>AS 中间可以有空格，并且可以和前面的文件打开模式连起来，例如 `APPENDA  S`。<br>file number 必须在 1~3 之间。<br>LEN 只能用于 RANDOM 模式，len 必须在 0~255 之间，如果 len 等于 0 或大于 128，则改为 32。如果省略 LEN 则 len 默认为 32。 |
| PLAY    | 和 REM 一样 |
| POKE `<addr expr>` , `<value expr>` | 把 addr 地址的字节设置为 value。<br>addr 转换为整数，如果是负数则取补码；addr 必须小于 65536。<br>value 必须在 0~255 之间。 |
| POP     | 最近的 GOSUB 记录出栈，然后跳过 POP 后面的部分字符，则 DATA 语句一样。 |
| READ `<lvalue>` [ , `<another lvalue>` ]... | 从 DATA 指针指向的位置读取数据。READ 语句会确保 DATA 语句后面的数据是用逗号隔开的字符串（可能用引号括起来，或者没用引号括起来。没有用引号括起来的字符串可以为空，并且其中不能出现冒号或者逗号。没有引号的字符串开头的空格会被去除，末尾的空格保留）。<br>READ 语句后面的每个变量会接收 DATA 中一个字符串（有引号/无引号），如果是字符串变量，则可以接收有引号或无引号的字符串，把有引号的字符串去掉引号。<br>如果是数字（整数/浮点数）变量，则只能接收无引号的字符串，并且字符串必须是合法的浮点数；对于整数变量，会把接收到的浮点数转换为整数。如果接收到的字符串为空，则接收到的浮点数为 0。 |
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
| TEXT    | 设置为文字模式，显示光标。然后清空屏幕和文字缓冲 |
| TRACE   | 启用 tracing。启用 tracing 后，每执行完一行，就会打印出当前的行号。 |
| WEND    | 跳转到最近的 WHILE 循环的位置后继续执行。<br>注意，如果 WHILE 循环结束，则会从和 WHILE 语句匹配的 WEND 语句后面继续执行，而不一定是当前的这个 WEND 语句。具体请看下面的注解。 |
| WHILE `<expr>` | 当前 `<expr>` 不为 0 时，执行循环。<br>不会检查 `<expr>` 的结果是否是数字；如果结果是字符串，该语句的行为未知。<br>当循环结束时，查找和这个 WHILE 语句匹配的 WEND 语句，然后从 WEND 语句后面继续执行。<br>查找匹配的 WEND 语句的具体方法请看下面的注解。 |

注：
- 以上的语句如果没有特别说明，都不能跟上参数。
- 上面的 `lvalue` 表示变量或数组。
- FOR 循环、WHILE 循环、GOSUB 共用一个栈。执行 NEXT、WEND、RETURN、POP 时，从栈顶到栈底查找对应的循环/子程序记录，如果找到了，则把找到的记录以及其上的所有记录弹出。  
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
    + `3` or
    + `4` and
    + `5` xor

# 表达式

注：
- 变量名可以用空格分隔。例如 `A B CD` 会被认为和 `ABCD` 是同一个变量。
- 变量名最长 17 字节（不包括 `$` / `%` 后缀，不包括其中的空格），超出的部分直接忽略。
- 字符串字面量的引号不必闭合。在这种情况下，字符串的内容到行尾为止。