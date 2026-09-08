//! 通过 `evdev` 读取 `/dev/input/event*` 的真实键盘事件。
//!
//! 这一层只负责「找到键盘」和「读取原始事件」，把按下/松开事件通过
//! channel 交给上层处理。按键的组合解析（如 `Ctrl + C`）留给后续的
//! 按键解析/状态模块处理。

use std::io;

use evdev::{Device, EventSummary, KeyCode};
use tokio::sync::mpsc;

/// 一次按键事件（按下或松开）。
pub struct KeyInput {
    /// 键
    pub key: KeyCode,
    /// 是否按下（`true` 按下，`false` 松开）
    pub pressed: bool,
}

/// 判断一个设备是不是键盘。
///
/// 鼠标、触控板也会上报按键事件（左键/右键），游戏手柄、媒体遥控器同样
/// 带 `KEY_*`/`BTN_*`。一个真正的键盘会同时支持字母键、回车和修饰键，
/// 用这几个代表按键即可把非键盘外设排除掉。
pub fn is_keyboard(device: &Device) -> bool {
    device.supported_keys().is_some_and(|keys| {
        keys.contains(KeyCode::KEY_A)
            && keys.contains(KeyCode::KEY_Z)
            && keys.contains(KeyCode::KEY_ENTER)
            && keys.contains(KeyCode::KEY_LEFTCTRL)
    })
}

/// 枚举系统中所有可读的键盘外设。
///
/// `evdev::enumerate()` 会跳过没有权限打开的设备，所以这里返回的
/// 都是可以直接读取的设备。
pub fn find_keyboards() -> Vec<Device> {
    evdev::enumerate()
        .map(|(_, device)| device)
        .filter(is_keyboard)
        .collect()
}

/// 持续读取某个键盘，把按下/松开事件发到 `tx`，直到设备断开或出错。
pub async fn run_keyboard_listener(
    device: Device,
    tx: mpsc::Sender<KeyInput>,
) -> io::Result<()> {
    let mut events = device.into_event_stream()?;
    loop {
        let event = events.next_event().await?;
        // 只关心按下（1）和松开（0），忽略自动重复（2）
        let (key, pressed) = match EventSummary::from(event) {
            EventSummary::Key(_, key, 1) => (key, true),
            EventSummary::Key(_, key, 0) => (key, false),
            _ => continue,
        };

        let input = KeyInput { key, pressed };
        // 接收端已退出（程序准备结束）时停止监听
        if tx.send(input).await.is_err() {
            break Ok(());
        }
    }
}
