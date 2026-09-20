use std::cell::{Cell, RefCell};
use std::rc::Rc;
use std::sync::mpsc::Receiver;
use std::time::Duration;

use adw::prelude::*;
use gtk::gio;
use gtk::glib;
use scs_capture::DeviceInventory;
use scs_system::SystemSnapshot;

use crate::live;
use crate::pages;
use crate::state::{app_subtitle, StudioState};
use crate::wizard;

pub fn start(app: &adw::Application) {
    let state = Rc::new(StudioState::load());
    adw::StyleManager::default().set_color_scheme(adw::ColorScheme::ForceDark);
    load_css();

    let window = adw::ApplicationWindow::builder()
        .application(app)
        .title("Shadow Creator Studio")
        .default_width(1320)
        .default_height(860)
        .build();
    window.add_css_class("scs-window");

    let toolbar = adw::ToolbarView::new();
    let header = adw::HeaderBar::new();
    let title = gtk::Label::new(Some(&app_subtitle()));
    title.add_css_class("heading");
    header.set_title_widget(Some(&title));

    let menu = gio::Menu::new();
    menu.append(Some("Setup wizard"), Some("win.wizard"));
    menu.append(Some("About"), Some("win.about"));
    let menu_btn = gtk::MenuButton::builder()
        .icon_name("open-menu-symbolic")
        .menu_model(&menu)
        .build();
    header.pack_end(&menu_btn);
    toolbar.add_top_bar(&header);

    let sidebar_list = gtk::ListBox::new();
    sidebar_list.add_css_class("navigation-sidebar");
    sidebar_list.add_css_class("scs-sidebar");
    sidebar_list.set_selection_mode(gtk::SelectionMode::Single);
    add_nav(&sidebar_list, "Record", "camera-video-symbolic");
    add_nav(&sidebar_list, "Library", "folder-videos-symbolic");
    add_nav(&sidebar_list, "Teleprompter", "document-edit-symbolic");
    add_nav(&sidebar_list, "Settings", "emblem-system-symbolic");
    add_nav(&sidebar_list, "Diagnostics", "dialog-information-symbolic");
    if let Some(row) = sidebar_list.row_at_index(0) {
        sidebar_list.select_row(Some(&row));
    }

    let record = Rc::new(pages::record::RecordPage::new(&state));
    let diagnostics = pages::diagnostics::DiagnosticsPage::new(&state, &window);
    let stack = gtk::Stack::new();
    stack.set_transition_type(gtk::StackTransitionType::Crossfade);
    stack.add_titled(&record.root, Some("record"), "Record");
    stack.add_titled(&pages::library::build(), Some("library"), "Library");
    stack.add_titled(
        &pages::teleprompter::build(),
        Some("teleprompter"),
        "Teleprompter",
    );
    stack.add_titled(
        &pages::settings::build(&state, &window),
        Some("settings"),
        "Settings",
    );
    stack.add_titled(&diagnostics.root, Some("diagnostics"), "Diagnostics");

    let stack_nav = stack.clone();
    sidebar_list.connect_row_activated(move |_, row| {
        let name = match row.index() {
            0 => "record",
            1 => "library",
            2 => "teleprompter",
            3 => "settings",
            4 => "diagnostics",
            _ => "record",
        };
        stack_nav.set_visible_child_name(name);
    });

    let split = adw::NavigationSplitView::new();
    split.set_min_sidebar_width(196.0);
    split.set_sidebar(Some(&adw::NavigationPage::new(&sidebar_list, "Studio")));
    split.set_content(Some(&adw::NavigationPage::new(&stack, "Record")));
    toolbar.set_content(Some(&split));
    window.set_content(Some(&toolbar));

    let wizard_action = gio::SimpleAction::new("wizard", None);
    let window_w = window.clone();
    let state_w = Rc::clone(&state);
    wizard_action.connect_activate(move |_, _| {
        wizard::present(&window_w, &state_w);
    });
    window.add_action(&wizard_action);

    let about_action = gio::SimpleAction::new("about", None);
    let window_a = window.clone();
    about_action.connect_activate(move |_, _| {
        let about = adw::AboutWindow::new();
        about.set_transient_for(Some(&window_a));
        about.set_modal(true);
        about.set_application_name("Shadow Creator Studio");
        about.set_version(scs_core::APP_VERSION);
        about.set_developer_name("Shadowfetch");
        about.set_comments(
            "Milestone 2: live camera preview and audio meters. Recording and live streaming are not implemented yet.",
        );
        about.set_license_type(gtk::License::MitX11);
        about.present();
    });
    window.add_action(&about_action);

    window.present();

    if !state.wizard_completed() {
        wizard::present(&window, &state);
    }

    let state_t = Rc::clone(&state);
    let record_t = Rc::clone(&record);
    let discover_rx: Rc<RefCell<Option<Receiver<DeviceInventory>>>> =
        Rc::new(RefCell::new(Some(live::spawn_discover())));
    let ticks = Rc::new(Cell::new(0u32));
    let last_snap: Rc<RefCell<SystemSnapshot>> =
        Rc::new(RefCell::new(state.monitor.borrow_mut().snapshot()));

    glib::timeout_add_local(Duration::from_millis(50), move || {
        let incoming = {
            let guard = discover_rx.borrow();
            guard.as_ref().and_then(|rx| rx.try_recv().ok())
        };
        if let Some(inventory) = incoming {
            record_t.apply_inventory(inventory, &state_t);
            *discover_rx.borrow_mut() = None;
        }

        let n = ticks.get().wrapping_add(1);
        ticks.set(n);
        if n % 40 == 0 {
            *last_snap.borrow_mut() = state_t.monitor.borrow_mut().snapshot();
        }
        if n % 400 == 0 && discover_rx.borrow().is_none() {
            *discover_rx.borrow_mut() = Some(live::spawn_discover());
        }

        record_t.refresh(&state_t, &last_snap.borrow());
        glib::ControlFlow::Continue
    });
}

fn add_nav(list: &gtk::ListBox, title: &str, icon: &str) {
    let row = adw::ActionRow::builder()
        .title(title)
        .activatable(true)
        .build();
    row.add_prefix(&gtk::Image::from_icon_name(icon));
    list.append(&row);
}

fn load_css() {
    let provider = gtk::CssProvider::new();
    provider.load_from_string(include_str!("style.css"));
    if let Some(display) = gtk::gdk::Display::default() {
        gtk::style_context_add_provider_for_display(
            &display,
            &provider,
            gtk::STYLE_PROVIDER_PRIORITY_APPLICATION,
        );
    }
}
