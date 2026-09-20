use adw::prelude::*;

pub fn build() -> adw::StatusPage {
    let page = adw::StatusPage::new();
    page.set_icon_name(Some("document-edit-symbolic"));
    page.set_title("Teleprompter");
    page.set_description(Some(
        "The teleprompter arrives in Milestone 7. No script is loaded and nothing is scrolling.",
    ));
    page
}
