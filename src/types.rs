use serde::{Serialize, Deserialize};

use crate::new_parser;

/// Represents a named set of connected Nodes
#[derive(Debug, PartialEq, Clone, Serialize, Deserialize)]
pub struct Net {
    pub index: u8,
    pub number: u8,
    pub nodes: Vec<Node>,
    pub special: bool,
    pub color: Color,
    pub machine: bool,
    pub name: String,
}

/// A message received from the jumperless
#[derive(Debug, Clone, PartialEq)]
pub enum Message {
    NetlistBegin,
    NetlistEnd,
    Net(Net),
    Bridgelist(Bridgelist),
}

pub type Bridgelist = Vec<(Node, Node)>;

#[derive(Debug, Copy, Clone, PartialEq)]
pub struct Color(pub [u8; 3]);

impl From<Color> for u32 {
    fn from(Color([r, g, b]): Color) -> Self {
        ((r as u32) << 16) | ((g as u32) << 8) | (b as u32)
    }
}

impl std::fmt::Display for Color {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let Color([r, g, b]) = self;
        write!(f, "#{:02x}{:02x}{:02x}", r, g, b)
    }
}

impl TryFrom<String> for Color {
    type Error = anyhow::Error;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        let trimmed = value.trim_start_matches("0x").trim_start_matches("0X").trim_start_matches("#");
        let (_, color) = new_parser::color(trimmed).map_err(|e| anyhow::anyhow!("Failed to parse color: {:?}", e))?;
        Ok(color)
    }
}

impl Serialize for Color {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer {
        serializer.serialize_str(&self.to_string())
    }
}

impl<'de> Deserialize<'de> for Color {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        deserializer.deserialize_any(ColorVisitor)
    }
}

struct ColorVisitor;

impl<'de> serde::de::Visitor<'de> for ColorVisitor {
    type Value = Color;

    fn expecting(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        f.write_str("a color formatted as '#rrggbb'")
    }

    fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        if !v.starts_with("#") {
            return Err(E::custom("Invalid color, expected to start with '#'"))
        }
        let (_, color) = new_parser::color(&v[1..]).map_err(|e| E::custom(format!("Error: {:?}", e)))?;
        Ok(color)
    }
}

/// Represents a node on the jumperless.
///
/// A node is everything that can be connected to any other nodes
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
    /// Construct Node for given column number, if it is in the valid range.
    pub fn col(n: u8) -> Option<Self> {
        if (1..=60).contains(&n) {
            Some(Node::Column(n))
        } else {
            None
        }
    }

    pub fn parse(s: &str) -> anyhow::Result<Self> {
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
