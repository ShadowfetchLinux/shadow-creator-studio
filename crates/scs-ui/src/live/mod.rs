pub mod jobs;
pub mod meters;
pub mod plan;
pub mod portal;
pub mod preview;
pub mod record;

use std::sync::mpsc::{self, Receiver};
use std::thread;

use scs_capture::DeviceInventory;

pub fn spawn_discover() -> Receiver<DeviceInventory> {
    let (tx, rx) = mpsc::channel();
    thread::Builder::new()
        .name("scs-discover".into())
        .spawn(move || {
            let inventory = DeviceInventory::discover();
            let _ = tx.send(inventory);
        })
        .ok();
    rx
}
