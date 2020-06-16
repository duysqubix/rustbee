//! Remote AtCommand API Query/Set example
//!
//! This example shows how to use ATcommand on a remote device
//!

use rustbee::{api, device::DigiMeshDevice};
use std::error;

#[cfg(target_os = "linux")]
static PORT: &'static str = "/dev/ttyUSB0";

#[cfg(target_os = "windows")]
static PORT: &'static str = "COM1";

static NODE_ID: &'static str = b"MY_NODE";
static DEST_ADDR: u64 = 0xabcdef01010203040506;

fn main() -> Result<(), Box<dyn error::Error>> {
    // first create instance of device
    let mut device = DigiMeshDevice::new(PORT, 9600)?;

    let set_all_id = api::RemoteAtCommandFrame {
        dest_addr: api::BROADCAST_ADDR,
        options: &api::RemoteCommandOptions {
            apply_changes: true,
        },
        atcmd: "ID",
        cmd_param: Some(0x7fff), // change all devices on same ID to a new ID (0x7fff)
    };

    let _ = device.send_frame(set_all_id)?;

    let get_all_id = api::RemoteAtCommandFrame {
        dest_addr: DEST_ADDR,
        options: &api::RemoteCommandOptions {
            apply_changes: true,
        },
        atcmd: "ID",
        cmd_param: None,
    };

    let response = device.send_frame(get_all_id)?;

    if let Some(resp) = response.downcast_ref::<api::RemoteAtCommandResponse>() {
        println!("{:?}", r.command_data);
    }

    Ok(())
}
