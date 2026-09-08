//! Wayland + layer-shell：创建真正的悬浮 Overlay 窗口。
//!
//! 这一层只负责「窗口本身」：全局样式、layer-shell 初始化（悬浮层、不抢焦点、
//! 钉在左下角），返回窗口和承载胶囊的水平容器。胶囊的绘制与过期见 [`super::renderer`]。

use gtk4 as gtk;
use gtk::prelude::*;
use gtk4_layer_shell::{Edge, KeyboardMode, Layer, LayerShell};

use crate::config::{MARGIN, SPACING};
use super::renderer::build_css;

/// 创建并初始化 layer-shell 悬浮窗口，返回窗口和水平胶囊容器。
pub fn build_window(app: &gtk::Application) -> (gtk::ApplicationWindow, gtk::Box) {
    // 全局样式：透明窗口 + 胶囊样式
    if let Some(display) = gtk::gdk::Display::default() {
        let provider = gtk::CssProvider::new();
        let css = build_css();
        provider.load_from_data(&css);
        gtk::style_context_add_provider_for_display(
            &display,
            &provider,
            gtk::STYLE_PROVIDER_PRIORITY_APPLICATION,
        );
    }

    let window = gtk::ApplicationWindow::builder()
        .application(app)
        .title("showkey")
        .resizable(false)
        .build();
    window.set_decorated(false);

    // layer-shell：悬浮层、不抢焦点、钉在左下角
    window.init_layer_shell();
    window.set_layer(Layer::Overlay);
    window.set_namespace(Some("showkey"));
    window.set_keyboard_mode(KeyboardMode::None);
    window.set_anchor(Edge::Bottom, true);
    window.set_anchor(Edge::Left, true);
    window.set_margin(Edge::Bottom, MARGIN);
    window.set_margin(Edge::Left, MARGIN);

    let container = gtk::Box::new(gtk::Orientation::Horizontal, SPACING);
    window.set_child(Some(&container));
    window.present();

    (window, container)
}
