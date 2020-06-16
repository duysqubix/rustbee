use crate::api::{self, AtCommand, AtCommands, RecieveApiFrame, TransmitApiFrame};
use bytes::{BufMut, BytesMut};
use serialport::*;
use std::convert::TryFrom;
use std::thread;
use std::time::Duration;

#[derive(Debug)]
pub enum Error {
    SerialError(serialport::Error),
    IOError(std::io::Error),
    DecodeError(std::str::Utf8Error),
    ApiError(api::Error),
    InvalidMode(String),
    DiscoveryError,
}

impl From<serialport::Error> for Error {
    fn from(err: serialport::Error) -> Self {
        Error::SerialError(err)
    }
}

impl From<std::io::Error> for Error {
    fn from(err: std::io::Error) -> Self {
        Error::IOError(err)
    }
}

impl From<std::str::Utf8Error> for Error {
    fn from(err: std::str::Utf8Error) -> Self {
        Error::DecodeError(err)
    }
}

impl From<api::Error> for Error {
    fn from(err: api::Error) -> Self {
        Error::ApiError(err)
    }
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match *self {
            Error::SerialError(ref err) => write!(f, "{}", err),
            Error::IOError(ref err) => write!(f, "{}", err),
            Error::DecodeError(ref err) => write!(f, "{}", err),
            Error::InvalidMode(ref err) => write!(f, "{}", err),
            Error::ApiError(ref err) => write!(f, "{}", err),
            Error::DiscoveryError => write!(f, "Could not complete discovery mode"),
        }
    }
}

impl std::error::Error for Error {}

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug)]
pub struct RemoteDigiMeshDevice {
    pub addr_64bit: u64,
    pub node_id: String,
    pub firmware_version: Option<u16>,
    pub hardware_version: Option<u16>,
}

pub struct DigiMeshDevice {
    pub addr_64bit: Option<u64>,
    pub node_id: Option<String>,
    pub firmware_version: Option<u16>,
    pub hardware_version: Option<u16>,
    pub nodes: Option<Vec<RemoteDigiMeshDevice>>,
    serial: Box<dyn SerialPort>,
    rx_buf: BytesMut,
    tx_buf: BytesMut,
}

impl std::fmt::Debug for DigiMeshDevice {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DigiMeshDevice")
            .field("addr_64bit", &format!("{:x?}", self.addr_64bit))
            .field("node_id", &format!("{:?}", self.node_id))
            .field("firmware_version", &format!("{:x?}", self.firmware_version))
            .field("hardware_version", &format!("{:x?}", self.hardware_version))
            .finish()
    }
}

