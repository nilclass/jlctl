use std::io::{Read, BufReader, BufRead, Write};
use std::time::Duration;
/*
    specialFunctionsString.replace("GND", "100");
    specialFunctionsString.replace("SUPPLY_5V", "105");
    specialFunctionsString.replace("SUPPLY_3V3", "103");
    specialFunctionsString.replace("DAC0_5V", "106");
    specialFunctionsString.replace("DAC1_8V", "107");
    specialFunctionsString.replace("I_N", "109");
    specialFunctionsString.replace("I_P", "108");
    specialFunctionsString.replace("EMPTY_NET", "127");
    specialFunctionsString.replace("ADC0_5V", "110");
    specialFunctionsString.replace("ADC1_5V", "111");
    specialFunctionsString.replace("ADC2_5V", "112");
    specialFunctionsString.replace("ADC3_8V", "113");
*/
enum Node {
    SUPPLY_5V,
    SUPPLY_3V3,
    GND,
    DAC_0,
    DAC_1,
    I_NEG,
    I_POS,
}

struct NodeId(&'static str);

struct Connection(NodeId, NodeId);

struct NodeFile(Vec<Connection>);

impl NodeFile {
    fn write_to<W: Write>(&self, w: &mut W) -> std::io::Result<()> {
        write!(w, "f{{\n")?;
        for connection in &self.0 {
            write!(w, "{}-{},\n", connection.0.0, connection.1.0)?;
        }
        write!(w, "}}\n")?;
        Ok(())
    }
}

fn main() {
    let mut serialport = serialport::new("/dev/ttyACM0", 57600).timeout(Duration::from_secs(5)).open().unwrap();
    let cmd = std::env::args().nth(1).unwrap();

    let node_file = NodeFile(vec![
        // Connection(NodeId("I_N"), NodeId("GND")),
        // Connection(NodeId("I_P"), NodeId("5")),
        // Connection(NodeId("SUPPLY_5V"), NodeId("1")),
        Connection(NodeId("DAC0_5V"), NodeId("5")),
    ]);

    match cmd.as_str() {
        "connect" => {
            node_file.write_to(&mut serialport).unwrap();
            serialport.flush().unwrap();
        }
        "netlist" => {
            serialport.write(b"n").unwrap();
            println!("{:#?}", parse_netlist(&mut serialport));
        }
        _ => panic!("Unknown cmd: {}", cmd),
    }
}

#[allow(dead_code)]
#[derive(Debug)]
struct NetlistEntry {
    index: u32,
    name: String,
    number: u32,
    nodes: String,
    bridges: String,
}

fn parse_netlist<R: Read>(r: &mut R) -> Vec<NetlistEntry> {
    let mut lines = BufReader::new(r).lines().map(|l| {
        l.unwrap().trim().to_owned()
    });
    lines.find(|l| {l == &"netlist"}).unwrap();
    lines.find(|l| l.starts_with("Index")).unwrap();

    let mut entries = vec![];
    
    for line in lines {
        if line.len() == 0 {
            break;
        }

        let (_, entry) = parse_netlist_line(&line).unwrap();
        entries.push(entry);
    }

    return entries
}

use nom::IResult;
use nom::combinator::map;
use nom::sequence::tuple;
use nom::bytes::complete::{take_while, take_till};

fn parse_tabs(input: &str) -> IResult<&str, &str> {
    take_while(|c| c == '\t')(input)
}

fn parse_string(input: &str) -> IResult<&str, String> {
    map(
        take_till(|c| c == '\t'),
        |s: &str| s.to_string(),
    )(input)
}

fn parse_netlist_line(input: &str) -> IResult<&str, NetlistEntry> {
    map(
        tuple((
            nom::character::complete::u32,
            parse_tabs,
            parse_string,
            parse_tabs,
            nom::character::complete::u32,
            parse_tabs,
            parse_string,
            parse_tabs,
            parse_string,
        )),
        |(index, _, name, _, number, _, nodes, _, bridges)| {
            NetlistEntry { index, name, number, nodes, bridges }
        }
    )(input)
}
