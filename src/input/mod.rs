//! 输入层：负责读取和处理键盘 / 鼠标事件。

pub mod evdev;
pub mod keymap;
pub mod pointer;
pub mod state;

pub use evdev::{find_keyboards, run_keyboard_listener};
pub use pointer::run_pointer_listener;
pub use state::report_keys;
