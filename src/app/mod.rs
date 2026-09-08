//! 应用层：showkey 作为应用与桌面环境的交互。
//!
//! 目前有系统托盘图标（[`tray`]）和设置窗口（[`settings_window`]）。后期会加入
//! 读 `.desktop` 文件与按 App ID 查找应用图标等能力。

pub mod settings_window;
pub mod tray;
