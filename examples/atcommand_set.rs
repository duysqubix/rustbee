//! AtCommand API Query example
//!
//! This example shows how to set the NodeId on connected XBee Device
//!
//!

use rustbee::{api, device::DigiMeshDevice};
use std::error;

#[cfg(target_os = "linux")]
static PORT: &'static str = "/dev/ttyUSB0";

#[cfg(target_os = "windows")]
static PORT: &'static str = "COM1";

static NODE_ID: &'static str = "MY_NODE";

fn main() -> Result<(), Box<dyn error::Error>> {
    // first create instance of device
    let mut device = DigiMeshDevice::new(PORT, 9600)?;

    // Construct At command and set node_id of device by supplying valid [u8] but None
    let _ = api::AtCommandFrame("NI", Some(NODE_ID.as_bytes()));

    // Now query new node_id
    let new_node_id = api::AtCommandFrame("NI", None);

    // returns dyn trait RecieveApiFrame
    let response = device.send_frame(new_node_id)?;

    // We can downcast to original AtCommandResponse struct to access members
    let atcommand_response = response.downcast_ref::<api::AtCommandResponse>();

    if let Some(obj) = atcommand_response {
        let cmd_data = &obj.command_data;
        println!("{:?}", cmd_data); // Some(b"MY_NODE")
    }

    Ok(())
}
