use gtk4::prelude::*;
use tracing::debug;

use crate::layer_shell;

/// Build the popup window. Returns the window and the label widget to update.
pub fn build_popup(app: &gtk4::Application, font_size: u32) -> (gtk4::Window, gtk4::Label) {
    let window = gtk4::Window::builder()
        .application(app)
        .title("accentd")
        .decorated(false)
        .resizable(false)
        .default_width(1)
        .default_height(1)
        .build();

    // Try layer-shell first (Sway, Hyprland, KDE Wayland)
    let has_layer_shell = layer_shell::try_init_layer_shell(&window);

    if !has_layer_shell {
        // Fallback: floating popup window
        // On X11 this creates an override-redirect-like window
        // On GNOME Wayland this is a regular window (best we can do)
    }

    let label = gtk4::Label::new(None);
    label.set_use_markup(true);
    label.set_halign(gtk4::Align::Center);
    label.set_valign(gtk4::Align::Center);

    let css_provider = gtk4::CssProvider::new();
    css_provider.load_from_data(&format!(
        "
        window {{
            background-color: rgba(40, 40, 40, 0.95);
            border-radius: 12px;
            border: 1px solid rgba(255, 255, 255, 0.1);
        }}
        label {{
            color: white;
            font-size: {}px;
            font-weight: bold;
            padding: 12px 20px;
            font-family: monospace;
        }}
        ",
        font_size,
    ));
    gtk4::style_context_add_provider_for_display(
        &gtk4::gdk::Display::default().expect("display"),
        &css_provider,
        gtk4::STYLE_PROVIDER_PRIORITY_APPLICATION,
    );

    window.set_child(Some(&label));

    // Start hidden
    window.set_visible(false);

    (window, label)
}

/// Show the popup with the given accented characters.
pub fn show_popup(window: &gtk4::Window, label: &gtk4::Label, accents: &[String], labels: &[u8]) {
    let parts: Vec<String> = accents
        .iter()
        .zip(labels.iter())
        .map(|(accent, num)| format!("<span color='#88aaff'>{}</span>:{}", num, accent))
        .collect();

    let markup = parts.join("  ");
    label.set_markup(&markup);

    window.set_visible(true);
    debug!(count = accents.len(), "popup shown");
}

/// Hide the popup.
pub fn hide_popup(window: &gtk4::Window) {
    window.set_visible(false);
    debug!("popup hidden");
}
