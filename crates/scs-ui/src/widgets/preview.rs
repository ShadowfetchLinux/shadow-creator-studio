use gtk::prelude::*;

pub fn build() -> gtk::Frame {
    let frame = gtk::Frame::new(None);
    frame.add_css_class("scs-preview");

    let inner = gtk::Box::new(gtk::Orientation::Vertical, 10);
    inner.set_halign(gtk::Align::Center);
    inner.set_valign(gtk::Align::Center);
    inner.set_hexpand(true);
    inner.set_vexpand(true);

    let kicker = gtk::Label::new(Some("PREVIEW"));
    kicker.add_css_class("scs-preview-kicker");

    let icon = gtk::Image::from_icon_name("camera-web-symbolic");
    icon.set_pixel_size(56);
    icon.set_opacity(0.45);

    let title = gtk::Label::new(Some("Camera / program preview"));
    title.add_css_class("scs-preview-title");

    let sub = gtk::Label::new(Some(
        "Placeholder canvas — live camera and screen preview arrive in a later milestone.",
    ));
    sub.add_css_class("scs-preview-sub");
    sub.set_wrap(true);
    sub.set_justify(gtk::Justification::Center);

    inner.append(&kicker);
    inner.append(&icon);
    inner.append(&title);
    inner.append(&sub);
    frame.set_child(Some(&inner));
    frame
}
