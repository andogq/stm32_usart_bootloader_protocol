mod protocol;

use std::io;

use serialport::SerialPort;
use thiserror::Error;

use self::protocol::{Command, CommonCommand, Protocol, ProtocolError, ProtocolVersion, Response};

#[derive(Debug, Error)]
pub enum StmDeviceError {
    #[error("device has already been initialised")]
    AlreadyInitialised,
    #[error("device is not initialised")]
    Uninitialised,
    #[error("expected response, recieved {0:#x}")]
    InvalidResponse(u8),
    #[error("recieved NACK")]
    Nack,
    #[error("failed to run command: {0:?}")]
    CommandFail(Command),
    #[error("retries exceeded to run command: {0:?}")]
    RetryExceeded(Command),
    #[error(transparent)]
    ProtocolError(#[from] ProtocolError),
    #[error(transparent)]
    Io(#[from] io::Error),
}

pub enum ProductId {
    /// STM32 product IDs are 16 bits long
    Stm32(u16),
    /// It is possible for a different sized ID to be generated, which is stored in this variant
    Unknown(Vec<u8>),
}
impl From<&[u8]> for ProductId {
    fn from(bytes: &[u8]) -> Self {
        if bytes.len() == 2 {
            Self::Stm32(u16::from_be_bytes([bytes[0], bytes[1]]))
        } else {
            Self::Unknown(bytes.to_vec())
        }
    }
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
            protocol_version: ProtocolVersion::unknown(),
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
            self.get_ack()?;

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

        self.get_ack()?;

        // Attempt to determine protocol version
        Ok(Protocol::try_from(response.as_slice())?)
    }

    pub fn get_version(&mut self) -> Result<ProtocolVersion, StmDeviceError> {
        self.retry_command(Command::Common(CommonCommand::Get))?;

        let response = self.read_bytes(3)?;

        self.get_ack()?;

        self.protocol_version = ProtocolVersion::from(response[0]);
        Ok(self.protocol_version.clone())
    }

    pub fn get_id(&mut self) -> Result<ProductId, StmDeviceError> {
        self.retry_command(Command::Common(CommonCommand::GetId))?;

        let response_size = self.read_byte()? + 1;
        let product_id = ProductId::from(self.read_bytes(response_size as usize)?.as_slice());
        self.get_ack()?;

        Ok(product_id)
    }

    pub fn read_memory(&mut self, address: u32, byte_count: u8) -> Result<Vec<u8>, StmDeviceError> {
        self.retry_command(Command::Common(CommonCommand::ReadMemory))?;

        // Send the address
        self.send_bytes(&address.to_be_bytes())?;

        // Send the amount of bytes to read
        self.send_byte(byte_count)?;

        // Read bytes back from device
        self.read_bytes(byte_count as usize + 1)
    }

    fn retry_command(&mut self, command: Command) -> Result<(), StmDeviceError> {
        for _ in 0..self.retry_amount {
            match self.send_command(command) {
                Ok(Response::Ack) => return Ok(()),
                Ok(Response::Nack) => return Err(StmDeviceError::CommandFail(command)),
                Err(_) => (),
            }
        }

        Err(StmDeviceError::RetryExceeded(command))
    }

    /// Sends a command to the device, and waits for the ACK. Returns `true` if the device
    /// acknowledged the command, indicating that the response can be read from it. Alternatively,
    /// `false` indicates that the command was discarded.
    fn send_command(&mut self, command: Command) -> Result<Response, StmDeviceError> {
        if self.initialised {
            self.send_byte(command.into())
        } else {
            Err(StmDeviceError::Uninitialised)
        }
    }

    /// Attempts to read an `ACK` from the device
    fn get_ack(&mut self) -> Result<(), StmDeviceError> {
        self.get_response()?.ack()
    }

    /// Attempts to read a response from the device.
    fn get_response(&mut self) -> Result<Response, StmDeviceError> {
        Response::try_from(self.read_byte()?)
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

    /// Calculates the checksum for a single byte (the compliment of it), and sends it to the
    /// device, and finally waits for an ACK.
    fn send_byte(&mut self, byte: u8) -> Result<Response, StmDeviceError> {
        self.port.write_all(&[byte, !byte])?;
        self.get_response()
    }

    /// Calculates the checksum for the bytes, and sends it all to the device, waiting for an ACK.
    fn send_bytes(&mut self, bytes: &[u8]) -> Result<Response, StmDeviceError> {
        self.port
            .write_all(&[bytes, &[bytes.iter().fold(0, |checksum, &b| checksum ^ b)]].concat())?;
        self.get_response()
    }
}
