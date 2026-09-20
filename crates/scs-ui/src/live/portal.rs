use std::sync::mpsc::{self, Receiver};
use std::sync::{Arc, Mutex};
use std::thread;

use gtk::gio;
use gtk::gio::prelude::*;
use glib::{Variant, VariantDict};
use scs_capture::{parse_stream_props, DesktopStream, SOURCE_MONITOR, SOURCE_WINDOW};

const DEST: &str = "org.freedesktop.portal.Desktop";
const PATH: &str = "/org/freedesktop/portal/desktop";
const IFACE: &str = "org.freedesktop.portal.ScreenCast";

#[derive(Debug, Clone)]
pub struct SharedDesktop {
    pub stream: DesktopStream,
    pub fd: i32,
}

pub fn request_share(want_window: bool) -> Receiver<Result<SharedDesktop, String>> {
    let (tx, rx) = mpsc::channel();
    thread::Builder::new()
        .name("scs-portal".into())
        .spawn(move || {
            let _ = tx.send(run_share(want_window));
        })
        .ok();
    rx
}

fn run_share(want_window: bool) -> Result<SharedDesktop, String> {
    let ctx = glib::MainContext::new();
    let _guard = ctx
        .acquire()
        .map_err(|_| "Could not take a D-Bus context.".to_string())?;
    let conn = gio::bus_get_sync(gio::BusType::Session, gio::Cancellable::NONE)
        .map_err(|e| format!("No session bus: {e}"))?;

    let create = VariantDict::new(None);
    create.insert("handle_token", unique_token().as_str());
    create.insert("session_handle_token", unique_token().as_str());
    let create_path = call_simple(&conn, "CreateSession", Variant::tuple_from_iter([create.end()]))?;
    let created = wait_response(&conn, &ctx, &create_path)?;
    let session = created
        .lookup_value("session_handle", None)
        .and_then(|v| v.get::<String>())
        .ok_or_else(|| "The portal did not return a session.".to_string())?;

    let types = if want_window {
        SOURCE_MONITOR | SOURCE_WINDOW
    } else {
        SOURCE_MONITOR
    };
    let select = VariantDict::new(None);
    select.insert("handle_token", unique_token().as_str());
    select.insert("types", types);
    select.insert("multiple", false);
    select.insert("cursor_mode", 2u32);
    select.insert("persist_mode", 1u32);
    let session_var = object_path_variant(&session)?;
    let select_path = call_simple(
        &conn,
        "SelectSources",
        Variant::tuple_from_iter([session_var.clone(), select.end()]),
    )?;
    wait_response(&conn, &ctx, &select_path)?;

    let start = VariantDict::new(None);
    start.insert("handle_token", unique_token().as_str());
    let start_path = call_simple(
        &conn,
        "Start",
        Variant::tuple_from_iter([session_var, Variant::from(""), start.end()]),
    )?;
    let started = wait_response(&conn, &ctx, &start_path)?;
    let stream = parse_started(&started)?;
    let fd = open_pw_remote(&conn, &session)?;
    Ok(SharedDesktop { stream, fd })
}

fn object_path_variant(path: &str) -> Result<Variant, String> {
    let clean = path.trim().trim_matches('\'');
    Variant::parse(
        Some(glib::VariantTy::OBJECT_PATH),
        &format!("objectpath '{clean}'"),
    )
    .or_else(|_| Variant::parse(Some(glib::VariantTy::OBJECT_PATH), &format!("'{clean}'")))
    .map_err(|e| format!("Invalid portal session path: {e}"))
}

fn unique_token() -> String {
    format!(
        "scs{:x}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(1)
    )
}

fn call_simple(conn: &gio::DBusConnection, method: &str, body: Variant) -> Result<String, String> {
    let reply = conn
        .call_sync(
            Some(DEST),
            PATH,
            IFACE,
            method,
            Some(&body),
            Some(glib::VariantTy::new("(o)").expect("(o)")),
            gio::DBusCallFlags::NONE,
            120_000,
            gio::Cancellable::NONE,
        )
        .map_err(human_portal)?;
    reply
        .child_value(0)
        .get::<String>()
        .ok_or_else(|| "Portal request path missing.".into())
}

