use std::collections::HashSet;

use serde::Serialize;

#[allow(dead_code)]
#[derive(Debug, Serialize)]
pub struct NetlistEntry {
    pub index: u32,
    pub name: String,
    pub number: u32,
    pub nodes: String,
    pub bridges: String,
}

#[allow(non_camel_case_types)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Node {
    GND,
    SUPPLY_5V,
    SUPPLY_3V3,
    DAC_0_5V,
    DAC_1_8V,
    I_N,
    I_P,
    ADC0_5V,
    ADC1_5V,
    ADC2_5V,
    ADC3_8V,
    D0,
    D1,
    D2,
    D3,
    D4,
    D5,
    D6,
    D7,
    D8,
    D9,
    D10,
    D11,
    D12,
    D13,
    A0,
    A1,
    A2,
    A3,
    A4,
    A5,
    A6,
    A7,
    RESET,
    AREF,
    Column(u8),
}

impl Node {
    fn col(n: u8) -> Option<Self> {
        if n >= 1 && n <= 60 {
            Some(Node::Column(n))
        } else {
            None
        }
    }

    fn parse(s: &str) -> anyhow::Result<Self> {
        if let Ok(n) = s.parse::<u8>() {
            Node::col(n).ok_or(anyhow::anyhow!("Invalid numerical node"))
        } else {
            use Node::*;
            match s {
                "GND" => Ok(GND),
                "SUPPLY_5V" => Ok(SUPPLY_5V),                                        
                "SUPPLY_3V3" => Ok(SUPPLY_3V3),
                "DAC_0_5V" => Ok(DAC_0_5V),
                "DAC_1_8V" => Ok(DAC_1_8V),
                "I_N" => Ok(I_N),
                "I_P" => Ok(I_P),
                "ADC0_5V" => Ok(ADC0_5V),
                "ADC1_5V" => Ok(ADC1_5V),
                "ADC2_5V" => Ok(ADC2_5V),
                "ADC3_8V" => Ok(ADC3_8V),
                "D0" => Ok(D0),
                "D1" => Ok(D1),
                "D2" => Ok(D2),
                "D3" => Ok(D3),
                "D4" => Ok(D4),
                "D5" => Ok(D5),
                "D6" => Ok(D6),
                "D7" => Ok(D7),
                "D8" => Ok(D8),
                "D9" => Ok(D9),
                "D10" => Ok(D10),
                "D11" => Ok(D11),
                "D12" => Ok(D12),
                "D13" => Ok(D13),
                "A0" => Ok(A0),
                "A1" => Ok(A1),
                "A2" => Ok(A2),
                "A3" => Ok(A3),
                "A4" => Ok(A4),
                "A5" => Ok(A5),
                "A6" => Ok(A6),
                "A7" => Ok(A7),
                "RESET" => Ok(RESET),
                "AREF" => Ok(AREF),
                _ => Err(anyhow::anyhow!("Unknown node"))}
        }
    }
}

impl std::fmt::Display for Node {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Node::Column(n) => write!(f, "{n}"),
            named => write!(f, "{named:?}"),
        }
    }
}

pub struct Connection(Node, Node);

impl std::fmt::Display for Connection {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}-{}", self.0, self.1)
    }
}

impl Connection {
    pub fn parse(source: &str) -> anyhow::Result<Self> {
        let (a, b) = source.split_once("-").ok_or(anyhow::anyhow!("Invalid segment: {}", source))?;
        Ok(Connection(Node::parse(a)?, Node::parse(b)?))
    }
}

impl PartialEq for Connection {
    fn eq(&self, other: &Self) -> bool {
        (self.0 == other.0 && self.1 == other.1) || (self.0 == other.1 && self.1 == other.0)
    }
}

pub struct NodeFile(Vec<Connection>);

impl NodeFile {
    pub fn parse(source: &str) -> anyhow::Result<Self> {
        let mut connections = vec![];
        for segment in source.split(",") {
            connections.push(Connection::parse(segment)?);
        }
        Ok(NodeFile(connections))
    }

    pub fn add_connection(&mut self, connection: Connection) {
        self.0.push(connection)
    }

    pub fn remove_connection(&mut self, connection: Connection) {
        self.0.retain(|c| *c != connection)
    }

    pub fn empty() -> Self {
        NodeFile(vec![])
    }
}

impl std::fmt::Display for NodeFile {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for (i, connection) in self.0.iter().enumerate() {
            if i != 0 {
                write!(f, ",")?;
            }
            write!(f, "{connection}")?;
        }
        Ok(())
    }
}

impl From<Vec<NetlistEntry>> for NodeFile {
    fn from(netlist: Vec<NetlistEntry>) -> Self {
        let mut bridges = HashSet::new();
        for entry in &netlist {
            for bridge in entry.bridges[1..(entry.bridges.len() - 1)].split(",") {
                bridges.insert(bridge);
            }
        }

        let mut connections = vec![];
        for bridge in bridges {
            if bridge == "0-0" { // this is used as a placeholder
                continue
            }
            connections.push(Connection::parse(bridge).expect("invalid bridge"));
        }
        Self(connections)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_node() {
        assert_eq!(Node::GND.to_string(), "GND".to_string());
        assert_eq!(Node::col(27).unwrap().to_string(), "27".to_string());
    }

    #[test]
    fn test_format_connection() {
        assert_eq!(Connection(Node::SUPPLY_5V, Node::col(33).unwrap()).to_string(), "SUPPLY_5V-33".to_string());
    }

    #[test]
    fn test_format_node_file() {
        assert_eq!(NodeFile(vec![
            Connection(Node::SUPPLY_5V, Node::col(33).unwrap()),
            Connection(Node::col(44).unwrap(), Node::col(27).unwrap()),
            Connection(Node::col(12).unwrap(), Node::col(13).unwrap()),
        ]).to_string(), "SUPPLY_5V-33,44-27,12-13".to_string());
    }
}
