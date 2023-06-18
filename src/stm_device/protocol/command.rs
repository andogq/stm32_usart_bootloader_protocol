use num_enum::TryFromPrimitive;

#[derive(Debug, Clone, Copy, TryFromPrimitive)]
#[repr(u8)]
pub enum CommonCommand {
    Get = 0x00,
    GetVersion = 0x01,
    GetId = 0x02,
    ReadMemory = 0x11,
    Go = 0x21,
    WriteMemory = 0x31,
    ExtendedErase = 0x44,
    WriteProtect = 0x63,
}

#[derive(Debug, Clone, Copy, TryFromPrimitive)]
#[repr(u8)]
pub enum PreV4Command {
    Erase = 0x43,
    WriteUnprotect = 0x73,
    ReadoutProtect = 0x82,
    ReadoutUnprotect = 0x92,
}

#[derive(Debug, Clone, Copy, TryFromPrimitive)]
#[repr(u8)]
pub enum PostV4Command {
    Special = 0x50,
    Write = 0x73,
}

#[derive(Debug, Clone, Copy)]
pub enum Command {
    Common(CommonCommand),
    PreV4(PreV4Command),
    PostV4(PostV4Command),
}
impl From<Command> for Vec<u8> {
    fn from(command: Command) -> Self {
        let b = match command {
            Command::Common(command) => command as u8,
            Command::PreV4(command) => command as u8,
            Command::PostV4(command) => command as u8,
        };
        vec![b, !b]
    }
}
