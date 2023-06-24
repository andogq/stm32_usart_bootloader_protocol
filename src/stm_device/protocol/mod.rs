mod command;

use thiserror::Error;

pub use command::{Command, CommonCommand, PostV4Command, PreV4Command};

use super::StmDeviceError;

#[derive(Debug, Clone)]
pub struct ProtocolVersion {
    pub version: Option<(u8, u8)>,
}
impl ProtocolVersion {
    pub fn unknown() -> Self {
        Self { version: None }
    }

    pub fn is_known(&self) -> bool {
        self.version.is_some()
    }

    pub fn is_post_v4(&self) -> bool {
        self.version
            .map(|version| version.0 >= 4)
            .unwrap_or_default()
    }

    pub fn parse_command(&self, command: u8) -> Option<Command> {
        let common = CommonCommand::try_from(command).map(Command::Common).ok();

        if common.is_some() {
            common
        } else if self.is_known() {
            if self.is_post_v4() {
                PostV4Command::try_from(command).map(Command::PostV4).ok()
            } else {
                PreV4Command::try_from(command).map(Command::PreV4).ok()
            }
        } else {
            None
        }
    }
}
impl From<u8> for ProtocolVersion {
    fn from(value: u8) -> Self {
        Self {
            version: Some(((value & 0xf0) >> 4, value & 0x0f)),
        }
    }
}

#[derive(Debug, Error)]
pub enum ProtocolError {
    #[error("missing byte for protocol version")]
    MissingProtocol,
    #[error("received invalid command from device: {0:#x}")]
    InvalidCommand(u8),
    #[error("unknown commands for protocol version {version:?}: {unknown_commands:?}")]
    UnknownCommands {
        version: ProtocolVersion,
        unknown_commands: Vec<u8>,
    },
}

#[derive(Debug, Clone)]
pub struct Protocol {
    pub version: ProtocolVersion,
}

impl TryFrom<&[u8]> for Protocol {
    type Error = ProtocolError;

    fn try_from(bytes: &[u8]) -> Result<Self, Self::Error> {
        let mut bytes = bytes.iter();

        let version = bytes
            .next()
            .ok_or(ProtocolError::MissingProtocol)
            .map(|&version| ProtocolVersion::from(version))?;

        // Make sure that the commands are valid
        let unknown_commands = bytes
            .filter_map(|&b| {
                if version.parse_command(b).is_some() {
                    None
                } else {
                    Some(b)
                }
            })
            .collect::<Vec<_>>();

        if unknown_commands.is_empty() {
            Ok(Self { version })
        } else {
            Err(ProtocolError::UnknownCommands {
                version,
                unknown_commands,
            })
        }
    }
}

pub enum Response {
    Ack,
    Nack,
}
impl Response {
    pub fn ack(self) -> Result<(), StmDeviceError> {
        match self {
            Self::Ack => Ok(()),
            Self::Nack => Err(StmDeviceError::Nack),
        }
    }
}
impl TryFrom<u8> for Response {
    type Error = StmDeviceError;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0x79 => Ok(Self::Ack),
            0x1F => Ok(Self::Nack),
            _ => Err(Self::Error::InvalidResponse(value)),
        }
    }
}
