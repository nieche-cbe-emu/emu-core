# emu-core-py

尼彩（Nieche）CoolBar `.cbe` 模拟核心的 **Python 参照实现**。

**这不是发布产物。** 三端外壳跑的是
[emu-core-rs](https://github.com/nieche-cbe-emu/emu-core-rs)。
这一份的用途只有两个：**开发**和**验证**。

```
emu/       机器、运行时、约 250 个宿主 API、显示、音频、字库
cbelib/    .cbe 容器解析与资源解码（图片、脚本、多包）
```

## 为什么留着它

因为它是**判据**。新行为先在这边跑通——Python 改起来快、出错看得清、
能随时插进去打印——确认对了之后再移植到 Rust，然后逐帧比对：
画面、宿主调用序列、调用次数三样都要一致。任何分歧一律以这边为准，
除非能证明这边错了。

两边都留着，是为了让任何一边的改动都会被另一边比对出来。
只留 Rust 的话，就再也没有独立的第二意见了。

差分工具在 [emu-tools](https://github.com/nieche-cbe-emu/emu-tools)：

```
tools/rsdiff.sh      画面 + 宿主调用序列 + 调用次数
tools/ffidiff.sh     C ABI + 输入整形，对这边的 Session
tools/enginediff.sh  两个 engine 进程的协议输出
tools/ci.sh          以上全部，加构建与单元测试
```

## 用法

```
pip install unicorn capstone
python3 -c "from emu.host import Session; s=Session('x.cbe').boot(); s.step()"
```

`emu/host.py` 里的 `Session` 和 Rust 侧的 `session.rs` 是同一份契约，
逐条对齐——差分能成立就靠这个。

## 说明

本仓库只有代码。游戏数据、手机固件和真机文件系统都不在这里，也不会提供。
