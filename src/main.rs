mod args;

use clap::Parser;
use serialport::{available_ports, SerialPortType, UsbPortInfo};

use args::*;

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
