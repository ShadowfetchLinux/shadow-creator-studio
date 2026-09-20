use adw::prelude::*;

mod app;
mod live;
mod pages;
mod state;
mod widgets;
mod wizard;

fn main() {
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
