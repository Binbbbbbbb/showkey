//! 按键状态管理：跟踪 Ctrl / Shift / Alt / Super 的按下状态，并把
//! 「修饰键图标 + 按键」组合成 `⇧⌃K` 这种显示字符串。

use std::sync::Arc;
use std::sync::atomic::Ordering;
use std::time::{Duration, Instant};

use evdev::KeyCode;
use tokio::sync::mpsc;

use super::evdev::KeyInput;
use super::keymap::{COMBO_SEP, Modifier, has_shift_variant, key_label, modifier_of};
use crate::config::{Hotkey, SettingsHandle};
use crate::core::{Chip, PauseControl};

/// 连续按键计数的超时时间：两次按下间隔超过它，`*N` 计数就重新开始。
const RESET_TIMEOUT: Duration = Duration::from_secs(1);

/// 当前按下的修饰键状态。
#[derive(Default)]
pub struct ModifierState {
    /// 各修饰键是否按下，下标对应 [`Modifier::ALL`] 的顺序。
    pressed: [bool; 4],
}

impl ModifierState {
    /// 设置某个修饰键的按下状态。
    pub fn set(&mut self, modifier: Modifier, pressed: bool) {
        self.pressed[modifier as usize] = pressed;
    }

    /// 某个修饰键是否按住。
    pub fn is_held(&self, modifier: Modifier) -> bool {
        self.pressed[modifier as usize]
    }

    /// 是否按住 Shift。
    pub fn shift_held(&self) -> bool {
        self.is_held(Modifier::Shift)
    }

    /// 当前修饰键状态 + `key` 组成一个 [`Hotkey`]（快捷键录制用）。
    pub fn to_hotkey(&self, key: KeyCode) -> Hotkey {
        Hotkey {
            ctrl: self.is_held(Modifier::Ctrl),
            shift: self.is_held(Modifier::Shift),
            alt: self.is_held(Modifier::Alt),
            super_key: self.is_held(Modifier::Super),
            key: key.code(),
        }
    }

    /// 当前按下的修饰键显示图标，按 Ctrl → Shift → Alt → Super 的顺序。
    pub fn modifiers(&self) -> Vec<&'static str> {
        Modifier::ALL
            .iter()
            .filter(|modifier| self.is_held(**modifier))
            .map(|modifier| modifier.icon())
            .collect()
    }

    /// 当前按住的修饰键 + 给定按键是否恰好等于 `hotkey`（用于暂停快捷键判定）。
    pub fn matches_hotkey(&self, hotkey: &Hotkey, key: KeyCode) -> bool {
        Modifier::ALL
            .iter()
            .all(|modifier| self.is_held(*modifier) == modifier.required_by(hotkey))
            && key.code() == hotkey.key
    }
}

/// 消费按键事件，跟踪修饰键状态并把显示字符串发到 `ui_tx`。
///
/// - 修饰键按下时单独发图标
/// - 普通键按下时，和当前按住的修饰键组合成 `⇧⌃K`（符号键按 Shift 切换字符）
/// - 连续按同一个组合会合并成 `⌃K *2` 这种形式（图标与计数之间加空格，避免字形重叠）
/// - 中间插入其它键，或两次按下间隔超过 [`RESET_TIMEOUT`]，都会重新计数
/// - 录制模式下捕获下一个组合存入 `captured`；按暂停快捷键会翻转 `paused` 而不显示
pub async fn report_keys(
    mut rx: mpsc::Receiver<KeyInput>,
    ui_tx: std::sync::mpsc::Sender<Chip>,
    settings: SettingsHandle,
    pause_ctl: Arc<PauseControl>,
) {
    let mut modifiers = ModifierState::default();
    let mut last_combo: Option<String> = None;
    let mut count: u32 = 0;
    let mut last_print: Option<Instant> = None;

    while let Some(KeyInput { key, pressed, .. }) = rx.recv().await {
        // 修饰键：更新状态；单独按下时也显示图标
        if let Some(modifier) = modifier_of(key) {
            modifiers.set(modifier, pressed);
            if pressed {
                let _ = ui_tx.send(Chip::key(modifier.icon().to_string()));
                // 打断连续计数，避免之后的普通键与之前合并成 *N
                last_combo = None;
            }
            continue;
        }
        // 普通键：只关心按下
        if !pressed {
            continue;
        }

        // 录制模式：把当前「修饰键 + 按键」作为快捷键捕获，不显示
        if pause_ctl.capture.load(Ordering::SeqCst) {
            let hotkey = modifiers.to_hotkey(key);
            *pause_ctl.captured.lock().unwrap() = Some(hotkey);
            pause_ctl.capture.store(false, Ordering::SeqCst);
            last_combo = None;
            continue;
        }

        // 暂停快捷键：翻转暂停状态，不显示该组合
        let hotkey = settings.read().unwrap().pause_hotkey;
        if modifiers.matches_hotkey(&hotkey, key) {
            pause_ctl.paused.fetch_xor(true, Ordering::SeqCst);
            last_combo = None;
            continue;
        }

        let shift = modifiers.shift_held();
        // 有 Shift 变体的符号键（如 `[`），按住 Shift 时折算成字符 `{`，不再显示 Shift 图标
        let consume_shift = shift && has_shift_variant(key);
        let mut mods = modifiers.modifiers();
        if consume_shift {
            mods.retain(|m| *m != Modifier::Shift.icon());
        }

        let mut parts: Vec<String> = mods.into_iter().map(str::to_string).collect();
        parts.push(key_label(key, shift));
        let combo = parts.join(COMBO_SEP);

        // 只有「同一个组合」且「没超时」才累加计数，否则重新从 1 开始
        let timed_out = last_print.is_none_or(|t| t.elapsed() >= RESET_TIMEOUT);
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
