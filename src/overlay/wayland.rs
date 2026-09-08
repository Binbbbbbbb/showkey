//! Wayland + layer-shell：创建真正的悬浮 Overlay 窗口。
//!
//! 这一层只负责「窗口本身」：layer-shell 初始化（悬浮层、不抢焦点、钉在左下角），
//! 返回窗口和承载胶囊的水平容器。全局样式与边距的（动态）应用见 [`super::renderer`]。

use gtk4 as gtk;
use gtk::prelude::*;
use gtk4_layer_shell::{Edge, KeyboardMode, Layer, LayerShell};

/// 创建并初始化 layer-shell 悬浮窗口，返回窗口和水平胶囊容器。
pub fn build_window(app: &gtk::Application) -> (gtk::ApplicationWindow, gtk::Box) {
    let window = gtk::ApplicationWindow::builder()
        .application(app)
        .title("showkey")
        .resizable(false)
        .build();
    window.set_decorated(false);

    // layer-shell：悬浮层、不抢焦点、钉在左下角（边距由 renderer 按设置动态设置）
    window.init_layer_shell();
    window.set_layer(Layer::Overlay);
    window.set_namespace(Some("showkey"));
    window.set_keyboard_mode(KeyboardMode::None);
    window.set_anchor(Edge::Bottom, true);
    window.set_anchor(Edge::Left, true);

    let container = gtk::Box::new(gtk::Orientation::Horizontal, 0);
    window.set_child(Some(&container));
    window.present();

    (window, container)
}
