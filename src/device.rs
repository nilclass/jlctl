use crate::logger::DeviceLogger;
use crate::parser;
use crate::types::{Bridgelist, ChipStatus, Color, Message, Net, SupplySwitchPos};
use anyhow::{Context, Result};
use serialport::SerialPort;
use std::io::{BufRead, BufReader, Write};
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::mpsc::{channel, Receiver, Sender};
use std::thread::{spawn, JoinHandle};
use std::time::Duration;

const PORT_TIMEOUT: Duration = Duration::from_millis(450);
const RESPONSE_TIMEOUT: Duration = Duration::from_millis(4000);

/// Represents a connection to a Jumperless device, on a fixed port.
pub struct Device<L: DeviceLogger> {
    port: Box<dyn SerialPort>,
    logger: L,
    reader: Option<(JoinHandle<()>, Receiver<Received>, Sender<()>)>,
    sequence: AtomicU32,
}

#[derive(Debug)]
enum Received {
    Message(Message),
    Unrecognized(String),
    Error(String),
}

fn parse_received(line: String) -> Received {
    match parser::message(&line) {
        Ok((_, message)) => Received::Message(message),
        Err(err) => {
            eprintln!("Error recognizing line: {:?}: {:?}", line, err);
            Received::Unrecognized(line)
        }
    }
}

/// Instructions are messages sent from the host to the Jumperless
enum Instruction {
    GetNetlist,
    SetNetlist(Vec<Net>),
    GetBridgelist,
    SetBridgelist(Bridgelist),
    GetSupplySwitch,
    SetSupplySwitch(SupplySwitchPos),
    Lightnet(String, Color),
    GetChipStatus,
    Raw(String, String),
}

impl Instruction {
    fn generate(&self, sequence_number: u32) -> String {
        match self {
            Instruction::Raw(instruction, args) => {
                format!("::{}:{}[{}]", instruction, sequence_number, args)
            }
            Instruction::GetNetlist => {
                format!("::getnetlist:{}[]", sequence_number)
            }
            Instruction::SetNetlist(nets) => {
                let nets: Vec<crate::types::TmpNet> =
                    nets.iter().map(|net| net.clone().into()).collect();
                format!(
                    "::netlist:{}{}",
                    sequence_number,
                    serde_json::to_string(&nets).expect("serialize nets")
                )
            }
            Instruction::GetBridgelist => {
                format!("::getbridgelist:{}[]", sequence_number)
            }
            Instruction::SetBridgelist(bridgelist) => {
                let mut line = format!("::bridgelist:{}[", sequence_number);
                for (i, (a, b)) in bridgelist.iter().enumerate() {
                    if i > 0 {
                        line += ",";
                    }
                    line += &a.to_string();
                    line += "-";
                    line += &b.to_string();
                }
                line += "]";
                line
            }
            Instruction::GetSupplySwitch => {
                format!("::getsupplyswitch:{}[]", sequence_number)
            }
            Instruction::SetSupplySwitch(pos) => {
                format!(
                    "::setsupplyswitch:{}[{}]",
                    sequence_number,
                    match pos {
                        SupplySwitchPos::V8 => "8V",
                        SupplySwitchPos::V3_3 => "3.3V",
                        SupplySwitchPos::V5 => "5V",
                    }
                )
            }
            Instruction::Lightnet(net_name, color) => {
                let color: u32 = (*color).into();
                format!(
                    "::lightnet:{}[{}: 0x{:06x}]",
                    sequence_number, net_name, color
                )
            }

            Instruction::GetChipStatus => {
                format!("::getchipstatus:{}[]", sequence_number)
            }
        }
    }
}

impl<L: DeviceLogger> Drop for Device<L> {
    fn drop(&mut self) {
        self.stop_reader_thread();
    }
}

impl<L: DeviceLogger> Device<L> {
    pub fn new(port_path: String, logger: L) -> Result<Self> {
        let port = serialport::new(port_path.as_str(), 57600)
            .timeout(PORT_TIMEOUT)
            .open()
            .with_context(|| format!("Failed to open serial port: {}", port_path))?;
        logger.open(port_path.as_str());
        let mut device = Self {
            port,
            logger,
            reader: None,
            sequence: AtomicU32::new(0),
        };

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
        self.send_instruction(Instruction::Raw(instruction, args))?;
        let success = loop {
            match self.receive() {
                Received::Message(Message::Ok(_)) => break true,
                Received::Message(Message::Error(_)) => break false,
                Received::Message(message) => messages.push(message),
                Received::Error(error) => {
                    return Err(anyhow::anyhow!("Received an error: {}", error))
                }
                Received::Unrecognized(chunk) => {
                    return Err(anyhow::anyhow!("Received unparsable: {:?}", chunk))
                }
            }
        };
        Ok((success, messages))
    }

    /// Retrieve current list of bridges
    pub fn bridgelist(&mut self) -> Result<Bridgelist> {
        let seq = self.send_instruction(Instruction::GetBridgelist)?;
        let bridgelist = loop {
            match self.receive() {
                Received::Message(Message::Bridgelist(bridgelist)) => break bridgelist,
                other => {
                    eprintln!("WARNING: received sth unexpected: {:?}", other);
                }
            }
        };
        self.receive_ok(seq)?;
        Ok(bridgelist)
    }

