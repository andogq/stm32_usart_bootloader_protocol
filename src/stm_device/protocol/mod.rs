mod command;

use thiserror::Error;

pub use command::{Command, CommonCommand, PostV4Command, PreV4Command};

#[derive(Debug)]
pub enum ProtocolVersion {
    Unknown,
    PreV4,
    PostV4,
}
impl ProtocolVersion {
    pub fn parse_command(&self, command: u8) -> Option<Command> {
        let common = CommonCommand::try_from(command).map(Command::Common).ok();
        let version = match self {
            Self::Unknown => None,
            Self::PreV4 => PreV4Command::try_from(command).map(Command::PreV4).ok(),
            Self::PostV4 => PostV4Command::try_from(command).map(Command::PostV4).ok(),
        };

        match (common, version) {
            (Some(command), _) => Some(command),
            (_, Some(version)) => Some(version),
            _ => None,
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

pub struct Protocol {
    pub version: ProtocolVersion,
    pub full_version: (u8, u8),
}

impl TryFrom<&[u8]> for Protocol {
    type Error = ProtocolError;

    fn try_from(bytes: &[u8]) -> Result<Self, Self::Error> {
        let mut bytes = bytes.iter();

        let protocol = bytes.next().ok_or(ProtocolError::MissingProtocol)?;
        let full_version = ((protocol & 0xf0) >> 4, protocol & 0x0f);
        let version = if full_version.0 < 4 {
            ProtocolVersion::PreV4
        } else {
            ProtocolVersion::PostV4
        };

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
            Ok(Self {
                version,
                full_version,
            })
        } else {
            Err(ProtocolError::UnknownCommands {
                version,
                unknown_commands,
            })
        }
    }
}
