use adw::prelude::*;

pub fn build() -> adw::StatusPage {
    let page = adw::StatusPage::new();
    page.set_icon_name(Some("folder-videos-symbolic"));
    page.set_title("Library");
    page.set_description(Some(
        "Recordings will land here in Milestone 6. Nothing is catalogued yet — this is not a hidden library.",
    ));
    page
}
