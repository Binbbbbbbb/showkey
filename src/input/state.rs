//! 按键状态管理：跟踪 Ctrl / Shift / Alt / Super 的按下状态，并把
//! 「修饰键图标 + 按键」组合成 `⇧⌃K` 这种显示字符串。

use std::time::{Duration, Instant};

use evdev::KeyCode;
use tokio::sync::mpsc;

use super::evdev::KeyInput;
use super::keymap::{
    has_shift_variant, key_label, modifier_label, ALT_ICON, CTRL_ICON, SHIFT_ICON, SUPER_ICON,
};
use crate::display::Chip;

/// 连续按键计数的超时时间：两次按下间隔超过它，`*N` 计数就重新开始。
const RESET_TIMEOUT: Duration = Duration::from_secs(1);

/// 当前按下的修饰键状态。
#[derive(Default)]
pub struct ModifierState {
    ctrl: bool,
    shift: bool,
    alt: bool,
    super_key: bool,
}

impl ModifierState {
    /// 按下/松开一个键，更新修饰键状态（非修饰键会被忽略）。
    pub fn update(&mut self, key: KeyCode, pressed: bool) {
        match key {
            KeyCode::KEY_LEFTCTRL | KeyCode::KEY_RIGHTCTRL => self.ctrl = pressed,
            KeyCode::KEY_LEFTSHIFT | KeyCode::KEY_RIGHTSHIFT => self.shift = pressed,
            KeyCode::KEY_LEFTALT | KeyCode::KEY_RIGHTALT => self.alt = pressed,
            KeyCode::KEY_LEFTMETA | KeyCode::KEY_RIGHTMETA => self.super_key = pressed,
            _ => {}
        }
    }

    /// 是否按住 Shift。
    pub fn shift_held(&self) -> bool {
        self.shift
    }

    /// 当前按下的修饰键显示图标，按 Ctrl → Shift → Alt → Super 的顺序。
    pub fn modifiers(&self) -> Vec<&'static str> {
        let mut list = Vec::new();
        if self.ctrl {
            list.push(CTRL_ICON);
        }
        if self.shift {
            list.push(SHIFT_ICON);
        }
        if self.alt {
            list.push(ALT_ICON);
        }
        if self.super_key {
            list.push(SUPER_ICON);
        }
        list
    }
}

/// 消费按键事件，跟踪修饰键状态并把显示字符串发到 `ui_tx`。
///
/// - 修饰键按下时单独发图标
/// - 普通键按下时，和当前按住的修饰键组合成 `⇧⌃K`（符号键按 Shift 切换字符）
/// - 连续按同一个组合会合并成 `⌃K *2` 这种形式（图标与计数之间加空格，避免字形重叠）
/// - 中间插入其它键，或两次按下间隔超过 [`RESET_TIMEOUT`]，都会重新计数
pub async fn report_keys(
    mut rx: mpsc::Receiver<KeyInput>,
    ui_tx: std::sync::mpsc::Sender<Chip>,
) {
    let mut modifiers = ModifierState::default();
    let mut last_combo: Option<String> = None;
    let mut count: u32 = 0;
    let mut last_print: Option<Instant> = None;

    while let Some(KeyInput { key, pressed, .. }) = rx.recv().await {
        // 修饰键：更新状态；单独按下时也显示图标
        if let Some(icon) = modifier_label(key) {
            modifiers.update(key, pressed);
            if pressed {
                let _ = ui_tx.send(Chip::key(icon.to_string()));
                // 打断连续计数，避免之后的普通键与之前合并成 *N
                last_combo = None;
            }
            continue;
        }
        // 普通键：只关心按下
        if !pressed {
            continue;
        }

        let shift = modifiers.shift_held();
        // 有 Shift 变体的符号键（如 `[`），按住 Shift 时折算成字符 `{`，不再显示 Shift 图标
        let consume_shift = shift && has_shift_variant(key);
        let mut mods = modifiers.modifiers();
        if consume_shift {
            mods.retain(|m| *m != SHIFT_ICON);
        }

        let mut parts: Vec<String> = mods.into_iter().map(str::to_string).collect();
        parts.push(key_label(key, shift));
        let combo = parts.join(" + ");

        // 只有「同一个组合」且「没超时」才累加计数，否则重新从 1 开始
        let timed_out = last_print.map_or(true, |t| t.elapsed() >= RESET_TIMEOUT);
        if !timed_out && last_combo.as_deref() == Some(combo.as_str()) {
            count += 1;
        } else {
            last_combo = Some(combo.clone());
            count = 1;
        }
        last_print = Some(Instant::now());

        if count == 1 {
            let _ = ui_tx.send(Chip::key(combo.clone()));
        } else {
            let _ = ui_tx.send(Chip::key(format!("{combo} *{count}")));
        }
    }
}
