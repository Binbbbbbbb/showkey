# showkey

在屏幕边缘以「悬浮胶囊」显示你按下的按键组合、鼠标点击和触控板手势的 Wayland 工具。

适用于录屏、直播、演示、教学等需要把输入可视化出来的场景。

## 简介

showkey 运行在 Wayland 的 layer-shell 悬浮层上，直接读取输入设备事件：

- **键盘**通过 `evdev` 读取；
- **鼠标 / 触控板**通过 `libinput` 读取（自动处理轻点、clickpad 分区、双指滚动、多指手势）。

事件会被格式化成一个个「胶囊」，在屏幕某个角落排开显示，最新的在右、旧的往左滑，几秒后自动淡出消失。

## 特性

- ⌨️ **键盘**：组合键（如 `⌘ + C`）、修饰键图标、特殊键图标（方向键、退格、媒体键、亮度等）
- 🖱️ **鼠标**：左 / 右 / 中键（颜色区分）、双击（`×2`）、滚轮
- 🖲️ **触控板**：轻点（1/2/3 指 → 左/右/中键）、双指滚动、三/四指滑动 → 对应按键组合
- 🎨 **高度可定制**：主题、胶囊配色、透明度、圆角、位置、间距、边距、显示时长、最大数量
- 🌐 **中英文界面**
- 🧰 **系统托盘图标**（单击开设置，右键菜单）
- ✨ **淡入淡出动画**

## 依赖

### 系统包（以 Fedora 为例）

| 包 | 用途 |
|---|---|
| `rust` / `cargo` | Rust 工具链 |
| `gtk4-devel` | GTK4 图形界面 |
| `libinput-devel` | 鼠标 / 触控板事件 |
| `systemd-devel` | libudev（libinput 的 udev 后端） |
| `gtk4-layer-shell-devel` | Wayland layer-shell（见下方说明） |

安装：

```bash
sudo dnf install rust cargo gtk4-devel libinput-devel systemd-devel
```

> **gtk4-layer-shell 说明**：本项目当前通过 `.cargo/config.toml` 里的 `PKG_CONFIG_PATH` 指向本地下载的 gtk4-layer-shell 开发库。如果你的发行版提供 `gtk4-layer-shell-devel` 包，可以全局安装后删除该配置文件。运行时需要 `libgtk4-layer-shell.so`（通常随该包或桌面环境安装）。

### 字体

按键图标（退格、方向键、媒体键、鼠标等）使用 [Nerd Font](https://www.nerdfonts.com/) 的图标，**必须安装一个 Nerd Font**，否则图标会显示成方块。推荐：

- [JetBrainsMono Nerd Font](https://www.nerdfonts.com/font-downloads)（程序 CSS 的首选字体）

### 运行环境

- Wayland 合成器，需支持 layer-shell 协议（如 [niri](https://github.com/YaLTeR/niri)）
- 一个 SNI / StatusNotifier 宿主，用于显示托盘图标（如 quickshell 的 bar）

## 安装

一键安装脚本（推荐）：

```bash
./install.sh
```

脚本会依次：编译 release → 安装二进制到 `~/.local/bin/showkey` → 安装图标 → 创建 `.desktop` 应用快捷方式（已存在则跳过）。

手动构建：

```bash
cargo build --release
# 二进制位于 target/release/showkey
```

## 运行

```bash
# 安装后（确保 ~/.local/bin 在 PATH 中）
showkey

# 或开发模式
cargo run
```

> **权限**：showkey 直接读取 `/dev/input/event*`。如果启动时报「没有找到可读取的键盘」，把用户加入 `input` 组后重新登录：
>
> ```bash
> sudo usermod -aG input $USER
> ```

## 使用

- **托盘图标**：单击打开设置窗口；右键菜单有「显示设置」「退出」。
- **设置窗口**：可调主题、胶囊配色、语言、透明度（最新 / 历史）、圆角、间距、位置（四角）、水平 / 垂直边距、显示时长、最大胶囊数。改动后点「保存」才生效，「重置」恢复默认。
- 设置保存后即时生效，无需重启。

### 触控板手势参考

| 手势 | 显示 |
|---|---|
| 三指左滑 | `⌘ + L` |
| 三指右滑 | `⌘ + H` |
| 三指上滑 | `⌘ + U` |
| 三指下滑 | `⌘ + I` |
| 四指上滑 | `⌘ + D` |

鼠标颜色：左键青蓝、右键黄、中键红、滚轮紫。

## 配置

配置文件位置：`~/.config/showkey/config.toml`（或 `$XDG_CONFIG_HOME/showkey/config.toml`）。缺失时使用默认值。

默认值：

| 设置 | 默认 |
|---|---|
| 界面主题 | 暗 |
| 胶囊配色 | 暗 |
| 语言 | 中文 |
| 最新胶囊透明度 | 85% |
| 历史胶囊透明度 | 50% |
| 圆角 | 18px |
| 间距 | 8px |
| 位置 | 右上 |
| 水平 / 垂直边距 | 24px |
| 显示时长 | 2750ms |
| 最大胶囊数 | 5 |