fn wait_response(
    conn: &gio::DBusConnection,
    ctx: &glib::MainContext,
    request_path: &str,
) -> Result<VariantDict, String> {
    let loop_ = glib::MainLoop::new(Some(ctx), false);
    let result = Arc::new(Mutex::new(None::<Result<VariantDict, String>>));
    let result_c = Arc::clone(&result);
    let loop_c = loop_.clone();
    conn.signal_subscribe(
        Some(DEST),
        Some("org.freedesktop.portal.Request"),
        Some("Response"),
        Some(request_path),
        None,
        gio::DBusSignalFlags::NONE,
        move |_, _, _, _, _, params| {
            let code = params.child_value(0).get::<u32>().unwrap_or(2);
            let dict = VariantDict::new(Some(&params.child_value(1)));
            let parsed = if code == 0 {
                Ok(dict)
            } else if code == 1 {
                Err("Screen share was cancelled.".into())
            } else {
                Err("The desktop portal could not start a screen share.".into())
            };
            *result_c.lock().unwrap() = Some(parsed);
            loop_c.quit();
        },
    );
    let timeout = Arc::clone(&result);
    let loop_t = loop_.clone();
    glib::timeout_add_seconds_local(120, move || {
        let mut guard = timeout.lock().unwrap();
        if guard.is_none() {
            *guard = Some(Err(
                "Timed out waiting for the screen-share dialog.".into(),
            ));
            loop_t.quit();
        }
        glib::ControlFlow::Break
    });
    loop_.run();
    let parsed = result
        .lock()
        .unwrap()
        .take()
        .unwrap_or_else(|| Err("No portal response.".into()));
    parsed
}

fn parse_started(dict: &VariantDict) -> Result<DesktopStream, String> {
    let streams = dict
        .lookup_value("streams", None)
        .ok_or_else(|| "The portal returned no streams.".to_string())?;
    let first = streams
        .try_child_value(0)
        .ok_or_else(|| "The portal returned an empty stream list.".to_string())?;
    let node_id = first.child_value(0).get::<u32>().unwrap_or(0);
    let props = VariantDict::new(Some(&first.child_value(1)));
    let (width, height) = props
        .lookup_value("size", None)
        .map(|s| {
            (
                s.child_value(0).get::<i32>().unwrap_or(1920) as u32,
                s.child_value(1).get::<i32>().unwrap_or(1080) as u32,
            )
        })
        .unwrap_or((1920, 1080));
    let source_type = props
        .lookup_value("source_type", None)
        .and_then(|v| v.get::<u32>());
    let mut stream = parse_stream_props(node_id, Some(width), Some(height), source_type);
    stream.restore_token = dict
        .lookup_value("restore_token", None)
        .and_then(|v| v.get::<String>());
    if stream.node_id == 0 {
        return Err("The portal stream had no PipeWire node.".into());
    }
    Ok(stream)
}

fn open_pw_remote(conn: &gio::DBusConnection, session: &str) -> Result<i32, String> {
    let empty = VariantDict::new(None).end();
    let body = Variant::tuple_from_iter([object_path_variant(session)?, empty]);
    let fd_list = gio::UnixFDList::new();
    let (reply, fds) = conn
        .call_with_unix_fd_list_sync(
            Some(DEST),
            PATH,
            IFACE,
            "OpenPipeWireRemote",
            Some(&body),
            Some(glib::VariantTy::new("(h)").expect("(h)")),
            gio::DBusCallFlags::NONE,
            30_000,
            Some(&fd_list),
            gio::Cancellable::NONE,
        )
        .map_err(|e| format!("Could not open the PipeWire remote: {e}"))?;
    let handle = reply.child_value(0).get::<i32>().unwrap_or(0);
    let fds = fds.ok_or_else(|| "Portal did not pass a PipeWire file descriptor.".to_string())?;
    let raw = fds
        .get(handle.max(0))
        .or_else(|_| fds.get(0))
        .map_err(|_| "Portal did not pass a PipeWire file descriptor.".to_string())?;
    let _ = rustix::io::fcntl_setfd(
        unsafe { rustix::fd::BorrowedFd::borrow_raw(raw) },
        rustix::io::FdFlags::empty(),
    );
    Ok(raw)
}

fn human_portal(err: glib::Error) -> String {
    let msg = err.message().to_string();
    if msg.to_ascii_lowercase().contains("cancel") {
        "Screen share was cancelled.".into()
    } else {
        format!("Desktop portal: {msg}")
    }
}
