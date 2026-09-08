//! Wayland + layer-shell：创建真正的悬浮 Overlay 窗口。
//!
//! 这一层只负责「窗口本身」：layer-shell 初始化（悬浮层、不抢焦点），返回窗口和
//! 承载胶囊的水平容器。锚定位置、全局样式与边距的（动态）应用见 [`super::renderer`]。

use gtk4 as gtk;
use gtk::prelude::*;
use gtk4_layer_shell::{KeyboardMode, Layer, LayerShell};

/// 创建并初始化 layer-shell 悬浮窗口，返回窗口和水平胶囊容器。
///
/// 窗口宽度由 renderer 按「最小宽度 × 最大数量」动态设置；胶囊在这个宽度内
/// 居右排列，新胶囊推旧胶囊左移。
pub fn build_window(app: &gtk::Application) -> (gtk::ApplicationWindow, gtk::Box) {
    let window = gtk::ApplicationWindow::builder()
        .application(app)
        .title("showkey")
        .resizable(false)
        .build();
    window.set_decorated(false);
    // 给一个正的初始最小尺寸，避免空容器时高度为 0 触发 Gdk 尺寸断言；
    // 实际宽度由 renderer 按设置动态设置
    window.set_size_request(1, 1);

    // layer-shell：悬浮层、不抢焦点（锚定与边距由 renderer 按设置动态设置）
    window.init_layer_shell();
    window.set_layer(Layer::Overlay);
    window.set_namespace(Some("showkey"));
    window.set_keyboard_mode(KeyboardMode::None);

    let container = gtk::Box::new(gtk::Orientation::Horizontal, 0);
    container.set_halign(gtk::Align::End);
    window.set_child(Some(&container));
    window.present();

    (window, container)
}
