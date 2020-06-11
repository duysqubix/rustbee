#![allow(dead_code)]
//!
//! XBee API Frame
//!
//!

use byteorder::ByteOrder;
use bytes::{BufMut, BytesMut};
use rand::Rng;
use serialport::prelude::*;

static DELIM: u8 = 0x7e;
pub static BROADCAST_ADDR: u64 = 0xffff;
#[derive(Debug)]
pub enum Error {
    FrameError(String),
    PayloadError(String),
    IOError(std::io::Error),
}

impl std::error::Error for Error {}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match *self {
            Error::FrameError(ref err) => write!(f, "{}", err),
            Error::PayloadError(ref err) => write!(f, "{}", err),
            Error::IOError(ref err) => write!(f, "{}", err),
        }
    }
}

impl From<std::io::Error> for Error {
    fn from(err: std::io::Error) -> Self {
        Error::IOError(err)
    }
}

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, PartialEq)]
pub enum FrameId {
    TransmitRequest,
    TransmitStatus,
    AtCommand,
    AtCommandResponse,
    Null,
}

impl FrameId {
    fn id(&self) -> u8 {
        match *self {
            FrameId::TransmitRequest => 0x90,
            FrameId::TransmitStatus => 0x8b,
            FrameId::AtCommand => 0x08,
            FrameId::AtCommandResponse => 0x88,
            FrameId::Null => 0xff,
        }
    }
}

pub trait RecieveApiFrame: std::fmt::Debug {
    fn recieve(ser: Box<dyn SerialPort>) -> Result<Self>
    where
        Self: std::marker::Sized;

    fn id(&self) -> FrameId;
    fn summary(&self) {
        println!("{:#x?}", self);
    }
    fn payload(&self) -> Result<BytesMut>;
}

pub trait TransmitApiFrame {
    fn gen(&self) -> Result<BytesMut>;
    fn delim(&self) -> u8 {
        0x7e
    }
    fn id(&self) -> FrameId;
    fn calc_checksum(&self, frame: &[u8]) -> Result<u8> {
        if frame.len() < 5 {
            return Err(Error::FrameError(
                "Frame length does not meet minimum requirements".to_string(),
            ));
        }

        let mut checksum: u64 = 0;
        for (pos, byte) in frame.iter().enumerate() {
            if pos > 2 {
                checksum += *byte as u64;
            }
        }

        Ok(0xff - (checksum as u8))
    }

    fn gen_frame_id(&self) -> u8 {
        let mut rng = rand::thread_rng();
        let r: u8 = rng.gen();
        r
    }
}

/**
 * AtCommand Support
 *
 *
 */
pub struct AtCommand<'a> {
    pub command: &'a str,
    pub parameter: &'a Option<&'a [u8]>,
    pub rcr_len: usize, // the number of carriage returns in the reponse for this command
}

#[derive(Debug)]
pub enum AtCommands<'a> {
    Discover(Option<&'a [u8]>),
    AtCmd((&'a str, Option<&'a [u8]>)),
    CmdMode(bool),
}

impl AtCommands<'_> {
    pub fn create(&self) -> AtCommand {
        match *self {
            AtCommands::CmdMode(ref state) => match state {
                true => AtCommand {
                    command: "+++",
                    parameter: &None,
                    rcr_len: 1,
                },
                false => AtCommand {
                    command: "CN",
                    parameter: &None,
                    rcr_len: 1,
                },
            },
            AtCommands::Discover(ref param) => AtCommand {
                command: "ND",
                parameter: param,
                rcr_len: 10 + 1,
            },
            AtCommands::AtCmd((ref cmd, ref param)) => AtCommand {
                command: cmd,
                parameter: param,
                rcr_len: 1,
            },
        }
    }
}
/**
 * /AtCommand Support  
 **/

#[derive(Debug)]
pub struct NullRecieve;
impl RecieveApiFrame for NullRecieve {
    fn id(&self) -> FrameId {
        FrameId::Null
    }
    fn recieve(mut _ser: Box<dyn SerialPort>) -> Result<Self> {
        Ok(Self)
    }

    fn summary(&self) {
        println!("{:#?}", self);
    }

    fn payload(&self) -> Result<BytesMut> {
        Err(Error::FrameError(
            "Uncallabe method for Null Recieve Frame".to_string(),
        ))
    }
}

#[derive(Debug)]
pub struct TransmitStatus {
    frame_id: u8,
    transmit_retry_count: u8,
    deliver_status: u8,
    discovery_status: u8,
    payload: Option<BytesMut>,
}

impl RecieveApiFrame for TransmitStatus {
    fn id(&self) -> FrameId {
        FrameId::TransmitStatus
    }

    fn recieve(mut ser: Box<dyn SerialPort>) -> Result<Self> {
        // wait for first
        let mut response: [u8; 11] = [0; 11];
        ser.read_exact(&mut response)?;
        Ok(Self {
            frame_id: response[4],
            transmit_retry_count: response[7],
            deliver_status: response[8],
            discovery_status: response[9],
            payload: Some(BytesMut::from(&response[..])),
        })
    }

    fn payload(&self) -> Result<BytesMut> {
        match &self.payload {
            Some(p) => Ok(p.clone()),
            None => Err(Error::FrameError("Empty payload".to_string())),
        }
    }
}

/******************* AtCommand Response Frame *******************/
pub struct AtCommandResponse {
    frame_id: u8,
    at_command: Vec<u8>,
    command_status: u8,
    command_data: Option<BytesMut>,
    payload: Option<BytesMut>,
}

