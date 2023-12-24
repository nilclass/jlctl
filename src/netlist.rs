use std::collections::HashSet;

use serde::{Deserialize, Serialize};

#[allow(dead_code)]
#[derive(Debug, Serialize, PartialEq)]
pub struct NetlistEntry {
    pub index: u32,
    pub name: String,
    pub number: u32,
    pub nodes: String,
    pub bridges: String,
}

#[allow(non_camel_case_types)]
#[allow(clippy::upper_case_acronyms)]
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
        if (1..=60).contains(&n) {
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

                // ALIASES: these are names used for the nodes in the netlist output.
                //   They are not supported as input for nodefiles.
                "5V" => Ok(SUPPLY_5V),
                "3V3" => Ok(SUPPLY_3V3),
                "DAC_0" => Ok(DAC_0_5V),
                "DAC_1" => Ok(DAC_1_8V),
                "I_NEG" => Ok(I_N),
                "I_POS" => Ok(I_P),

                _ => Err(anyhow::anyhow!("Unknown node: {}", s)),
            }
        }
    }
}

impl Serialize for Node {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        match self {
            Node::Column(n) => serializer.serialize_u8(*n),
            other => serializer.serialize_str(other.to_string().as_str()),
        }
    }
}

impl<'de> Deserialize<'de> for Node {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        deserializer.deserialize_any(NodeVisitor)
    }
}

struct NodeVisitor;

impl<'de> serde::de::Visitor<'de> for NodeVisitor {
    type Value = Node;

    fn expecting(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        f.write_str("an integer between 1 and 60 or a string describing a known node")
    }

    fn visit_u64<E>(self, v: u64) -> Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        Node::col(v as u8).ok_or_else(|| E::custom(format!("Node number out of range: {}", v)))
    }

    fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        Node::parse(v).map_err(|e| E::custom(e.to_string()))
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

#[derive(Copy, Clone, Serialize, Deserialize)]
pub struct Connection(pub Node, pub Node);

impl std::fmt::Display for Connection {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}-{}", self.0, self.1)
    }
}

impl Connection {
    pub fn parse(source: &str) -> anyhow::Result<Self> {
        let (a, b) = source
            .split_once('-')
            .ok_or(anyhow::anyhow!("Invalid segment: {}", source))?;
        Ok(Connection(Node::parse(a)?, Node::parse(b)?))
    }
}

impl PartialEq for Connection {
    fn eq(&self, other: &Self) -> bool {
        (self.0 == other.0 && self.1 == other.1) || (self.0 == other.1 && self.1 == other.0)
    }
}

#[derive(Serialize, Deserialize)]
pub struct NodeFile(Vec<Connection>);

impl NodeFile {
    pub fn parse(source: &str) -> anyhow::Result<Self> {
        let mut connections = vec![];
        for segment in source.split(',') {
            connections.push(Connection::parse(segment)?);
        }
        Ok(NodeFile(connections))
    }

    pub fn add_connection(&mut self, connection: Connection) {
        if !self.has(connection) {
            self.0.push(connection)
        }
    }

    pub fn remove_connection(&mut self, connection: Connection) {
        self.0.retain(|c| *c != connection)
    }

    pub fn add_from(&mut self, other: NodeFile) {
        for c in other.0 {
            self.add_connection(c);
        }
    }

    pub fn remove_from(&mut self, other: NodeFile) {
        for c in other.0 {
            self.remove_connection(c);
        }
    }

    fn has(&self, connection: Connection) -> bool {
        self.0.iter().any(|c| *c == connection)
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
            for bridge in entry.bridges[1..(entry.bridges.len() - 1)].split(',') {
                bridges.insert(bridge);
            }
        }

        let mut connections = vec![];
        for bridge in bridges {
            if bridge == "0-0" {
                // this is used as a placeholder
                continue;
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
        assert_eq!(
            Connection(Node::SUPPLY_5V, Node::col(33).unwrap()).to_string(),
            "SUPPLY_5V-33".to_string()
        );
    }

    #[test]
    fn test_format_node_file() {
        assert_eq!(
            NodeFile(vec![
                Connection(Node::SUPPLY_5V, Node::col(33).unwrap()),
                Connection(Node::col(44).unwrap(), Node::col(27).unwrap()),
                Connection(Node::col(12).unwrap(), Node::col(13).unwrap()),
            ])
            .to_string(),
            "SUPPLY_5V-33,44-27,12-13".to_string()
        );
    }

    #[test]
    fn test_nodefile_add_and_remove() {
        let mut file = NodeFile(vec![]);
        assert_eq!(file.to_string(), "".to_string());
        file.add_connection(Connection::parse("7-13").unwrap());
        assert_eq!(file.to_string(), "7-13".to_string());
        file.add_connection(Connection::parse("14-22").unwrap());
        assert_eq!(file.to_string(), "7-13,14-22".to_string());
        // duplicate
        file.add_connection(Connection::parse("14-22").unwrap());
        assert_eq!(file.to_string(), "7-13,14-22".to_string());
        // also duplicate (different order)
        file.add_connection(Connection::parse("22-14").unwrap());
        assert_eq!(file.to_string(), "7-13,14-22".to_string());

        // add a few more, from nodefile
        file.add_from(NodeFile::parse("11-19,12-14,19-33").unwrap());
        assert_eq!(file.to_string(), "7-13,14-22,11-19,12-14,19-33".to_string());

        // remove one
        file.remove_connection(Connection::parse("7-13").unwrap());
        assert_eq!(file.to_string(), "14-22,11-19,12-14,19-33".to_string());
        // remove one (different order)
        file.remove_connection(Connection::parse("14-12").unwrap());
        assert_eq!(file.to_string(), "14-22,11-19,19-33".to_string());

        // remove a few more, from nodefile
        file.remove_from(NodeFile::parse("33-19,14-22").unwrap());
        assert_eq!(file.to_string(), "11-19".to_string());
    }
}
