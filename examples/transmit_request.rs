//! Transmit Request Example 
//!
//! This example shows how to send two transmit request packets. One that will
//! broadcast through all connected frames within same network ID and the other to 
//! a specific addr
//!
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

    let broadcast = api::TransmitRequestFrame{
        dest_addr: api::BROADCAST_ADDR,
        broadcast_radius: 0,
        options: Some(
            &api::TransmitRequestOptions{
                disable_ack: false,
                disable_route_discovery: false,
                enable_unicast_ack: false,
                enable_unicast_trace_route: false,
                api::MessagesMode::DigiMesh,

            }
        ),
        payload: b"HELLO FROM RUST!!"
    };
    
    // all devices with same Network ID will have the payload broadcasted too.
    let transmit_status = device.send_frame(broadcast)?; 


    let unicast_msg = api::TransmitRequestFrame{
        dest_addr: DEST_ADDR,
        broadcast_radius: 0,
        options: Some(
            &api::TransmitRequestOptions{
                disable_ack: false,
                disable_route_discovery: false,
                enable_unicast_ack: false,
                enable_unicast_trace_route: false,
                api::MessagesMode::DigiMesh,

            }
        ),

        payload: b"Hello individual device!"
    };   
   
    // will send payload to DEST_ADDR if it is found on the same network ID
    let transmit_status = device.send_frame(unicast_msg)?;
    Ok(())
}