impl std::fmt::Debug for AtCommandResponse {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let atcmd = std::str::from_utf8(&self.at_command[..]).ok();

        let cmd_data = match self.command_data {
            Some(ref data) => format!("{:x?}", &data[..]),
            None => format!("None"),
        };

        f.debug_struct("AtCommandResponse")
            .field("FrameId", &format!("0x{:02x?}", self.frame_id))
            .field("AtCommand", &format!("{}", atcmd.unwrap()))
            .field("Command Status", &format!("{}", self.command_status))
            .field("Command Data", &cmd_data)
            .finish()
    }
}

impl RecieveApiFrame for AtCommandResponse {
    fn id(&self) -> FrameId {
        FrameId::AtCommandResponse
    }

    fn recieve(mut ser: Box<dyn SerialPort>) -> Result<Self> {
        let mut buffer = BytesMut::with_capacity(256);
        let mut mini_buf: [u8; 1] = [0];
        let old_timeout = ser.timeout();
        ser.set_timeout(std::time::Duration::from_millis(100));
        loop {
            if let Err(ref err) = ser.read_exact(&mut mini_buf) {
                println!("{:?}", err);
                break;
            }
            buffer.put_u8(mini_buf[0]);
        }
        let mut cmd_data = None;
        if buffer.len() > 9 {
            cmd_data = Some(BytesMut::from(&buffer[8..buffer.len() - 1]));
        }

        let mut at_cmd: Vec<u8> = Vec::new();
        at_cmd.push(buffer[5]);
        at_cmd.push(buffer[6]);
        ser.set_timeout(old_timeout);
        Ok(Self {
            frame_id: buffer[4],
            at_command: at_cmd,
            command_status: buffer[7],
            command_data: cmd_data,
            payload: Some(buffer),
        })
    }

    fn payload(&self) -> Result<BytesMut> {
        match &self.payload {
            Some(p) => Ok(p.clone()),
            None => Err(Error::FrameError("Emtpy payload".to_string())),
        }
    }
}

/*******************************************************************/

pub enum MessagingMode {
    PointToPoint,
    Repeater,
    DigiMesh,
}

pub struct TransmitRequestOptions {
    pub disable_ack: bool,
    pub disable_route_discovery: bool,
    pub enable_unicast_nack: bool,
    pub enable_unicast_trace_route: bool,
    pub mode: MessagingMode,
}

impl TransmitRequestOptions {
    pub fn compile(&self) -> u8 {
        let mut val: u8 = 0;

        if self.disable_ack == true {
            val |= 1 << 0;
        }
        if self.disable_route_discovery == true {
            val |= 1 << 1;
        }
        if self.enable_unicast_nack == true {
            val |= 1 << 2;
        }

        if self.enable_unicast_trace_route == true {
            val |= 1 << 3;
        }

        match self.mode {
            MessagingMode::PointToPoint => (0x1 << 6) | val,
            MessagingMode::Repeater => (0x2 << 6) | val,
            MessagingMode::DigiMesh => (0x3 << 6) | val,
        }
    }
}

pub struct TransmitRequestFrame<'a> {
    pub dest_addr: u64,
    pub broadcast_radius: u8,
    pub options: Option<&'a TransmitRequestOptions>,
    pub payload: &'a [u8],
}

impl TransmitApiFrame for TransmitRequestFrame<'_> {
    fn id(&self) -> FrameId {
        FrameId::TransmitRequest
    }

    fn gen(&self) -> Result<BytesMut> {
        let mut packet = BytesMut::new();
        let mut rng = rand::thread_rng();
        if self.payload.len() > 65535 - 112 {
            return Err(Error::PayloadError("Payload exceeds max size".to_string()));
        }

        let frame_id: u8 = rng.gen();

        packet.put_u8(self.delim());
        packet.put_u16((self.payload.len() as u16) + (0x0e as u16));
        packet.put_u8(0x10);
        packet.put_u8(frame_id);
        packet.put_u64(self.dest_addr);
        packet.put_u16(0xfffe);
        packet.put_u8(self.broadcast_radius);

        match self.options {
            Some(opts) => packet.put_u8(opts.compile()),
            None => packet.put_u8(0),
        }
        packet.put(self.payload);

        let chksum = self.calc_checksum(&packet[..])?;
        packet.put_u8(chksum);

        Ok(packet)
    }
}

pub struct AtCommandFrame<'a>(pub &'a str, pub Option<&'a [u8]>);
impl TransmitApiFrame for AtCommandFrame<'_> {
    fn id(&self) -> FrameId {
        FrameId::AtCommand
    }

    fn gen(&self) -> Result<BytesMut> {
        let mut packet = BytesMut::with_capacity(9);
        let frame_id: u8 = self.gen_frame_id();
        packet.put_u8(DELIM);
        packet.put_u16(0); // length 0 just a placeholder
        packet.put_u8(self.id().id());
        packet.put_u8(frame_id);
        packet.put(self.0.as_bytes());
        if let Some(param) = self.1 {
            packet.put(&param[..])
        }

        let packet_len = (packet.len() - 3) as u16;
        packet[1] = (packet_len >> 8) as u8;
        packet[2] = (packet_len & 0xff) as u8;
        let chksum = self.calc_checksum(&packet[..])?;
        packet.put_u8(chksum);
        println!("{:?}", packet);
        println!("{:x?}", &packet[..]);
        Ok(packet)
    }
}
