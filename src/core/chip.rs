//! 从输入层发往 UI 线程的「胶囊」消息类型，供键盘 / 鼠标共用。
//!
//! 键盘胶囊无强调色（用默认前景色）；鼠标胶囊带一个 [`Accent`]，让不同按钮在
//! 悬浮层里用不同颜色区分（具体颜色由 overlay 的 CSS 按主题映射）。

/// 鼠标按钮的强调色类别（在 overlay 的 CSS 里映射成主题相关的颜色）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Accent {
    /// 左键：青蓝
    Left,
    /// 右键：黄
    Right,
    /// 中键：红
    Middle,
    /// 滚轮：紫
    Scroll,
}

/// 一个待显示的胶囊：文本 + 可选的强调色。
pub struct Chip {
    pub text: String,
    /// 强调色类别；`None` 表示普通键盘胶囊（用默认前景色）。
    pub accent: Option<Accent>,
}

impl Chip {
    /// 键盘胶囊（无强调色）。
    pub fn key(text: String) -> Self {
        Self { text, accent: None }
    }

    /// 鼠标胶囊（带按钮强调色）。
    pub fn mouse(text: String, accent: Accent) -> Self {
        Self {
            text,
            accent: Some(accent),
        }
    }
}
