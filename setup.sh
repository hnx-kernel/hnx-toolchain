#!/usr/bin/env bash
# setup.sh  ——  idempotent & portable
set -euo pipefail

############################## 可修改常量 #################################
# 如果希望装在系统目录，可以改成 /opt/hnx-toolchain
DEFAULT_INSTALL_DIR="${HOME}/.local/share/hnx-toolchain"
RC_FILE="${HOME}/.$(basename "$SHELL")rc"   # .zshrc / .bashrc
###########################################################################

# 1. 解析命令行：--install-dir 可覆盖默认路径
INSTALL_DIR="$DEFAULT_INSTALL_DIR"
while [[ $# -gt 0 ]]; do
  case "$1" in
    --install-dir) INSTALL_DIR="$2"; shift 2 ;;
    *) echo "Unknown option $1"; exit 1 ;;
  esac
done

# 2. 真正安装（cargo install）
echo "=> Installing HNX toolchain to $INSTALL_DIR"
cargo install --path crates/ohlink-ld   --root "$INSTALL_DIR"
cargo install --path crates/ohlink-ar   --root "$INSTALL_DIR"
cargo install --path crates/elf2ohlink  --root "$INSTALL_DIR"
cargo install --path crates/ohlink-objdump --root "$INSTALL_DIR" --bin oh-readohl
echo "=> Installation complete"

# 3. 生成一小段 env 脚本（idempotent marker）
ENV_SNIPPET="
# >>> HNX toolchain block >>>
export HNX_TOOLCHAIN=\"$INSTALL_DIR\"
export PATH=\"\$HNX_TOOLCHAIN/bin:\$PATH\"
export RUST_TARGET_PATH=\"${INSTALL_DIR}/targets\${RUST_TARGET_PATH:+:\$RUST_TARGET_PATH}\"
# <<< HNX toolchain block <<<
"

# 4. 如果 rc 文件里已经有我们的 block，先删掉旧块
if grep -q '^# >>> HNX toolchain block >>>' "$RC_FILE" 2>/dev/null; then
  echo "=> Removing old HNX block from $RC_FILE"
  # BSD & GNU sed 兼容
  sed -i.bak '/^# >>> HNX toolchain block >>>/,/^# <<< HNX toolchain block <<</d' "$RC_FILE"
fi

# 5. 追加新块
echo "=> Appending fresh HNX block to $RC_FILE"
printf '%s\n' "$ENV_SNIPPET" >> "$RC_FILE"

# 6. 立即在当前 shell 生效（用户无需手动 source）
eval "$ENV_SNIPPET"

# 7. 可选：把 target JSON 也拷过去，让 RUST_TARGET_PATH 能搜到
mkdir -p "$INSTALL_DIR/targets"
cp config/aarch64-hnx-ohlink.json "$INSTALL_DIR/targets/"

echo "=> Setup finished!"
echo "   HNX_TOOLCHAIN=$HNX_TOOLCHAIN"
echo "   PATH updated;  you can now run:"
echo "     cargo build --target aarch64-hnx-ohlink"