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
