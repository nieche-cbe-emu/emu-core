# rust —— 模拟器核心的 Rust 侧

按 `阶段 01..06` 的顺序搬。**Python 侧永远是参照实现**：
任何行为分歧一律以 Python 为准，除非能证明 Python 那边错了。

    cbelib/    容器解析、资源解码（阶段 02）

## 差分测试

基准由 Python 侧生成：

    python3 tools/baseline.py gen 300     -> spec/baseline/*.txt

Rust 侧跑出同样格式的文件，再用 `tools/baseline.py diff` 找第一处分叉。
输入脚本 `default-v1` 的定义在 `tools/baseline.py` 的文档字符串里，
两边必须一致。

## 移植纪律

Python 侧那些注释**必须一起搬**。它们记的不是"这行在干什么"，
而是"这里为什么不能按直觉写"——参数顺序反着给、4 像素行距、
Storage_Date 第四个参数是 isLoad、DF_Read* 的小端陷阱。
丢掉注释等于把踩过的坑重新挖一遍。

## 进度

    阶段 01  差分底座        完成  spec/baseline/*.txt，29 个模块，重跑逐字节一致
    阶段 02  容器 + LZ + 图片 完成  29/29 与 Python 逐行一致
                                   4157 张图（含 732 张带 alpha）、1702 个 LZ 条目
    阶段 03  机器层          进行中
             ├ 内存布局      完成  29/29 段落摆放与 Python 一致
             ├ 分配器        完成  5 个单元测试（复用、栈顶回退、相邻合并、对齐、拒绝巨量）
             ├ 规格表        完成  vmspec.rs，SYS 25 项 / 27 张表 / 901 个字段
             ├ unicorn 接通  完成  unicorn-engine 2.1.5 编译通过
             ├ 陷阱表        完成  new_trap / new_table / native_const
             ├ 读写原语      完成  Mach trait：r8/r16/r32/w32/cstr/read_upto/arg/ret
             ├ 钩子          完成  陷阱区、空指针兜底、未映射内存、非法指令
             ├ call 循环     完成  含空指针续跑（最多 20000 次）
             ├ 冒烟          完成  29/29：段落落位、陷阱槽、堆读写、兜底区读零
             └ 跑通模块入口   完成  见阶段 04
    阶段 04  宿主接口        进行中
             ├ 宿主上下文     完成  sys 表、Get/Init 槽位、gm_TRACE、老式跳板
             ├ manager 表    完成  惰性建表，表尾留白记未实现
             ├ 入口约定判别   完成  capstone，29/29 与 Python 判别一致
             ├ 老 SDK 系统调用 完成  1950 注册 / 2001 取对象 / 143 内存管理器
             ├ 引导验收      完成  **29/29 与 Python 逐行一致**（style/cb0/cb1/managers/exit）
             ├ 接口注册表     完成  三级匹配（精确 / manager 通配 / 名字唯一）
             ├ 内存批        完成  dF_Malloc_In / mallocBigMen / MB_Malloc / gblock …
             ├ C 库批        完成  memcpy/memset/strlen/strcpy/strncpy/strcat/atoi/rand
             ├ 帧缓冲与图像仓  完成  gfx.rs：4 像素行距、掩码透明、整块读写
             ├ 图形批        完成  VMDrawImageWithClipEx / ClipAndAlphaEx /
                                   FillRect(Ex) / IMG_CreateImageFormStream / ReleaseImage
             ├ 屏幕与输入批   完成  vmAddScreen(Ex) / GAME_isKeyDown / IsPointer*
             ├ 系统计费批     完成  BILLING_GetPayNumByAppId（离线直启的关键）/
                                   initMemoryBlock / SCREEN_ChangeScreen / DF_SetDataPackage
             ├ 帧循环        完成  pending 队列、屏幕栈、live_screens、default-v1 输入
             ├ DF 资源包      完成  initDFDataPackage + 13 个方法槽、根包与子包拼表
             ├ DF 读写与字体   完成  DF_Read/Write Short/Int（一律小端）、字体度量
             └ 其余接口       进行中  差分工具会逐批点名（见下）

### 现状

    引导            29/29 与 Python 逐行一致
    C ABI 帧哈希     29/29 全对
    帧差分（60 帧）  **29/29 完全一致**
    帧差分（300 帧） 28/29，只剩战争机器在第 260 帧分叉

"帧完全一致"的判据是三项同时成立：**画面哈希、宿主调用名序列哈希、
调用次数**，逐帧比对。

### 差分怎么用

    $CARGO_TARGET_DIR/release/framecmp <file.cbe> 300 > /tmp/rs.txt
    python3 tools/baseline.py diff spec/baseline/<名字>.txt /tmp/rs.txt

它会直接告诉你**第几帧先分叉、分在调用序列还是画面上**。
再用 `DIAG=1 framecmp <file.cbe> 1` 看那一帧还缺哪些接口——
下一批搬什么不用猜，工具点名。

### 搬运顺序按调用频次

全语料 60 帧里，被调用过的接口 235 个、合计 41 万次，
**前 24 个就占了 95%**。所以先搬热的，冷的按需补。前十：

    VMDrawImageWithClipEx      234419 次   14 个模块
    VMDrawImageClipAndAlphaEx   56433 次   19 个模块
    DF_ReadInt                  34603 次   16 个模块
    VMGetStringWidth            11538 次   18 个模块
    VMFillRect                   6248 次    4 个模块
    GAME_isKeyDown               4354 次   16 个模块
    strncpy                      4065 次    1 个模块
    VMDrawImageClipAndAlpha2     3960 次    2 个模块
    SCREEN_IsPointerDown         3660 次   16 个模块
    dF_Malloc_In                 3597 次   22 个模块
    阶段 05  FFI + 三端      进行中
             ├ C ABI 导出层   完成  emuffi -> libnieche.{dylib,a}，nieche.h
             ├ 端到端验收     完成  **29/29 经 C ABI 帧哈希全对**（tools/ffidiff.sh）
             └ 三端换芯       未做
    阶段 06  双轨并存        未开始

## 构建

**CARGO_TARGET_DIR 必须指到不含空格的路径。**
unicorn 的构建脚本会直接拒绝："main directory cannot contain spaces nor colons"，
而本仓库的目录名里有空格（`nicai emulator`）。

    export PATH="/opt/homebrew/Cellar/rustup/1.29.0_2/bin:$PATH"
    export CARGO_TARGET_DIR="$HOME/.cache/nieche-rust"
    cd rust && cargo build --release

## 各阶段的验收怎么跑

阶段 02 —— 容器、LZ、图片解码：

    python3 tools/dumpcontainer.py <file.cbe>            > /tmp/py.txt
    $CARGO_TARGET_DIR/release/cbedump <file.cbe>         > /tmp/rs.txt
    diff /tmp/py.txt /tmp/rs.txt

阶段 03 —— 段落摆放：

    python3 tools/dumplayout.py <file.cbe>               > /tmp/py.txt
    $CARGO_TARGET_DIR/release/layout <file.cbe>          > /tmp/rs.txt
    diff /tmp/py.txt /tmp/rs.txt

阶段 04 —— 引导（这一项最值钱：cb0/cb1 是模块自己写回来的函数地址，
入口约定判错、宿主上下文摆错、内存落位错，任何一样都会让它们对不上）：

    python3 tools/dumpboot.py <file.cbe>                 > /tmp/py.txt
    $CARGO_TARGET_DIR/release/bootcmp <file.cbe>         > /tmp/rs.txt
    diff /tmp/py.txt /tmp/rs.txt

两边都必须逐行相同。条目数据不直接比，比 CRC32；
type-2 条目额外比"解压后"的 CRC32 和长度；
图片条目额外比宽高、RGB565 全图 CRC32、透明索引、alpha 掩码 CRC32。

## 规格表怎么更新

    python3 tools/genspec.py       # 读固件 DWARF -> emu/vmspec.py
    python3 tools/genspec_rs.py    # 读 emu/vmspec.py -> rust/emucore/src/vmspec.rs

Rust 侧从 vmspec.py 转而不是直接读固件：那份 197MB 的 axf 是私有资产，
公开仓库和 CI 都拿不到。以 vmspec.py 为单一真源，两边就不可能漂移。

## 架构：为什么宿主接口是裸 fn 而不是闭包

unicorn 的钩子拿到的是 `&mut Unicorn<D>`，而处理器本身存在 `D` 里。
如果处理器是 `Box<dyn FnMut>`，调用时就要同时可变借用"处理器"和"整个 D"，
借用检查器不会答应。裸 `fn` 是 `Copy`：

    let h = uc.get_data().slots[idx].handler;   // 复制出来，借用当场结束
    if let Some(f) = h { f(uc); }               // 再调用毫无阻碍

代价是处理器不能捕获环境。需要"这一格属于哪张表、哪个偏移"这类信息时，
放进 `Slot` 的 `tag` / `off` 字段，由处理器自己回头查。
Python 侧那个 `_make_method(tag, off, nm)` 闭包，在 Rust 里就是这么落地的。
