use crate::device::Device;
use anyhow::{Context, Result};
use log::{debug, error};
use serialport::{SerialPortInfo, SerialPortType, UsbPortInfo};
use std::collections::HashMap;
use serde::Serialize;

/// Identifies and manages the jumperless [`Device`], to communicate with.
pub struct DeviceManager {
    path: Option<String>,
    device: Option<Device>,
}

#[derive(Serialize)]
pub struct Status {
    connected: bool,
}

impl DeviceManager {
    /// Create a DeviceManager
    ///
    /// If `path` is given, it is the only serial port that will be used. The manager
    /// will not try to identify ports, and always use this single port.
    ///
    /// Otherwise [`DeviceManager::list_ports`] is called and the first port with role
    /// [`PortRole::JumperlessPrimary`] is used.
    pub fn new(path: Option<String>) -> Self {
        if path.is_some() {
            log::info!(
                "Initialize DeviceManager, with fixed port {}",
                path.as_ref().unwrap()
            );
        } else {
            log::info!("Initialize DeviceManager, with dynamic port detection");
        }
        Self { path, device: None }
    }

    pub fn status(&mut self) -> Result<Status> {
        let connected = self.with_device(|_| { Ok(()) }).is_ok();
        Ok(Status { connected })
    }

    /// Attempts to open the device (if it is not already open) and passes it to the closure.
    /// If an error occurs, the device is forgotten, so the next call will try to open the
    /// device again.
    pub fn with_device<T, F: FnOnce(&mut Device) -> Result<T>>(&mut self, f: F) -> Result<T> {
        f(self.device()?).map_err(|e| self.forget_device(e))
    }

    pub fn close_device(&mut self) {
        self.device = None;
    }

    fn forget_device(&mut self, error: anyhow::Error) -> anyhow::Error {
        log::error!("Error communicating with device: {}", error);
        self.device = None;
        error
    }

    fn device(&mut self) -> Result<&mut Device> {
        if self.device.is_some() && self.device.as_ref().unwrap().is_alive() {
            Ok(self.device.as_mut().unwrap())
        } else {
            log::info!("Attempting to open device");
            self.open()
        }
    }

    fn open(&mut self) -> Result<&mut Device> {
        let port_path = self.port_path()?;
        let device = Device::new(port_path.clone(), "log.txt".to_string())?;
        self.device = Some(device);
        log::info!("Connected to jumperless on port {}", port_path);
        Ok(self.device.as_mut().unwrap())
    }

    fn port_path(&self) -> Result<String> {
        if self.path.is_some() {
            return Ok(self.path.as_ref().unwrap().to_owned());
        }

        let primary = self
            .list_ports()?
            .into_iter()
            .find(|port| port.role == PortRole::JumperlessPrimary)
            .ok_or(anyhow::anyhow!("No matching serial port found"))?;

        debug!("Found primary: {:?}", primary.info);

        Ok(primary.info.port_name.clone())
    }

    /// List all (USB) serial ports, and attempt to identify the Jumperless
    pub fn list_ports(&self) -> Result<Vec<FoundPort>> {
        let port_infos =
            serialport::available_ports().with_context(|| "Failed to list available ports")?;
        let mut by_usb_id: HashMap<(u16, u16), Vec<SerialPortInfo>> = HashMap::new();

        for info in &port_infos {
            debug!("Checking port {:?}", info);
            match &info.port_type {
                serialport::SerialPortType::UsbPort(usb) => {
                    let id = (usb.vid, usb.pid);
                    debug!("  USB: {:?}", id);
                    by_usb_id.entry(id).or_default().push(info.clone());
                }
                unhandled => {
                    log::warn!(
                        "Ignoring port {:?}. Unhandled port type: {:?}",
                        info.port_name,
                        unhandled
                    )
                }
            }
        }

        let mut found = vec![];

        for (id, infos) in &mut by_usb_id {
            let SerialPortType::UsbPort(UsbPortInfo { product, .. }) = &infos[0].port_type else {
                unreachable!()
            };

            if product.is_some() && product.as_ref().unwrap() == "Jumperless" {
                // remove "tty" ports on Mac OS (only use the "cu" ones)
                fixup_mac_ports(infos);

                match infos.len() {
                    1 => {
                        debug!(
                            "Matching USB device {:4x}:{:4x} with single port",
                            id.0, id.1
                        );
                        found.push(FoundPort {
                            info: infos[0].clone(),
                            role: PortRole::JumperlessPrimary,
                        });
                    }
                    2 => {
                        let (a, b) = (&infos[0].port_name, &infos[1].port_name);
                        let (primary, arduino) = if a > b { (1, 0) } else { (0, 1) };
                        debug!("Matching USB device {:4x}:{:4x} with two ports: primary={}, arduino={}", id.0, id.1,
                               infos[primary].port_name, infos[arduino].port_name);
                        found.push(FoundPort {
                            info: infos[primary].clone(),
                            role: PortRole::JumperlessPrimary,
                        });
                        found.push(FoundPort {
                            info: infos[arduino].clone(),
                            role: PortRole::JumperlessArduino,
                        });
                    }
                    _ => {
                        error!(
                            "Matching device {:4x}:{:4x} with more than two ports: {:#?}",
                            id.0, id.1, infos
                        );
                    }
                }
            } else {
                for info in infos {
                    found.push(FoundPort {
                        info: info.clone(),
                        role: PortRole::Unknown,
                    });
                }
            }
        }

        Ok(found)
    }
}

