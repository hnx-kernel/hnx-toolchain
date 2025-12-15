#!/bin/bash
# Setup toolchain for aarch64-hnx-ohlink

TOOLCHAIN_NAME="aarch64-hnx-ohlink"
TOOLCHAIN_DIR="${PWD}/tools/${TOOLCHAIN_NAME}"

# Create directories
mkdir -p ${TOOLCHAIN_DIR}/{bin,lib,include,share}

echo "Setting up ${TOOLCHAIN_NAME}..."
#!/usr/bin/env bash
set -e
export RUST_TARGET_PATH="$HOME/.local/share/rust-targets"
export PATH="$HNX_TOOLCHAIN/bin:$PATH"

rustc --print target-spec-json --target aarch64-hnx-ohlink > /dev/null \
  && echo "✅ JSON 合法" \
  || { echo "❌ JSON 格式错误"; exit 1; }

cargo build --target aarch64-hnx-ohlink --quiet \
  && echo "✅ cargo build 成功" \
  || { echo "❌ 构建失败"; exit 1; }

oh-readohl target/aarch64-hnx-ohlink/debug/hello-ohlink.ohlink | grep -q "NoteAbi version=1" \
  && echo "✅ ohlink 格式正确" \
  || { echo "❌ 无 NoteAbi"; exit 1; }

echo "全部验证通过！"
# 安装到 $PREFIX/bin
cargo install --path crates/ohlink-ld   --root "$TOOLCHAIN_DIR"
cargo install --path crates/ohlink-ar   --root "$TOOLCHAIN_DIR"
cargo install --path crates/elf2ohlink --root "$TOOLCHAIN_DIR"
# 可选：把 oh-readohl 也装进去
cargo install --path crates/ohlink-objdump --root "$TOOLCHAIN_DIR" --bin oh-readohl
# zsh 支持
if [ -f ~/.zshrc ] && ! grep -q "${TOOLCHAIN_DIR}/bin" ~/.zshrc; then
    echo "export PATH=\"${TOOLCHAIN_DIR}/bin:\$PATH\"" >> ~/.zshrc
    echo "Added ${TOOLCHAIN_DIR}/bin to PATH in ~/.zshrc"
    source ~/.zshrc
fi
echo "Setup finished. Please restart your terminal or run 'source ~/.bashrc' or 'source ~/.zshrc' to update PATH."
