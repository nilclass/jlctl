use time::OffsetDateTime;
use time::format_description::well_known::Iso8601;
use anyhow::{Context, Result};
use serialport::SerialPort;
use std::sync::mpsc::{channel, Sender, Receiver};
use std::time::Duration;
use std::sync::{Arc, Mutex};
use std::io::{Write, BufRead, BufReader};
use std::fs::File;
use std::thread::{spawn, JoinHandle};
use crate::new_parser;
use crate::types::{Message, Net, Bridgelist, Color};

/// Represents a connection to a Jumperless device, on a fixed port.
pub struct Device {
    port: Box<dyn SerialPort>,
    log: Arc<Mutex<File>>,
    reader: Option<(JoinHandle<()>, Receiver<Received>, Sender<()>)>
}

#[derive(Debug)]
enum Received {
    Message(Message),
    Unrecognized(String),
    Error(String),
}

fn parse_received(line: String) -> Received {
    match new_parser::message(&line) {
        Ok((_, message)) => Received::Message(message),
        Err(err) => {
            eprintln!("Error recognizing line: {:?}: {:?}", line, err);
            Received::Unrecognized(line)
        }
    }
}

/// Commands are messages sent from the host to the Jumperless
enum Command {
    GetNetlist,
    SetNetlist(Vec<Net>),
    GetBridgelist,
    SetBridgelist(Bridgelist),
    SetSupplySwitch(SupplySwitchPos),
    Lightnet(String, Color),
    Raw(String, String),
}

/// Represents the position of the supply switch.
///
/// NOTE: the Jumperless cannot detect the actual state of the switch.
///   Instead the user must correctly advertise the state to the board,
///   for the power rows to be lit up correctly.
#[derive(Clone, Debug)]
pub enum SupplySwitchPos {
    V8,
    V3_3,
    V5,
}

impl std::str::FromStr for SupplySwitchPos {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        match s {
            "8V" => Ok(SupplySwitchPos::V8),
            "3.3V" => Ok(SupplySwitchPos::V3_3),
            "5V" => Ok(SupplySwitchPos::V5),
            _ => Err(anyhow::anyhow!("Unknown variant")),
        }
    }
}

impl Drop for Device {
    fn drop(&mut self) {
        self.stop_reader_thread();
    }
}

impl Device {
    pub fn new(port_path: String, log_path: String) -> Result<Self> {
        let port = serialport::new(port_path.as_str(), 57600)
            .timeout(Duration::from_millis(100))
            .open()
            .with_context(|| format!("Failed to open serial port: {}", port_path))?;
        let log = File::options()
            .create(true)
            .append(true)
            .open(log_path.as_str())
            .map(|f| Arc::new(Mutex::new(f)))
            .with_context(|| format!("Failed to open log file: {}", log_path))?;
        let mut device = Self { port, log, reader: None };

        device.start_reader_thread()?;

        Ok(device)
    }

    /// Check if the connection is alive.
    ///
    /// Returns false if the reader thread encountered an error.
    pub fn is_alive(&self) -> bool {
        let (thread, _, _) = self.reader.as_ref().unwrap();
        !thread.is_finished()
    }

    pub fn raw(&mut self, instruction: String, args: String) -> Result<(bool, Vec<Message>)> {
        let mut messages = vec![];
        self.send_command(Command::Raw(instruction, args))?;
        let success = loop {
            match self.receive() {
                Received::Message(Message::Ok) => break true,
                Received::Message(Message::Error) => break false,
                Received::Message(message) => messages.push(message),
                Received::Error(error) => return Err(anyhow::anyhow!("Received an error: {}", error)),
                Received::Unrecognized(chunk) => return Err(anyhow::anyhow!("Received unparsable: {:?}", chunk)),
            }
        };
        Ok((success, messages))
    }

    /// Retrieve current list of bridges
    pub fn bridgelist(&mut self) -> Result<Bridgelist> {
        self.send_command(Command::GetBridgelist)?;
        loop {
            match self.receive() {
                Received::Message(Message::Bridgelist(bridgelist)) => return Ok(bridgelist),
                other => {
                    eprintln!("WARNING: received sth unexpected: {:?}", other);
                }
            }
        }
    }

    /// Upload new list of bridges
    pub fn set_bridgelist(&mut self, bridgelist: Bridgelist) -> Result<()> {
        self.send_command(Command::SetBridgelist(bridgelist))?;
        Ok(())
    }

    /// Retrieve list of nets
    pub fn netlist(&mut self) -> Result<Vec<Net>> {
        self.send_command(Command::GetNetlist)?;
        let mut result = vec![];
        loop {
            match self.receive() {
                Received::Message(Message::NetlistBegin) => {
                    break;
                }
                Received::Error(error) => {
                    return Err(anyhow::anyhow!("while reading netlist: {}", error));
                }
                other => {
                    eprintln!("WARNING: received sth unexpected waiting for begin: {:?}", other);
                }
            }
        }
        loop {
            match self.receive() {
                Received::Message(Message::NetlistEnd) => {
                    break;
                }
                Received::Message(Message::Net(net)) => {
                    result.push(net);
                }
                Received::Error(error) => {
                    return Err(anyhow::anyhow!("while reading netlist: {}", error));
                }
                other => {
                    eprintln!("WARNING: received sth unexpected: {:?}", other);
                }
            }
        }
        Ok(result)
    }

