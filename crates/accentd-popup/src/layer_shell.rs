use gtk4_layer_shell::{Edge, KeyboardMode, Layer, LayerShell};
use tracing::info;

/// Try to initialize the window as a layer-shell surface (for wlroots compositors).
/// Returns true if layer-shell was successfully applied.
/// NOTE: GNOME Wayland does not support wlr-layer-shell. The popup falls back to a
/// regular window, which may not appear above fullscreen apps or position correctly.
pub fn try_init_layer_shell(window: &gtk4::Window) -> bool {
    if !gtk4_layer_shell::is_supported() {
        info!("layer-shell not supported on this compositor");
        return false;
    }

    window.init_layer_shell();
    window.set_layer(Layer::Overlay);
    window.set_keyboard_mode(KeyboardMode::None);

    // Don't anchor to any edge = centered
    window.set_anchor(Edge::Top, false);
    window.set_anchor(Edge::Bottom, false);
    window.set_anchor(Edge::Left, false);
    window.set_anchor(Edge::Right, false);

    // Use exclusive zone of -1 to not reserve space
    window.set_exclusive_zone(-1);

    info!("layer-shell initialized");
    true
}
