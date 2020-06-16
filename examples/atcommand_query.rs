//! AtCommand API Query example
//!
//! This example shows how to read the NodeId from connected XBee Device
//!
//!

use rustbee::{api, device::DigiMeshDevice};
use std::error;

#[cfg(target_os = "linux")]
static PORT: &'static str = "/dev/ttyUSB0";

#[cfg(target_os = "windows")]
static PORT: &'static str = "COM1";

fn main() -> Result<(), Box<dyn error::Error>> {
    // first create instance of device
    let mut device = DigiMeshDevice::new(PORT, 9600)?;

    // Construct At command to ask for node_id of device
    let node_id_request = api::AtCommandFrame("NI", None);
    // returns dyn trait RecieveApiFrame
    let response = device.send_frame(node_id_request)?;
    // We can downcast to original AtCommandResponse struct to access members
    let atcommand_response = response.downcast_ref::<api::AtCommandResponse>();

    if let Some(obj) = atcommand_response {
        let cmd_data = &obj.command_data;
        println!("{:?}", cmd_data);
    }

    Ok(())
}
