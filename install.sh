#!/usr/bin/env bash
# showkey 一键打包安装脚本
#
# 用法：./install.sh
# 做的事：编译 release → 装二进制到 ~/.local/bin/showkey → 装图标到
#        $XDG_DATA_HOME/showkey/icon/（回退 ~/.local/share/showkey/icon/）
#        → 创建应用快捷方式（.desktop，已存在则跳过）。
set -euo pipefail

# 项目根目录（脚本所在目录）
PROJECT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
BIN_DIR="${HOME}/.local/bin"
ICON_DIR="${XDG_DATA_HOME:-${HOME}/.local/share}/showkey/icon"

echo "==> 编译 release 二进制…"
cd "${PROJECT_DIR}"
cargo build --release

echo "==> 安装二进制 → ${BIN_DIR}/showkey"
install -Dm755 "${PROJECT_DIR}/target/release/showkey" "${BIN_DIR}/showkey"

echo "==> 安装图标 → ${ICON_DIR}/icon.svg"
install -Dm644 "${PROJECT_DIR}/icon/icon.svg" "${ICON_DIR}/icon.svg"

# 应用快捷方式（.desktop 启动项）：已存在则不创建，避免覆盖用户改动
DESKTOP_DIR="${XDG_DATA_HOME:-${HOME}/.local/share}/applications"
DESKTOP_FILE="${DESKTOP_DIR}/showkey.desktop"
ICON_THEME_DIR="${XDG_DATA_HOME:-${HOME}/.local/share}/icons/hicolor/scalable/apps"

if [[ -f "${DESKTOP_FILE}" ]]; then
    echo "==> 应用快捷方式已存在，跳过：${DESKTOP_FILE}"
else
    echo "==> 创建应用快捷方式 → ${DESKTOP_FILE}"
    # 快捷方式图标装到标准图标主题路径，让 Icon=showkey 能解析到
    install -Dm644 "${PROJECT_DIR}/icon/icon.svg" "${ICON_THEME_DIR}/showkey.svg"
    install -Dm644 /dev/stdin "${DESKTOP_FILE}" <<EOF
[Desktop Entry]
Type=Application
Name=showkey
Comment=显示按键组合的悬浮胶囊
Exec=${BIN_DIR}/showkey
Icon=showkey
Terminal=false
Categories=Utility;
EOF
fi

echo "==> 完成！现在可在终端直接运行：showkey"

# 提示 PATH
if [[ ":$PATH:" != *":${BIN_DIR}:"* ]]; then
    echo "    注意：${BIN_DIR} 不在 PATH 中，请把它加入 PATH，例如："
    echo '    export PATH="$HOME/.local/bin:$PATH"'
fi