impl DigiMeshDevice {
    pub fn new<'a>(port: &'a str, baud: u32) -> Result<Self> {
        let settings = SerialPortSettings {
            baud_rate: baud,
            data_bits: DataBits::Eight,
            flow_control: FlowControl::None,
            parity: Parity::None,
            stop_bits: StopBits::One,
            timeout: Duration::from_millis(20000),
        };

        let mut device = Self {
            serial: serialport::open_with_settings(port, &settings)?,
            rx_buf: BytesMut::with_capacity(128),
            tx_buf: BytesMut::with_capacity(128),
            addr_64bit: None,
            node_id: None,
            firmware_version: None,
            hardware_version: None,
            nodes: None,
        };
        let addr = device.get_64bit_addr()?;
        let node_id = device.get_node_id()?;
        let hw_version = device.get_hardware_version()?;
        let fw_version = device.get_firmware_version()?;

        device.addr_64bit = Some(addr);
        device.node_id = Some(node_id);
        device.hardware_version = Some(hw_version);
        device.firmware_version = Some(fw_version);

        Ok(device)
    }

    pub fn get_firmware_version(&mut self) -> Result<u16> {
        if let None = self.firmware_version {
            let fw = self.send_frame(api::AtCommandFrame("VR", None))?;
            let fw = fw
                .downcast_ref::<api::AtCommandResponse>()
                .ok_or(Error::ApiError(api::Error::DerefError))?
                .command_data
                .as_ref()
                .unwrap();
            return Ok(u16::from_be_bytes(<[u8; 2]>::try_from(&fw[..]).unwrap()));
        }
        Ok(self.firmware_version.unwrap())
    }

    pub fn get_hardware_version(&mut self) -> Result<u16> {
        if let None = self.hardware_version {
            let fw = self.send_frame(api::AtCommandFrame("HV", None))?;
            let fw = fw
                .downcast_ref::<api::AtCommandResponse>()
                .ok_or(Error::ApiError(api::Error::DerefError))?
                .command_data
                .as_ref()
                .unwrap();
            return Ok(u16::from_be_bytes(<[u8; 2]>::try_from(&fw[..]).unwrap()));
        }
        Ok(self.hardware_version.unwrap())
    }

    pub fn get_node_id(&mut self) -> Result<String> {
        if let None = self.node_id {
            // get node_id
            let node_id = self.send_frame(api::AtCommandFrame("NI", None))?;
            let node_id = node_id
                .downcast_ref::<api::AtCommandResponse>()
                .ok_or(Error::ApiError(api::Error::DerefError))?
                .command_data
                .as_ref()
                .unwrap();
            let node_id = std::str::from_utf8(&node_id[..])?;

            return Ok(String::from(node_id));
        }
        Ok(self.node_id.clone().unwrap())
    }

    pub fn get_64bit_addr(&mut self) -> Result<u64> {
        if let None = self.addr_64bit {
            // get 64bit addr of device
            let sh = self.send_frame(api::AtCommandFrame("SH", None))?;
            let sl = self.send_frame(api::AtCommandFrame("SL", None))?;

            let sh = sh
                .downcast_ref::<api::AtCommandResponse>()
                .ok_or(Error::ApiError(api::Error::DerefError))?;
            let sl = sl
                .downcast_ref::<api::AtCommandResponse>()
                .ok_or(Error::ApiError(api::Error::DerefError))?;
            let upper = sh.command_data.as_ref().unwrap();
            let lower = sl.command_data.as_ref().unwrap();
            let upper = u32::from_be_bytes(<[u8; 4]>::try_from(&upper[..]).unwrap()); // messy but works
            let lower = u32::from_be_bytes(<[u8; 4]>::try_from(&lower[..]).unwrap());

            let addr_64bit: u64 = ((upper as u64) << 32) | (lower as u64);
            return Ok(addr_64bit);
        }
        Ok(self.addr_64bit.unwrap())
    }

    pub fn send<'a>(&mut self, data: &'a [u8]) -> Result<usize> {
        Ok(self.serial.write(data)?)
    }

    pub fn discover_nodes(&mut self, timeout: Option<std::time::Duration>) -> Result<()> {
        let discover_cmd = api::AtCommandFrame("ND", None).gen()?;
        self.serial.write(&discover_cmd[..])?;
        let old_timeout = self.serial.timeout();

        match timeout {
            Some(t) => self.serial.set_timeout(t)?,
            None => self
                .serial
                .set_timeout(std::time::Duration::from_secs(15))?,
        }

        let mut api_responses: Vec<api::AtCommandResponse> = Vec::new();
        let mut remote_devices: Vec<RemoteDigiMeshDevice> = Vec::new();
        let mut break_loop = false;
        loop {
            if break_loop == true {
                break;
            }
            println!("Iteration...");
            let response = api::AtCommandResponse::recieve(self.serial.try_clone()?);

            match response {
                Ok(resp) => api_responses.push(resp),
                Err(_) => {
                    break_loop = true;
                }
            }
        }
        self.serial.set_timeout(old_timeout)?;

        if api_responses.len() > 0 {
            println!("{:?}", api_responses);

            for rd in api_responses.iter() {
                let ref buf = &rd.command_data.as_ref().unwrap();
                let addr = u64::from_be_bytes(<[u8; 8]>::try_from(&buf[2..10]).unwrap());
                let mut end_idx = 10;
                for i in 10..buf.len() - 1 {
                    if buf[i] == 0 {
                        break;
                    }
                    end_idx += 1;
                }
                let node_id = std::str::from_utf8(&buf[10..end_idx])?;
                let d = RemoteDigiMeshDevice {
                    addr_64bit: addr,
                    node_id: String::from(node_id),
                    firmware_version: None,
                    hardware_version: None,
                };

                remote_devices.push(d);
            }
            self.nodes = Some(remote_devices);
            return Ok(());
        }
        Err(Error::DiscoveryError)
    }

    pub fn send_frame<T: api::TransmitApiFrame>(
        &mut self,
        frame: T,
    ) -> Result<Box<dyn api::RecieveApiFrame>> {
        let packet = frame.gen()?; // creats bytes mut
        self.serial.write(&packet[..])?;
        let response: Box<dyn api::RecieveApiFrame>;

        let old_timeout = self.serial.timeout();
        if frame.id() == api::FrameId::TransmitRequest {
            response = Box::new(api::TransmitStatus::recieve(self.serial.try_clone()?)?);
        } else if frame.id() == api::FrameId::AtCommand {
            self.serial
                .set_timeout(std::time::Duration::from_millis(100))?;
            response = Box::new(api::AtCommandResponse::recieve(self.serial.try_clone()?)?);
        } else if frame.id() == api::FrameId::RemoteAtCommand {
            self.serial
                .set_timeout(std::time::Duration::from_millis(3000))?;
            response = Box::new(api::RemoteAtCommandResponse::recieve(
                self.serial.try_clone()?,
            )?);
        } else {
            response = Box::new(api::NullRecieve::recieve(self.serial.try_clone()?)?);
        }

        self.serial.set_timeout(old_timeout)?;
        Ok(response)
    }

    /// send an AT command and returns the result
    pub fn atcmd<'a>(&mut self, atcmd: &'a AtCommand) -> Result<()> {
        self.tx_buf.clear();
        self.rx_buf.clear();

        if atcmd.command != "+++" {
            self.tx_buf.put(&b"AT"[..]);
            self.tx_buf.put(atcmd.command.as_bytes());

            if let Some(data) = &atcmd.parameter {
                self.tx_buf.put(&data[..]);
            }
            self.tx_buf.put_u8(0x0d);
        } else {
            self.tx_buf.put(atcmd.command.as_bytes());
        }

        self.serial.write(&self.tx_buf[..])?;
        let mut buf: [u8; 1] = [0; 1];
        let mut cr_counter = 0;
        loop {
            if buf[0] == b'\r' {
                cr_counter += 1;
                if cr_counter == atcmd.rcr_len {
                    break;
                }
            }
            self.serial.read_exact(&mut buf)?;
            self.rx_buf.put_u8(buf[0]);
        }

        if self.rx_buf.len() < 1 {
            return Err(Error::IOError(std::io::Error::new(
                std::io::ErrorKind::UnexpectedEof,
                "RX buf empty",
            )));
        }
        Ok(())
    }

    pub fn command_mode(&mut self, mode: bool) -> Result<()> {
        match mode {
            true => {
                thread::sleep(Duration::from_millis(1000));
                self.atcmd(&AtCommands::CmdMode(true).create())?;
                thread::sleep(Duration::from_millis(1000));
            }
            false => {
                self.atcmd(&AtCommands::CmdMode(false).create())?;
            }
        }
        Ok(())
    }
}
