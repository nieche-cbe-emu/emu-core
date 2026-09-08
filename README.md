# emu-core

尼彩（Nieche）CoolBar `.cbe` 模块的模拟核心。靠 Unicorn 跑 ARMv5TE 指令，
宿主 API 用陷阱表在宿主侧实现。

**两套实现并存**，这是刻意的：

```
emu/       Python 参照实现：机器、运行时、约 250 个宿主 API、显示、音频、字库
cbelib/    .cbe 容器解析与资源解码（图片、脚本、多包）
rust/      Rust 实现：分发用的运行时，另出 C ABI（libnieche）与 engine 进程
```

Python 那份是**判据**。Rust 那份必须逐帧对上它——画面、宿主调用序列、
调用次数三样都要一致。任何行为分歧一律以 Python 为准，除非能证明 Python 错了。
两边都留着，是因为这样任何一边的改动都会被另一边比对出来。

## Python 侧

```
pip install unicorn capstone
python3 -c "from emu.host import Session; s=Session('x.cbe').boot(); s.step()"
```

`emu/host.py` 里的 `Session` 是**冻结接口**，三端外壳只准用这些：
`boot / stop / step / set_keys / set_touch / soft_key / take_events / size`。
按键的边沿判定、短按锁存、触摸排队、长按连发、软键两段式都在这一层，
外壳不要自己再实现一遍。

## Rust 侧

```
cd rust && cargo build --release
```

产物三个：`libnieche`（C ABI，见 `emuffi/nieche.h`）、`engine`（无界面引擎进程，
协议和 `emu-tools` 里的 `tools/engine.py` 逐字节一致）、以及若干比对用的二进制。

`emu/native.py` 用 ctypes 接 `libnieche`，对外是和 `Session` 一样的接口，
所以 Windows 和安卓的 Python 外壳换核心不用改代码。

## 支持情况

29 个模块语料上：全部能引导；Python 与 Rust 的 60 帧逐帧差分 29/29 完全一致，
300 帧 28/29（战争机器在第 260 帧分叉，未解）。

两代入口 ABI 都接住了（新 SDK 的 screen 模型，与老 SDK 的数字系统调用），
容器的两种头（magic 4 / 8）、多包、大端(BE-32)模块、屏幕尺寸认领都实现了。

## 说明

本仓库只有代码。游戏数据、手机固件和真机文件系统都不在这里，也不会提供。
