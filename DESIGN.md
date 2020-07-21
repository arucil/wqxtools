# 前端
- 选型
    + react
    + monaco editor
    + dock-spawn-ts
    
- 窗口：
    + float、dock、移动、打开、关闭窗口时实时保存当前 layout 到 local storage
    + 用手机打开是gvb/lava模拟器
- 菜单：
    
    + 窗口：屏幕、键盘、编辑器、目录树、变量表、错误列表

- 错误列表：
    + 显示 language server 报告的错误，点击错误信息跳转到对应位置
    + 切换编辑器 tab 时切换错误
    
- 设置窗口：
    + 屏幕放大
    + sleep延迟
    + 前景色、背景色
    
- toolbar：
    + 运行、停止、暂停
    + 上传、下载、打包下载
    + 打开（file tree中选中的文件）、保存、另存为
    + 重做、撤销
    + 剪切、复制、粘贴

- 编辑器：
    + 编辑器可以有多个窗口，每个窗口可以有多个 tab。每个窗口是一个 monaco editor 实例。
        - 从目录树打开文件时，从 last active 编辑器窗口创建一个 tab
    + 关闭编辑器、关闭页面时如果有未保存的修改，提示
    + 插入文曲星图标
    + bas：选择 语言类型：
        - 切换字体（前端）
        - 修改 bas 源码的 machine 指令（language server 端）
        - peek/poke地址、\[中断表]（language server 端）
    + 右键菜单：
        - 保存
        - 另存为
        - 运行
    + 编辑 .bas 文件时如果写了扩展语法则不允许保存，只能另存为 .bas.txt 文件
    + 编辑 .bas 文件时要在第一行写 rem ext 才能使用扩展语法
    
- 保存：language server 检查是否符合 bas 格式（语句类型、文件大小等），如果不是则拒绝保存，提示是否保存为 .bas.txt 格式，如果是则打开 .bas.txt 文件，同时清除原文件的 dirty 标记。

- 运行：从 language server 得到 Program 数据（raw pointer?），启动模拟器 web worker，发送 Program 和执行的指令数量 counter

- 模拟器 web worker：得到 Program，运行 rust。
    + 运行：
        result = sim.start(Program)
        while result != Stop
            result = sim.continue(message)
    + rust 返回：INPUT、SLEEP、PAUSE（执行完 counter 个指令后暂停执行）、Stop、GetTime、SetDirtyArea、ChangeCaretPos、ShowCaret、HideCaret、...
    + receive message：
        - 暂停、停止
        - 修改变量
        - GetVariables
        - ModifyVariable
    
- 屏幕：
    + requestAnimationFrame：如果有 dirty area 或 caret 显示/位置变化则刷新。
    + setInterval：如果 caret 显示则显示 caret。接收到 ShowCaret、HideCaret 之后重置 interval，显示 caret。
    
- 键盘：
    + 接收到按键事件或者鼠标点击后按下
    + 显示每个键对应的文曲星按键
    
- 变量表：
    + 暂停后发送 GetVariables 消息到模拟器 web worker
    + 修改变量时发送 ModifyVariable 消息到模拟器 web worker
    + 程序结束运行后只能查看变量，不能修改变量

# 语法
- 新增：
    - INPUT 输入函数
    - BINARY 文件操作
    - SLEEP
    - CHECKKEY，允许接收按键名称字符串（不区分大小写）
    - PAINT
    - LOAD
    - POINT(x, y)

# 问题
