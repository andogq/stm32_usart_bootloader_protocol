mod protocol;

use std::io;

use serialport::SerialPort;
use thiserror::Error;

use self::protocol::{Command, CommonCommand, Protocol, ProtocolError, ProtocolVersion};

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
    #[error("failed to run command: {0:?}")]
    CommandFail(Command),
    #[error("retries exceeded to run command: {0:?}")]
    RetryExceeded(Command),
    #[error(transparent)]
    ProtocolError(#[from] ProtocolError),
    #[error(transparent)]
    Io(#[from] io::Error),
}

pub struct StmDevice {
    port: Box<dyn SerialPort>,

    initialised: bool,
    protocol_version: ProtocolVersion,

    retry_amount: usize,
}
impl StmDevice {
    pub fn new(port: Box<dyn SerialPort>) -> Self {
        Self {
            port,
            initialised: false,
            protocol_version: ProtocolVersion::Unknown,
            retry_amount: 5,
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

            // Indicate that device is now initialised
            self.initialised = true;

            Ok(())
        }
    }

    pub fn get_protocol(&mut self) -> Result<Protocol, StmDeviceError> {
        // Send the command
        self.retry_command(Command::Common(CommonCommand::Get))?;

        // Get the number of bytes in the response
        let response_bytes = self.read_byte()? + 1;
        let response = self.read_bytes(response_bytes as usize)?;

        // Attempt to determine protocol version
        Ok(Protocol::try_from(response.as_slice())?)
    }

    fn retry_command(&mut self, command: Command) -> Result<(), StmDeviceError> {
        for _ in 0..self.retry_amount {
            match self.send_command(command) {
                Ok(true) => return Ok(()),
                Ok(false) => return Err(StmDeviceError::CommandFail(command)),
                Err(_) => (),
            }
        }

        Err(StmDeviceError::RetryExceeded(command))
    }

    /// Sends a command to the device, and waits for the ACK. Returns `true` if the device
    /// acknowledged the command, indicating that the response can be read from it. Alternatively,
    /// `false` indicates that the command was discarded.
    fn send_command(&mut self, command: Command) -> Result<bool, StmDeviceError> {
        if self.initialised {
            self.port.write_all(&Vec::from(command))?;

            self.get_ack()
        } else {
            Err(StmDeviceError::Uninitialised)
        }
    }

    /// Attempts to read an `ACK` from the device, returning `true` if one is found, otherwise
    /// returning `false`. [StmDeviceError] is returned
    fn get_ack(&mut self) -> Result<bool, StmDeviceError> {
        match self.read_byte()? {
            ACK => Ok(true),
            NACK => Ok(false),
            b => Err(StmDeviceError::ExpectedAck(b)),
        }
    }

    fn read_byte(&mut self) -> Result<u8, StmDeviceError> {
        let mut buf = [0u8; 1];
        self.port.read_exact(&mut buf)?;
        Ok(buf[0])
    }

    fn read_bytes(&mut self, amount: usize) -> Result<Vec<u8>, StmDeviceError> {
        let mut buf = vec![0u8; amount];
        self.port.read_exact(&mut buf)?;
        Ok(buf)
    }
}
