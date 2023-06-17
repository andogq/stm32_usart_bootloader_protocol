use std::{io, path::PathBuf};

use clap::{builder::PossibleValue, Parser, Subcommand, ValueEnum};
use serialport::{available_ports, SerialPortType, UsbPortInfo};
use thiserror::Error;

#[derive(Error, Debug)]
#[error(transparent)]
enum UartControlError {
    SerialPort(#[from] serialport::Error),
    #[error("Unable to convert path to string")]
    PathError,
    IoError(#[from] io::Error),
}

#[derive(Clone)]
enum DataBits {
    Five,
    Six,
    Seven,
    Eight,
}
impl ValueEnum for DataBits {
    fn value_variants<'a>() -> &'a [Self] {
        &[Self::Five, Self::Six, Self::Seven, Self::Eight]
    }

    fn to_possible_value(&self) -> Option<PossibleValue> {
        Some(PossibleValue::new(match self {
            DataBits::Five => "5",
            DataBits::Six => "6",
            DataBits::Seven => "7",
            DataBits::Eight => "8",
        }))
    }
}
impl From<DataBits> for serialport::DataBits {
    fn from(data_bits: DataBits) -> Self {
        match data_bits {
            DataBits::Five => Self::Five,
            DataBits::Six => Self::Six,
            DataBits::Seven => Self::Seven,
            DataBits::Eight => Self::Eight,
        }
    }
}

#[derive(Clone, ValueEnum)]
enum Parity {
    Even,
    Odd,
    None,
}
impl From<Parity> for serialport::Parity {
    fn from(parity: Parity) -> Self {
        match parity {
            Parity::Even => Self::Even,
            Parity::Odd => Self::Odd,
            Parity::None => Self::None,
        }
    }
}

#[derive(Clone)]
enum StopBits {
    One,
    Two,
}
impl ValueEnum for StopBits {
    fn value_variants<'a>() -> &'a [Self] {
        &[Self::One, Self::Two]
    }

    fn to_possible_value(&self) -> Option<PossibleValue> {
        Some(PossibleValue::new(match self {
            Self::One => "1",
            Self::Two => "2",
        }))
    }
}
impl From<StopBits> for serialport::StopBits {
    fn from(stop_bits: StopBits) -> Self {
        match stop_bits {
            StopBits::One => Self::One,
            StopBits::Two => Self::Two,
        }
    }
}

#[derive(Parser)]
struct Args {
    #[command(subcommand)]
    command: Option<Command>,
}

#[derive(Subcommand)]
enum Command {
    /// Lists available serial devices
    ListDevices,
    /// Connects to a device
    Connect {
        /// Path to the serial device to connect to
        device_path: PathBuf,

        /// Baud rate to operate at
        #[arg(short, long, default_value_t = 9600)]
        baud: u32,

        /// Number of data bits to use
        #[arg(short, long, value_enum, default_value_t = DataBits::Eight)]
        data_bits: DataBits,

        /// Parity of data bits
        #[arg(short, long, value_enum, default_value_t = Parity::Even)]
        parity: Parity,

        /// Stop bits after data bits
        #[arg(short, long, value_enum, default_value_t = StopBits::One)]
        stop_bits: StopBits,
    },
}

fn main() -> Result<(), UartControlError> {
    let args = Args::parse();

    if let Some(command) = args.command {
        match command {
            Command::ListDevices => {
                let ports = available_ports()?;

                println!("Available serial devices:");
                for port in ports {
                    let device_type = match port.port_type {
                        SerialPortType::UsbPort(_) => "USB Port",
                        SerialPortType::PciPort => "PCI Port",
                        SerialPortType::BluetoothPort => "Bluetooth Port",
                        SerialPortType::Unknown => "Unknown Port",
                    };

                    println!("    {}: {}", port.port_name, device_type);

                    if let SerialPortType::UsbPort(UsbPortInfo {
                        manufacturer,
                        product,
                        serial_number,
                        ..
                    }) = port.port_type
                    {
                        println!(
                            "        {}",
                            [
                                Some(
                                    [product, manufacturer]
                                        .into_iter()
                                        .flatten()
                                        .collect::<Vec<_>>()
                                        .join(", "),
                                ),
                                serial_number.map(|serial| format!("({serial})")),
                            ]
                            .into_iter()
                            .flatten()
                            .collect::<Vec<_>>()
                            .join(" ")
                        )
                    };
                }
            }
            Command::Connect {
                device_path,
                baud,
                data_bits,
                parity,
                stop_bits,
            } => {
                let mut port = serialport::new(
                    device_path.to_str().ok_or(UartControlError::PathError)?,
                    baud,
                )
                .data_bits(data_bits.into())
                .parity(parity.into())
                .stop_bits(stop_bits.into())
                .open()?;

                // Wake up chip
                port.write_all(&[0x7F])?;

                // Wait for response
                let mut response_buf = [0u8; 1];
                port.read_exact(&mut response_buf)?;
                if response_buf[0] == 0x79 {
                    println!("Device awoken successfully");
                } else {
                    println!("Unknown byte recieved from device: {:#x?}", response_buf[0]);
                }
            }
        };
    }

    Ok(())
}
