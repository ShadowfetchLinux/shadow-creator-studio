pub fn build() -> adw::StatusPage {
    let page = adw::StatusPage::new();
    page.set_icon_name(Some("folder-videos-symbolic"));
    page.set_title("Library");
    page.set_description(Some(
        "Takes land in the recordings folder as timestamped MKVs. A browsable library arrives in Milestone 6 — this page is not a hidden catalogue.",
    ));
    page
}