    /// Upload new list of bridges
    pub fn set_bridgelist(&mut self, bridgelist: Bridgelist) -> Result<()> {
        let seq = self.send_instruction(Instruction::SetBridgelist(bridgelist))?;
        self.receive_ok(seq)
    }

    pub fn receive_ok(&mut self, sequence_number: u32) -> Result<()> {
        self.receive_ok_capture(sequence_number, |_| {})
    }

    pub fn receive_ok_capture<F: FnMut(Message)>(
        &mut self,
        sequence_number: u32,
        mut capture: F,
    ) -> Result<()> {
        loop {
            match self.receive() {
                Received::Message(Message::Ok(Some(seq))) if seq == sequence_number => {
                    return Ok(())
                }
                Received::Message(Message::Error(Some(seq))) if seq == sequence_number => {
                    return Err(anyhow::anyhow!("Received error response"))
                }
                Received::Message(message) => capture(message),
                Received::Error(error) => return Err(anyhow::anyhow!("{:?}", error)),
                _ => {}
            }
        }
    }

    /// Retrieve list of nets
    pub fn netlist(&mut self) -> Result<Vec<Net>> {
        let seq = self.send_instruction(Instruction::GetNetlist)?;
        let mut result = vec![];
        let mut begin = false;
        self.receive_ok_capture(seq, |message| match message {
            Message::NetlistBegin => {
                begin = true;
            }
            Message::NetlistEnd => {
                begin = false;
            }
            Message::Net(net) => {
                result.push(net);
            }
            _ => {}
        })?;
        Ok(result)
    }

    /// Upload new list of nets
    pub fn set_netlist(&mut self, nets: Vec<Net>) -> Result<()> {
        let seq = self.send_instruction(Instruction::SetNetlist(nets))?;
        self.receive_ok(seq)
    }

    pub fn supply_switch(&mut self) -> Result<SupplySwitchPos> {
        let seq = self.send_instruction(Instruction::GetSupplySwitch)?;
        let mut result = None;
        self.receive_ok_capture(seq, |message| {
            if let Message::SupplySwitch(pos) = message {
                result = Some(pos);
            }
        })?;
        result.ok_or(anyhow::anyhow!("No ::supplyswitch message received!"))
    }

    pub fn set_supply_switch(&mut self, pos: SupplySwitchPos) -> Result<()> {
        let seq = self.send_instruction(Instruction::SetSupplySwitch(pos))?;
        self.receive_ok(seq)
    }

    pub fn chipstatus(&mut self) -> Result<Vec<ChipStatus>> {
        let seq = self.send_instruction(Instruction::GetChipStatus)?;
        let mut result = vec![];
        let mut begin = false;
        self.receive_ok_capture(seq, |message| match message {
            Message::ChipStatusBegin => {
                begin = true;
            }
            Message::ChipStatus(chip_status) => {
                if begin {
                    result.push(chip_status);
                }
            }
            Message::ChipStatusEnd => {
                begin = false;
            }
            _ => {}
        })?;
        Ok(result)
    }

    pub fn lightnet(&mut self, name: String, color: Color) -> Result<()> {
        self.send_instruction(Instruction::Lightnet(name, color))?;
        Ok(())
    }

    fn send_instruction(&mut self, instruction: Instruction) -> Result<u32> {
        let sequence_number = self.sequence.fetch_add(1, Ordering::SeqCst) + 1;
        let msg = instruction.generate(sequence_number);
        self.logger.sent(&msg);
        write!(self.port, "{}\r\n", msg)?;
        Ok(sequence_number)
    }

    fn receive(&mut self) -> Received {
        let (_, recv, _) = self.reader.as_mut().expect("Reader thread");
        match recv.recv_timeout(RESPONSE_TIMEOUT) {
            Ok(received) => received,
            _ => Received::Error("Timeout while receiving reply".to_string()),
        }
    }

    pub fn clear_nodefile(&mut self) -> Result<()> {
        self.port.write_all(b"f{\n}\n")?;
        Ok(())
    }

    fn start_reader_thread(&mut self) -> Result<()> {
        let port = self.port.try_clone()?;
        let logger = self.logger.clone();
        let (send, recv) = channel();
        let (send_stop, recv_stop) = channel();
        self.reader = Some((
            spawn(move || Device::reader_thread(port, logger, send, recv_stop)),
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

    fn reader_thread(
        port: Box<dyn SerialPort>,
        logger: L,
        sender: Sender<Received>,
        stop: Receiver<()>,
    ) {
        let mut lines = BufReader::new(port).lines();
        loop {
            if let Ok(()) = stop.try_recv() {
                return;
            }
            match lines.next() {
                None => return,
                Some(Ok(line)) => {
                    let line = line.trim_matches('\r').to_owned();
                    logger.received(&line);
                    if line.starts_with("::") {
                        sender.send(parse_received(line)).unwrap();
                    }
                }
                Some(Err(err)) => {
                    if let std::io::ErrorKind::TimedOut = err.kind() {
                        // ignore timeout. It happens whenever the device does not send anything for a given amount of time.
                    } else {
                        eprintln!("ERROR: {:?}", err);
                        sender
                            .send(Received::Error(format!(
                                "Read from serial port failed: {:?}",
                                err
                            )))
                            .unwrap();

                        // terminate thread
                        return;
                    }
                }
            }
        }
    }
}
