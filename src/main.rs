mod args;
mod stm_device;

use std::time::Duration;

use clap::Parser;
use serialport::{available_ports, SerialPortType, UsbPortInfo};

use args::*;
use stm_device::{StmDevice, StmDeviceError};
use thiserror::Error;

#[derive(Error, Debug)]
#[error(transparent)]
pub enum UartControlError {
    SerialPort(#[from] serialport::Error),
    #[error("Unable to convert path to string")]
    PathError,
    StmDevice(#[from] StmDeviceError),
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
                let mut device = StmDevice::new(
                    serialport::new(
                        device_path.to_str().ok_or(UartControlError::PathError)?,
                        baud,
                    )
                    .data_bits(data_bits.into())
                    .parity(parity.into())
                    .stop_bits(stop_bits.into())
                    .timeout(Duration::from_millis(200))
                    .open()?,
                );

                device.initialise()?;
                println!("Device initialised successfully");

                let protocol = device.get_protocol()?;
                println!(
                    "Device running protocol {}.{}",
                    protocol.full_version.0, protocol.full_version.1
                );
            }
        };
    }

    Ok(())
}
