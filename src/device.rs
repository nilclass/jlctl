use crate::netlist::{NetlistEntry, NodeFile};
use crate::parser::parse_netlist;
use anyhow::{Context, Result};
use serialport::SerialPort;
use std::time::Duration;

/// Represents a connection to a Jumperless device, on a fixed port.
pub struct Device {
    port: Box<dyn SerialPort>,
}

impl Device {
    pub fn new(path: String) -> Result<Self> {
        let port = serialport::new(path.as_str(), 57600)
            .timeout(Duration::from_secs(5))
            .open()
            .with_context(|| format!("Failed to open serial port: {}", path))?;
        Ok(Self { port })
    }

    pub fn netlist(&mut self) -> Result<Vec<NetlistEntry>> {
        self.port.write_all(b"n")?;
        Ok(parse_netlist(&mut self.port))
    }

    pub fn send_nodefile(&mut self, nodefile: &NodeFile) -> Result<()> {
        self.port.write_all(b"f{\n")?;
        self.port.write_all(nodefile.to_string().as_bytes())?;
        self.port.write_all(b",\n}\n")?;
        Ok(())
    }

    pub fn clear_nodefile(&mut self) -> Result<()> {
        self.port.write_all(b"f{\n}\n")?;
        Ok(())
    }
}
