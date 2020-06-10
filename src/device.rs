use crate::api::{self, AtCommand, AtCommands, RecieveApiFrame};
use bytes::{BufMut, BytesMut};
use serialport::*;
use std::thread;
use std::time::Duration;

#[derive(Debug)]
pub enum Error {
    SerialError(serialport::Error),
    IOError(std::io::Error),
    DecodeError(std::str::Utf8Error),
    ApiError(api::Error),
    InvalidMode(String),
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
        }
    }
}

impl std::error::Error for Error {}

pub type Result<T> = std::result::Result<T, Error>;

pub struct DigiMeshDevice {
    pub serial: Box<dyn SerialPort>,
    pub rx_buf: BytesMut,
    pub tx_buf: BytesMut,
}

impl DigiMeshDevice {
    pub fn new() -> Result<Self> {
        let settings = SerialPortSettings {
            baud_rate: 9600,
            data_bits: DataBits::Eight,
            flow_control: FlowControl::None,
            parity: Parity::None,
            stop_bits: StopBits::One,
            timeout: Duration::from_millis(20000),
        };

        Ok(Self {
            serial: serialport::open_with_settings("/dev/ttyUSB0", &settings)?,
            rx_buf: BytesMut::with_capacity(128),
            tx_buf: BytesMut::with_capacity(128),
        })
    }

    pub fn send<'a>(&mut self, data: &'a [u8]) -> Result<usize> {
        Ok(self.serial.write(data)?)
    }

    pub fn send_frame<T: api::TransmitApiFrame>(
        &mut self,
        frame: T,
    ) -> Result<Box<dyn api::RecieveApiFrame>> {
        let packet = frame.gen()?; // creats bytes mut
        self.serial.write(&packet[..])?;
        let response: Box<dyn api::RecieveApiFrame>;
        if frame.id() == api::FrameId::TransmitRequest {
            response = Box::new(api::TransmitStatus::recieve(self.serial.try_clone()?)?);
        } else {
            response = Box::new(api::NullRecieve::recieve(self.serial.try_clone()?)?);
        }
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
