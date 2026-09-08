//! 按键 → 显示名 的映射（按键解析的一部分）。
//!
//! 把 evdev 的 `KeyCode` 翻译成适合显示的样子：
//! - 数字 / 符号键显示对应字符，并按 Shift 切换（`[` ↔ `{`、`1` ↔ `!`）
//! - 字母键显示键帽上的大写字母（`KEY_A` → `A`），不做大小写转换
//! - 无法用字符表达的特殊键用 [Nerd Font](https://www.nerdfonts.com/) 的
//!   Material Design 图标代替，风格统一（建议使用 JetBrains Mono Nerd Font）
//!
//! 图标码位取自 Material Design Icons，Nerd Font 的 `nf-md-*` 系列直接沿用
//! 这些原生码位。

use evdev::KeyCode;

/// 修饰键的显示图标（`nf-md-apple_keyboard_*`）。
pub const CTRL_ICON: &str = "\u{F0634}";
pub const SHIFT_ICON: &str = "\u{F0636}";
pub const ALT_ICON: &str = "\u{F0635}";
pub const SUPER_ICON: &str = "\u{F0633}";

/// 修饰键 → 显示图标。非修饰键返回 `None`。
pub fn modifier_label(key: KeyCode) -> Option<&'static str> {
    Some(match key {
        KeyCode::KEY_LEFTCTRL | KeyCode::KEY_RIGHTCTRL => CTRL_ICON,
        KeyCode::KEY_LEFTSHIFT | KeyCode::KEY_RIGHTSHIFT => SHIFT_ICON,
        KeyCode::KEY_LEFTALT | KeyCode::KEY_RIGHTALT => ALT_ICON,
        KeyCode::KEY_LEFTMETA | KeyCode::KEY_RIGHTMETA => SUPER_ICON,
        _ => return None,
    })
}

/// 数字 / 符号键：返回 `(无 Shift, 有 Shift)` 的显示字符。
fn symbol_label(key: KeyCode) -> Option<(&'static str, &'static str)> {
    Some(match key {
        KeyCode::KEY_1 => ("1", "!"),
        KeyCode::KEY_2 => ("2", "@"),
        KeyCode::KEY_3 => ("3", "#"),
        KeyCode::KEY_4 => ("4", "$"),
        KeyCode::KEY_5 => ("5", "%"),
        KeyCode::KEY_6 => ("6", "^"),
        KeyCode::KEY_7 => ("7", "&"),
        KeyCode::KEY_8 => ("8", "*"),
        KeyCode::KEY_9 => ("9", "("),
        KeyCode::KEY_0 => ("0", ")"),
        KeyCode::KEY_MINUS => ("-", "_"),
        KeyCode::KEY_EQUAL => ("=", "+"),
        KeyCode::KEY_LEFTBRACE => ("[", "{"),
        KeyCode::KEY_RIGHTBRACE => ("]", "}"),
        KeyCode::KEY_SEMICOLON => (";", ":"),
        KeyCode::KEY_APOSTROPHE => ("'", "\""),
        KeyCode::KEY_GRAVE => ("`", "~"),
        KeyCode::KEY_BACKSLASH => ("\\", "|"),
        KeyCode::KEY_COMMA => (",", "<"),
        KeyCode::KEY_DOT => (".", ">"),
        KeyCode::KEY_SLASH => ("/", "?"),
        _ => return None,
    })
}

/// 特殊键 → Nerd Font 图标（`nf-md-*`）。
fn icon_label(key: KeyCode) -> Option<&'static str> {
    Some(match key {
        KeyCode::KEY_SPACE => "\u{F1050}", //     keyboard-space
        KeyCode::KEY_ENTER => "\u{F0311}", //     keyboard-return
        KeyCode::KEY_BACKSPACE => "\u{F030D}", // keyboard-backspace
        KeyCode::KEY_TAB => "\u{F0312}", //       keyboard-tab
        KeyCode::KEY_ESC => "\u{F12B7}", //       keyboard-esc
        KeyCode::KEY_UP => "\u{F005D}", //        arrow-up
        KeyCode::KEY_DOWN => "\u{F0045}", //      arrow-down
        KeyCode::KEY_LEFT => "\u{F004D}", //      arrow-left
        KeyCode::KEY_RIGHT => "\u{F0054}", //     arrow-right
        KeyCode::KEY_HOME => "\u{F02DC}", //      home
        KeyCode::KEY_END => "\u{F0794}", //       arrow-collapse-right
        KeyCode::KEY_PAGEUP => "\u{F0BB2}", //    page-previous
        KeyCode::KEY_PAGEDOWN => "\u{F0BB0}", //  page-next
        KeyCode::KEY_DELETE => "\u{F01B4}", //    delete
        KeyCode::KEY_CAPSLOCK => "\u{F030E}", //  keyboard-caps
        _ => return None,
    })
}

/// 该键是否有 Shift 变体（按住 Shift 时会显示成另一个字符，如 `[` → `{`）。
pub fn has_shift_variant(key: KeyCode) -> bool {
    symbol_label(key).is_some()
}

/// 按键 → 显示名。`shift` 表示是否同时按住 Shift。
///
/// 字母键不做大小写转换（始终显示键帽上的大写字母），其余未知按键
/// 退回去掉 `KEY_` 前缀的形式（如 `KEY_F1` → `F1`）。
pub fn key_label(key: KeyCode, shift: bool) -> String {
    if let Some((base, shifted)) = symbol_label(key) {
        return if shift { shifted } else { base }.to_string();
    }

    if let Some(icon) = icon_label(key) {
        return icon.to_string();
    }

    let debug = format!("{key:?}");
    debug.strip_prefix("KEY_").unwrap_or(&debug).to_string()
}
