# HNX Toolchain

A custom toolchain suite for the HNX hybrid kernel, featuring the Ohlink binary format and targeting `aarch64-hnx-ohlink`.

## Tree
```
hnx-toolchain/
├── 📂 rust-targets/         # Rust 目标描述
│   ├── 📜 aarch64-hnx-ohlink.json
│   └── 📜 aarch64-hnx.json
│
├── 📂 crates/
│   ├── 📂 ohlink-format/    # ohlink 格式库（核心）
│   │   ├── 📜 src/lib.rs
│   │   └── 📜 src/parser.rs
│   │
│   ├── 📂 ohlink-ld/        # ohlink 链接器
│   │   ├── 📜 src/main.rs
│   │   └── 📜 src/linker.rs
│   │
│   ├── 📂 ohlink-ar/        # ohlink 归档工具
│   ├── 📂 ohlink-readobj/   # ohlink 读取工具
│   ├── 📂 ohlink-asm/       # 汇编器前端
│   └── 📂 ohlink-gcc/       # GCC 前端包装
│
├── 📂 sysroot/              # 最小 sysroot
│   ├── 📂 usr/include/
│   └── 📂 usr/lib/
│
├── 📂 patches/              # 补丁文件
│   ├── 📜 rustc.patch
│   └── 📜 llvm.patch
│
├── 📂 tests/                # 工具链测试
├── 📜 build.py              # 构建脚本
├── 📜 setup.sh              # 安装脚本
└── 📜 README.md
```



## Current Status
🚧 Under active development

## Project Structure
- `crates/` - Rust工具链组件
- `design/` - 设计文档
- `scripts/` - 构建和安装脚本
- `tests/` - 测试套件

## Quick Start
```bash
# Build all tools
cargo build --release

# Build specific tool
cargo build -p ohlink-format

# Run tests
cargo test
 
## Overview
- 目标：基于自定义二进制格式 Ohlink，构建面向 `aarch64-hnx-ohlink` 的完整工具链（编译、转换、链接、检查）。
- 特色：对象/可执行使用 `.ohlink`，库使用 `.ohlib`；提供 objdump/nm 等工具进行解析与检查。

主要组件：
- `crates/ohlink-format`：Ohlink/Ohlib 格式定义与解析
- `crates/elf2ohlink`：ELF → Ohlink 对象转换
- `crates/ohlink-ld`：Ohlink 链接器，支持多输入、库解析与 AArch64 重定位
- `crates/ohlink-objdump`：显示 Ohlink 文件头、段/节与重定位；识别 `.ohlib`
- `crates/ohlink-nm`：列出符号；支持 `.ohlib` 成员符号表

## Setup
```bash
# 初始化本地工具链目录（包含 clang 包装脚本与用法输出）
bash setup-toolchain.sh
```
脚本输出的用法示例位于末尾，包括 C 编译、转换、链接与归档命令。

## Workflow
```bash
# 1) 使用包装的 clang 生成 ELF 对象 (.o)
tools/aarch64-hnx-ohlink/bin/clang -c source.c -o source.o

# 2) 将 ELF 对象转换为 Ohlink 对象 (.ohlink)
cargo run -p elf2ohlink -- source.o -o source.ohlink

# 3) 链接生成 Ohlink 可执行 (.ohlink)
cargo run -p ohlink-ld -- main.ohlink -o a.exe.ohlink

# 4) 归档生成 Ohlink 库 (.ohlib)
cargo run -p ohlink-ld -- --library -o libhnxc.ohlib foo.ohlink bar.ohlink

# 5) 使用库参与链接（选择性引入成员，解析未定义符号）
cargo run -p ohlink-ld -- main.ohlink libhnxc.ohlib -o a.exe.ohlink

# 6) 全量引入库成员（类似 --whole-archive）
cargo run -p ohlink-ld -- --whole-archive main.ohlink libhnxc.ohlib -o a.exe.ohlink
```

## Inspect
```bash
# 查看 Ohlink 头部/段与节（自动识别 .ohlib）
cargo run -p ohlink-objdump -- header a.exe.ohlink
cargo run -p ohlink-objdump -- sections a.exe.ohlink
cargo run -p ohlink-objdump -- header libhnxc.ohlib
cargo run -p ohlink-objdump -- sections libhnxc.ohlib

# 列出符号（支持 .ohlib 成员符号）
cargo run -p ohlink-nm -- a.exe.ohlink
cargo run -p ohlink-nm -- libhnxc.ohlib
```

## Magic (file 命令识别)
```bash
# 使用自定义 magic 文件进行识别测试
file -m scripts/ohlink.magic path/to/file.ohlink
file -m scripts/ohlink.magic path/to/libhnxc.ohlib
```

## AArch64 Relocations
链接器当前支持：
- `REL32`/`REL64`、`ABS32`/`ABS64`、`BRANCH26`
- `ADR_PREL_PG_HI21`（ADRP 页相对）、`ADD_ABS_LO12_NC`（ADD 低 12 位）、`LD_PREL_LO19`（LDR literal 19 位）

## Development
```bash
# 构建全部工具
cargo build --release

# 构建指定工具
cargo build -p ohlink-format
cargo build -p ohlink-ld
cargo build -p ohlink-objdump
cargo build -p ohlink-nm

# 运行基础测试
cargo test
```
