use gtk::gdk::{MemoryFormat, MemoryTexture};
use gtk::glib::Bytes;
use gtk::prelude::*;

pub struct PreviewView {
    pub root: gtk::Frame,
    stack: gtk::Stack,
    picture: gtk::Picture,
    title: gtk::Label,
    body: gtk::Label,
}

impl PreviewView {
    pub fn build() -> Self {
        let frame = gtk::Frame::new(None);
        frame.add_css_class("scs-preview");

        let stack = gtk::Stack::new();
        stack.set_vexpand(true);
        stack.set_hexpand(true);

        let picture = gtk::Picture::new();
        picture.set_content_fit(gtk::ContentFit::Contain);
        picture.set_can_shrink(true);
        picture.set_hexpand(true);
        picture.set_vexpand(true);

        let message = gtk::Box::new(gtk::Orientation::Vertical, 10);
        message.set_halign(gtk::Align::Center);
        message.set_valign(gtk::Align::Center);
        message.set_hexpand(true);
        message.set_vexpand(true);
        let kicker = gtk::Label::new(Some("PREVIEW"));
        kicker.add_css_class("scs-preview-kicker");
        let title = gtk::Label::new(Some("Looking for devices…"));
        title.add_css_class("scs-preview-title");
        let body = gtk::Label::new(Some(
            "Camera and audio devices are scanned in the background.",
        ));
        body.add_css_class("scs-preview-sub");
        body.set_wrap(true);
        body.set_justify(gtk::Justification::Center);
        message.append(&kicker);
        message.append(&title);
        message.append(&body);

        stack.add_named(&message, Some("message"));
        stack.add_named(&picture, Some("live"));
        frame.set_child(Some(&stack));

        Self {
            root: frame,
            stack,
            picture,
            title,
            body,
        }
    }

    pub fn show_message(&self, title: &str, body: &str) {
        self.title.set_text(title);
        self.body.set_text(body);
        self.stack.set_visible_child_name("message");
    }

    pub fn show_frame(&self, width: u32, height: u32, rgb: &[u8]) {
        let stride = width.saturating_mul(3) as usize;
        let needed = stride.saturating_mul(height as usize);
        if rgb.len() < needed || width == 0 || height == 0 {
            return;
        }
        let bytes = Bytes::from(&rgb[..needed]);
        let texture = MemoryTexture::new(
            width as i32,
            height as i32,
            MemoryFormat::R8g8b8,
            &bytes,
            stride,
        );
        self.picture.set_paintable(Some(&texture));
        self.stack.set_visible_child_name("live");
    }
}
