以下语句、函数的语义来自 NC3000 的 GVBASIC+。

# 语句

| 语句 | 语义 |
|:---:|:---|
| AUTO    | 和 REM 一样 |
| BEEP    | beep |
| COPY    | 和 REM 一样 |
| DATA     | 忽略其后的所有字符，直到行尾，或者遇到没有被双引号括起来的 `:`。<br>双引号不需要成对出现；如果一个双引号没有成对出现，则这个双引号标记的字符串到行尾为止 |
| DEL     | 和 REM 一样 |
| EDIT    | 和 REM 一样 |
| FILES   | 和 REM 一样 |
| GOSUB   | 跳转子程序。如果后面有跟上行号（行号的数字中间没有空格），则跳转到行号，<br>否则跳转到行号为 `0` 的行；如果没有行号为 `0` 的行，则报错 `UNDEF'D STATEMENT`。<br>执行 RETURN 返回后，行号后面的字符会被忽略，和 DATA 语句一样。 |
| GOTO    | <p>无条件条组焊。如果后面有跟上行号（行号的数字中间没有空格），则跳转到行号，<br>否则跳转到行号为 `0` 的行；如果没有行号为 `0` 的行，则报错 `UNDEF'D STATEMENT1`。</p><p>由于在判断行号之后就立即跳转到目标行号继续执行，因此 GOTO 语句后面的内容不会被检查。<br>例：`10 GOTO 20 something wrong` 这个 GOTO 语句后面的 `something wrong` 不会被检查。</p> |
| KILL    | 和 REM 一样 |
| LIST    | 和 REM 一样 |
| LOAD    | 和 REM 一样 |
| NEW     | 和 REM 一样 |
| PLAY    | 和 REM 一样 |
| REM     | 注释。忽略其后到行尾的所有字符 |
| RENAME  | 和 REM 一样 |
| SAVE    | 和 REM 一样 |
| STOP    | 和 REM 一样 |