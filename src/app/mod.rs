//! 应用层：showkey 作为应用与桌面环境的交互。
//!
//! 目前有系统托盘图标（[`tray`]）、设置窗口（[`settings_window`]）和开机自启
//! （[`autostart`]，写 / 删 `~/.config/autostart/showkey.desktop`）。

pub mod autostart;
pub mod settings_window;
pub mod tray;