/// A serial port that was found by [`DeviceManager::list_ports`]
pub struct FoundPort {
    /// The original port info
    pub info: SerialPortInfo,
    /// Identified role
    pub role: PortRole,
}

impl FoundPort {
    /// USB vendor and product ID
    pub fn usb_id(&self) -> (u16, u16) {
        let SerialPortType::UsbPort(UsbPortInfo { vid, pid, .. }) = self.info.port_type else {
            unreachable!()
        };
        (vid, pid)
    }
}

/// A role, used in [FoundPort]
#[derive(Debug, PartialEq)]
pub enum PortRole {
    /// No idea what this device is
    Unknown,
    /// The device identifies as a Jumperless, and this port is either the only one, or the one
    /// with the lower port number.
    JumperlessPrimary,
    /// The device identifies as a Jumperless, and this port is the higher one of the two.
    JumperlessArduino,
}

fn fixup_mac_ports(infos: &mut Vec<SerialPortInfo>) {
    // On MacOS for every real serial port there are two device
    // nodes: one starting with "cu", one with "tty".
    //
    // If both exist, we filter out the "tty" ones and only use the "cu"s.

    let (mut cu, mut tty) = (false, false);

    for info in infos.iter() {
        if info.port_name.starts_with("/dev/cu.") {
            cu = true;
        }
        if info.port_name.starts_with("/dev/tty.") {
            tty = true;
        }
    }

    if cu && tty {
        infos.retain(|info| info.port_name.starts_with("/dev/cu."))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fixup_mac_ports() {
        let mut mac_ports = vec![
            SerialPortInfo {
                port_name: "/dev/cu.usbmodem01".to_string(),
                port_type: SerialPortType::UsbPort(UsbPortInfo {
                    vid: 44203,
                    pid: 4882,
                    serial_number: Some("0".to_string()),
                    manufacturer: Some("Architeuthis Flux".to_string()),
                    product: Some("Jumperless".to_string()),
                }),
            },
            SerialPortInfo {
                port_name: "/dev/tty.usbmodem01".to_string(),
                port_type: SerialPortType::UsbPort(UsbPortInfo {
                    vid: 44203,
                    pid: 4882,
                    serial_number: Some("0".to_string()),
                    manufacturer: Some("Architeuthis Flux".to_string()),
                    product: Some("Jumperless".to_string()),
                }),
            },
            SerialPortInfo {
                port_name: "/dev/cu.usbmodem03".to_string(),
                port_type: SerialPortType::UsbPort(UsbPortInfo {
                    vid: 44203,
                    pid: 4882,
                    serial_number: Some("0".to_string()),
                    manufacturer: Some("Architeuthis Flux".to_string()),
                    product: Some("Jumperless".to_string()),
                }),
            },
            SerialPortInfo {
                port_name: "/dev/tty.usbmodem03".to_string(),
                port_type: SerialPortType::UsbPort(UsbPortInfo {
                    vid: 44203,
                    pid: 4882,
                    serial_number: Some("0".to_string()),
                    manufacturer: Some("Architeuthis Flux".to_string()),
                    product: Some("Jumperless".to_string()),
                }),
            },
        ];

        let expected = vec![mac_ports[0].clone(), mac_ports[2].clone()];

        fixup_mac_ports(&mut mac_ports);

        assert_eq!(mac_ports, expected);
    }
}
