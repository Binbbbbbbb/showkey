//! 输入层：负责读取和处理键盘事件。

pub mod evdev;
pub mod keymap;
pub mod state;

pub use evdev::{find_keyboards, run_keyboard_listener};
pub use state::report_keys;
