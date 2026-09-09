//! 跨线程共享的运行时控制状态：暂停显示、快捷键录制。

use std::sync::atomic::AtomicBool;
use std::sync::Mutex;

use crate::config::Hotkey;

/// 暂停显示与快捷键录制的共享控制状态。
///
/// 由主线程创建，克隆到键盘线程（读 / 写）与设置窗口（读），用 `Arc` 共享。
pub struct PauseControl {
    /// 暂停显示：置位后主循环丢弃新胶囊。
    pub paused: AtomicBool,
    /// 录制模式：置位后键盘线程捕获下一个组合写入 [`Self::captured`]。
    pub capture: AtomicBool,
    /// 录制结果：键盘线程写入，设置窗口轮询读取。
    pub captured: Mutex<Option<Hotkey>>,
}

impl PauseControl {
    pub fn new() -> Self {
        Self {
            paused: AtomicBool::new(false),
            capture: AtomicBool::new(false),
            captured: Mutex::new(None),
        }
    }
}
