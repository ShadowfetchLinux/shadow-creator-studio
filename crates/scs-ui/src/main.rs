use adw::prelude::*;

mod app;
mod ipc;
mod live;
mod pages;
mod state;
mod widgets;
mod wizard;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if let Some(action) = ipc::parse_cli_action(&args) {
        match ipc::send_action(&action) {
            Ok(()) => return,
            Err(err) => {
                eprintln!("{err}");
                std::process::exit(2);
            }
        }
    }

    if let Err(err) = adw::init() {
        eprintln!("libadwaita init failed: {err}");
        std::process::exit(1);
    }

    let app = adw::Application::builder()
        .application_id(scs_core::APP_ID)
        .build();
    app.connect_activate(app::start);
    app.run();
}
