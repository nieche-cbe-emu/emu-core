# emu-core

尼彩（Nieche）CoolBar `.cbe` 模块的模拟核心。纯 Python，靠 Unicorn 跑 ARMv5TE 指令，
宿主 API 用陷阱表在 Python 侧实现。

```
emu/       机器、运行时、约 200 个宿主 API、显示、音频、字库
cbelib/    .cbe 容器解析与资源解码（图片、脚本、多包）
```

## 依赖

```
pip install unicorn capstone
```

## 用法

```python
from emu.host import Session
s = Session("众神之战.CBE").boot()
while True:
    s.set_keys(mask)              # 32 位键掩码
    s.set_touch(x, y, "down")     # 客户机坐标，240x400
    frame = s.step()              # RGB565 原始帧缓冲
    for e in s.take_events(): ... # 音频 / 退出 / 日志
s.stop()                          # 必须调用，游戏在这里落存档
```

`Session` 是三端唯一共用的入口：macOS 与 Windows 以子进程方式驱动它，
安卓通过 Chaquopy 在同一进程内直接调用。

## 数据目录

`$NIECHE_HOME`，默认 `~/.nieche-emu`：

```
config.json      配置
saves/<模块>/    存档
fs/<模块>/       模块自己写的文件
```

## 语料现状

29 个模块中 26 个跑进主循环、0 崩溃。未通过的三个：武林外传两个版本用老 SDK 的
数字系统调用（46 个功能号只解出 1 个）；歪歪猫缺 WPay 计费数据文件。
另有数个联网游戏可运行但服务器早已停止服务。

研究记录、固件分析与工具见 [emu-tools](https://github.com/nieche-cbe-emu/emu-tools)。

## 键位映射：尚未定论

模拟器目前把 **bit12 当左软键、bit13 当右软键**，挂断键不映射任何位。

依据来自孤岛的过场文本页：屏幕左下角画「加速」、右下角画「跳过」——软键标签的
标准位置——按 bit13 触发的正是「跳过」，按 bit12/确定 触发「加速」；进游戏后
底部左「菜单」右「商城」，bit12 打开的就是菜单。众神之战同样 bit12 开菜单。

**但这不足以当作结论，右软键的行为对不上它自己的标签：**

- 众神之战右下角画「任务」，按 bit13 弹的却是「是否退出游戏？」
- 孤岛游戏内右下角画「商城」，按 bit13 弹的却是「确定要回到标题界面？」

两个游戏都没有用右软键打开标签上写的那个功能。可能是这些标签本来就只吃触摸
（众神之战的「任务」，32 个位逐个长按 40 帧都打不开，游戏也从不调用
`Get_CurKeyDownState` 读原始键状态字），也可能是映射本身还不对。

挂断键不映射任何位，同样只是推断：没有任何模块轮询过一个"挂断位"，
真机上它由手机系统直接终止应用。

固件里 `CurKeyDownState` 这个全局是间接寻址的，literal pool 里搜不到引用，
所以还没能从固件侧读出手机键码到位的翻译表。这条线索还没走完。

键位在模拟器里可以自己改。**欢迎带着实机对照的结果开 issue。**
