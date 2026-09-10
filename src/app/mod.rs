//! 应用层：showkey 作为应用与桌面环境的交互。
//!
//! 目前有系统托盘图标（[`tray`]）、设置窗口（[`settings_window`]）、开机自启
//! （[`autostart`]，写 / 删 `~/.config/autostart/showkey.desktop`）和内置字体注册
//! （[`font`]）。

pub mod autostart;
pub mod font;
pub mod settings_window;
pub mod tray;
