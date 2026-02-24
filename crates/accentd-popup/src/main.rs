mod ipc_client;
mod layer_shell;
mod window;

use accentd_core::config::Config;
use accentd_core::ipc::DaemonMsg;
use gtk4::glib;
use gtk4::prelude::*;
use std::cell::RefCell;
use std::os::unix::net::UnixStream;
use std::rc::Rc;
use std::sync::mpsc as std_mpsc;
use std::time::{Duration, Instant};
use tracing::{info, warn};

struct IpcState {
    rx: std_mpsc::Receiver<DaemonMsg>,
    _stream: UnixStream,
}

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

        let initial = match ipc_client::connect() {
            Ok((rx, stream)) => {
                info!("connected to accentd daemon");
                Some(IpcState { rx, _stream: stream })
            }
            Err(e) => {
                warn!(error = %e, "failed to connect to daemon, will retry");
                None
            }
        };

        let ipc_state: Rc<RefCell<Option<IpcState>>> = Rc::new(RefCell::new(initial));
        let last_reconnect: Rc<RefCell<Instant>> = Rc::new(RefCell::new(Instant::now()));

        let popup_window = Rc::new(popup_window);
        let popup_label = Rc::new(popup_label);

        let pw = Rc::clone(&popup_window);
        let pl = Rc::clone(&popup_label);

        glib::timeout_add_local(Duration::from_millis(16), move || {
            let mut state = ipc_state.borrow_mut();

            if let Some(ref ipc) = *state {
                loop {
                    match ipc.rx.try_recv() {
                        Ok(msg) => match msg {
                            DaemonMsg::ShowPopup { accents, labels, .. } => {
                                window::show_popup(&pw, &pl, &accents, &labels);
                            }
                            DaemonMsg::HidePopup => {
                                window::hide_popup(&pw);
                            }
                            _ => {}
                        },
                        Err(std_mpsc::TryRecvError::Empty) => break,
                        Err(std_mpsc::TryRecvError::Disconnected) => {
                            warn!("daemon disconnected, will try to reconnect");
                            *state = None;
                            break;
                        }
                    }
                }
            } else {
                // Try to reconnect every ~1 second
                let mut last = last_reconnect.borrow_mut();
                if last.elapsed() >= Duration::from_secs(1) {
                    *last = Instant::now();
                    match ipc_client::try_connect() {
                        Ok((rx, stream)) => {
                            info!("reconnected to accentd daemon");
                            *state = Some(IpcState { rx, _stream: stream });
                        }
                        Err(_) => {}
                    }
                }
            }

            glib::ControlFlow::Continue
        });
    });

    app.run_with_args::<&str>(&[]);
}
