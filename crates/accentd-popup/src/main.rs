mod ipc_client;
mod layer_shell;
mod window;

use accentd_core::config::Config;
use accentd_core::ipc::DaemonMsg;
use gtk4::glib;
use gtk4::prelude::*;
use std::rc::Rc;
use std::time::Duration;
use tracing::{info, warn};

fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env()
                .add_directive("accentd_popup=info".parse().unwrap()),
        )
        .init();

    info!("accentd-popup starting");

    let config = Config::load().unwrap_or_default();
    let font_size = config.popup.font_size;

    let app = gtk4::Application::builder()
        .flags(gtk4::gio::ApplicationFlags::NON_UNIQUE)
        .build();

    app.connect_activate(move |app| {
        let (popup_window, popup_label) = window::build_popup(app, font_size);

        // Connect to daemon IPC
        let ipc_rx = match ipc_client::connect() {
            Ok((rx, _write_stream)) => {
                info!("connected to accentd daemon");
                Some(rx)
            }
            Err(e) => {
                warn!(error = %e, "failed to connect to daemon, running in standalone mode");
                None
            }
        };

        let popup_window = Rc::new(popup_window);
        let popup_label = Rc::new(popup_label);

        if let Some(rx) = ipc_rx {
            // Poll IPC messages on the GTK main loop
            let pw = Rc::clone(&popup_window);
            let pl = Rc::clone(&popup_label);

            glib::timeout_add_local(Duration::from_millis(16), move || {
                while let Ok(msg) = rx.try_recv() {
                    match msg {
                        DaemonMsg::ShowPopup {
                            accents, labels, ..
                        } => {
                            window::show_popup(&pw, &pl, &accents, &labels);
                        }
                        DaemonMsg::HidePopup => {
                            window::hide_popup(&pw);
                        }
                        _ => {}
                    }
                }
                glib::ControlFlow::Continue
            });
        }
    });

    app.run_with_args::<&str>(&[]);
}
