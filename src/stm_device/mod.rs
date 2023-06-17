use std::io;

use serialport::SerialPort;
use thiserror::Error;

const ACK: u8 = 0x79;
const NACK: u8 = 0x1F;

#[derive(Debug, Error)]
pub enum StmDeviceError {
    #[error("device has already been initialised")]
    AlreadyInitialised,
    #[error("device is not initialised")]
    Uninitialised,
    #[error("expected (n)ack, recieved {0:#x}")]
    ExpectedAck(u8),
    #[error(transparent)]
    Io(#[from] io::Error),
}

pub struct StmDevice {
    port: Box<dyn SerialPort>,

    initialised: bool,
}
impl StmDevice {
    pub fn new(port: Box<dyn SerialPort>) -> Self {
        Self {
            port,
            initialised: false,
        }
    }

    pub fn initialise(&mut self) -> Result<(), StmDeviceError> {
        if self.initialised {
            Err(StmDeviceError::AlreadyInitialised)
        } else {
            // Send wake up byte
            self.port.write_all(&[0x7F])?;

            // Wait for response
            while !self.get_ack()? {}

            Ok(())
        }
    }

    fn get_ack(&mut self) -> Result<bool, StmDeviceError> {
        let mut buf = [0u8; 1];
        self.port.read_exact(&mut buf)?;

        match buf[0] {
            ACK => Ok(true),
            NACK => Ok(false),
            b => Err(StmDeviceError::ExpectedAck(b)),
        }
    }
}