    /// Upload new list of nets
    pub fn set_netlist(&mut self, nets: Vec<Net>) -> Result<()> {
        self.send_command(Command::SetNetlist(nets))?;
        Ok(())
    }

    // pub fn receive_message(&mut self) -> Option<Message> {
    //     match self.receive() {
    //         Received::Message(message) => Some(message),
    //         _ => None,
    //     }
    // }

    pub fn set_supply_switch(&mut self, pos: SupplySwitchPos) -> Result<()> {
        self.send_command(Command::SetSupplySwitch(pos))?;
        Ok(())
    }

    pub fn lightnet(&mut self, name: String, color: Color) -> Result<()> {
        self.send_command(Command::Lightnet(name, color))?;
        Ok(())
    }

    fn send_command(&mut self, command: Command) -> Result<()> {
        let msg = match command {
            Command::Raw(instruction, args) => {
                format!("::{}[{}]", instruction, args)
            }
            Command::GetNetlist => {
                "::getnetlist[]".to_string()
            }
            Command::SetNetlist(nets) => {
                format!("::netlist{}", serde_json::to_string(&nets)?)
            }
            Command::GetBridgelist => {
                "::getbridgelist[]".to_string()
            }
            Command::SetBridgelist(bridgelist) => {
                let mut line = "::bridgelist[".to_string();
                for (i, (a, b)) in bridgelist.into_iter().enumerate() {
                    if i > 0 {
                        line += ",";
                    }
                    line += &a.to_string();
                    line += "-";
                    line += &b.to_string();
                }
                line
            }
            Command::SetSupplySwitch(pos) => {
                format!("::setsupplyswitch[{}]", match pos {
                    SupplySwitchPos::V8 => "8V",
                    SupplySwitchPos::V3_3 => "3.3V",
                    SupplySwitchPos::V5 => "5V",
                })
            }
            Command::Lightnet(net_name, color) => {
                let color: u32 = color.into();
                format!("::lightnet[{}: 0x{:06x}]", net_name, color)
            }
        };
        write_log(&self.log, &format!("SEND {}", msg))?;
        write!(self.port, "{}\r\n", msg)?;
        Ok(())
    }

    fn receive(&mut self) -> Received {
        let (_, recv, _) = self.reader.as_mut().expect("Reader thread");
        recv.recv_timeout(std::time::Duration::from_millis(200)).expect("receive")
    }

    // pub fn send_nodefile(&mut self, nodefile: &NodeFile) -> Result<()> {
    //     self.port.write_all(b"f{\n")?;
    //     self.port.write_all(nodefile.to_string().as_bytes())?;
    //     self.port.write_all(b",\n}\n")?;
    //     Ok(())
    // }

    pub fn clear_nodefile(&mut self) -> Result<()> {
        self.port.write_all(b"f{\n}\n")?;
        Ok(())
    }

    fn start_reader_thread(&mut self) -> Result<()> {
        let port = self.port.try_clone()?;
        let log = self.log.clone();
        let (send, recv) = channel();
        let (send_stop, recv_stop) = channel();
        self.reader = Some((
            spawn(move || Device::reader_thread(port, log, send, recv_stop)),
            recv,
            send_stop,
        ));
        Ok(())
    }

    fn stop_reader_thread(&mut self) {
        if let Some((thread, _, send_stop)) = self.reader.take() {
            _ = send_stop.send(());
            _ = thread.join();
        }
    }

    fn reader_thread(port: Box<dyn SerialPort>, log: Arc<Mutex<File>>, sender: Sender<Received>, stop: Receiver<()>) {
        let mut lines = BufReader::new(port).lines();
        _ = write_log(&log, "OPEN");
        loop {
            if let Ok(()) = stop.try_recv() {
                return;
            }
            match lines.next() {
                None => return,
                Some(Ok(line)) => {
                    let line = line.trim_matches('\r').to_owned();
                    _ = write_log(&log, &format!("REVC {}", line));
                    if line.starts_with("::") {
                        sender.send(parse_received(line)).unwrap();
                    }
                }
                Some(Err(err)) => {
                    if let std::io::ErrorKind::TimedOut = err.kind() {
                        // ignore timeout. It happens whenever the device does not send anything for a given amount of time.
                    } else {
                        eprintln!("ERROR: {:?}", err);
                        sender.send(Received::Error(format!("Read from serial port failed: {:?}", err))).unwrap();

                        // terminate thread
                        return;
                    }
                }
            }
        }
    }
}

fn write_log(log: &Arc<Mutex<File>>, line: &str) -> anyhow::Result<()> {
    let mut log = log.lock().expect("log mutex");
    write!(log, "[{}] {}\n", OffsetDateTime::now_utc().format(&Iso8601::DEFAULT).unwrap(), line)?;
    Ok(())
}
