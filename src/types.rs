use serde::{Deserialize, Serialize};

use crate::parser;

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

/// this is the net format expected by the device, for `::netlist` input.
/// It's currently different from the `Net` format used internally by jlctl.
#[derive(Serialize)]
pub struct TmpNet {
    pub index: u8,
    pub number: u8,
    pub nodes: String,
    pub special: bool,
    pub color: Color,
    pub machine: bool,
    pub name: String,
}

impl From<Net> for TmpNet {
    fn from(
        Net {
            index,
            number,
            nodes,
            special,
            color,
            machine,
            name,
        }: Net,
    ) -> Self {
        let node_strings: Vec<String> = nodes.into_iter().map(|node| node.to_string()).collect();
        TmpNet {
            index,
            number,
            special,
            color,
            machine,
            name,
            nodes: node_strings.join(","),
        }
    }
}

/// A message received from the jumperless
#[derive(Debug, Clone, PartialEq)]
pub enum Message {
    Ok(Option<u32>),
    Error(Option<u32>),
    NetlistBegin,
    NetlistEnd,
    Net(Net),
    Bridgelist(Bridgelist),
    SupplySwitch(SupplySwitchPos),
}

pub type Bridgelist = Vec<(Node, Node)>;

/// Represents the position of the supply switch.
///
/// NOTE: the Jumperless cannot detect the actual state of the switch.
///   Instead the user must correctly advertise the state to the board,
///   for the power rows to be lit up correctly.
#[derive(Copy, Clone, Debug, PartialEq)]
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

impl std::fmt::Display for SupplySwitchPos {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self {
                SupplySwitchPos::V3_3 => "3.3V",
                SupplySwitchPos::V5 => "5V",
                SupplySwitchPos::V8 => "8V",
            }
        )
    }
}

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
        let trimmed = value
            .trim_start_matches("0x")
            .trim_start_matches("0X")
            .trim_start_matches('#');
        let (_, color) = parser::color(trimmed)
            .map_err(|e| anyhow::anyhow!("Failed to parse color: {:?}", e))?;
        Ok(color)
    }
}

impl Serialize for Color {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
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
        if !v.starts_with('#') {
            return Err(E::custom("Invalid color, expected to start with '#'"));
        }
        let (_, color) =
            parser::color(&v[1..]).map_err(|e| E::custom(format!("Error: {:?}", e)))?;
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
    DAC0,
    DAC1,
    ISENSE_MINUS,
    ISENSE_PLUS,
    ADC0,
    ADC1,
    ADC2,
    ADC3,
    NANO_D0,
    NANO_D1,
    NANO_D2,
    NANO_D3,
    NANO_D4,
    NANO_D5,
    NANO_D6,
    NANO_D7,
    NANO_D8,
    NANO_D9,
    NANO_D10,
    NANO_D11,
    NANO_D12,
    NANO_D13,
    NANO_A0,
    NANO_A1,
    NANO_A2,
    NANO_A3,
    NANO_A4,
    NANO_A5,
    NANO_A6,
    NANO_A7,
    NANO_RESET,
    NANO_AREF,
    RP_GPIO_0,
    RP_UART_Rx,
    RP_UART_Tx,
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

                // these are the canonical names
                "GND" => Ok(GND),
                "SUPPLY_5V" => Ok(SUPPLY_5V),
                "SUPPLY_3V3" => Ok(SUPPLY_3V3),
                "DAC0" => Ok(DAC0),
                "DAC1" => Ok(DAC1),
                "ISENSE_MINUS" => Ok(ISENSE_MINUS),
                "ISENSE_PLUS" => Ok(ISENSE_PLUS),
                "ADC0" => Ok(ADC0),
                "ADC1" => Ok(ADC1),
                "ADC2" => Ok(ADC2),
                "ADC3" => Ok(ADC3),
                "NANO_D0" => Ok(NANO_D0),
                "NANO_D1" => Ok(NANO_D1),
                "NANO_D2" => Ok(NANO_D2),
                "NANO_D3" => Ok(NANO_D3),
                "NANO_D4" => Ok(NANO_D4),
                "NANO_D5" => Ok(NANO_D5),
                "NANO_D6" => Ok(NANO_D6),
                "NANO_D7" => Ok(NANO_D7),
                "NANO_D8" => Ok(NANO_D8),
                "NANO_D9" => Ok(NANO_D9),
                "NANO_D10" => Ok(NANO_D10),
                "NANO_D11" => Ok(NANO_D11),
                "NANO_D12" => Ok(NANO_D12),
                "NANO_D13" => Ok(NANO_D13),
                "NANO_A0" => Ok(NANO_A0),
                "NANO_A1" => Ok(NANO_A1),
                "NANO_A2" => Ok(NANO_A2),
                "NANO_A3" => Ok(NANO_A3),
                "NANO_A4" => Ok(NANO_A4),
                "NANO_A5" => Ok(NANO_A5),
                "NANO_A6" => Ok(NANO_A6),
                "NANO_A7" => Ok(NANO_A7),
                "NANO_RESET" => Ok(NANO_RESET),
                "NANO_AREF" => Ok(NANO_AREF),
                "RP_GPIO_0" => Ok(RP_GPIO_0),
                "RP_UART_Rx" => Ok(RP_UART_Rx),
                "RP_UART_Tx" => Ok(RP_UART_Tx),



                // ALIASES: these are names used for the nodes in the netlist output.
                //   They are not supported as input for nodefiles.

                "5V" => Ok(SUPPLY_5V),
                "3V3" => Ok(SUPPLY_3V3),
                "DAC0_5V" => Ok(DAC0),
                "DAC1_8V" => Ok(DAC1),
                "I_N" => Ok(ISENSE_MINUS),
                "I_P" => Ok(ISENSE_PLUS),
                "ADC0_5V" => Ok(ADC0),
                "ADC1_5V" => Ok(ADC1),
                "ADC2_5V" => Ok(ADC2),
                "ADC3_8V" => Ok(ADC3),
                "D0" => Ok(NANO_D0),
                "D1" => Ok(NANO_D1),
                "D2" => Ok(NANO_D2),
                "D3" => Ok(NANO_D3),
                "D4" => Ok(NANO_D4),
                "D5" => Ok(NANO_D5),
                "D6" => Ok(NANO_D6),
                "D7" => Ok(NANO_D7),
                "D8" => Ok(NANO_D8),
                "D9" => Ok(NANO_D9),
                "D10" => Ok(NANO_D10),
                "D11" => Ok(NANO_D11),
                "D12" => Ok(NANO_D12),
                "D13" => Ok(NANO_D13),
                "A0" => Ok(NANO_A0),
                "A1" => Ok(NANO_A1),
                "A2" => Ok(NANO_A2),
                "A3" => Ok(NANO_A3),
                "A4" => Ok(NANO_A4),
                "A5" => Ok(NANO_A5),
                "A6" => Ok(NANO_A6),
                "A7" => Ok(NANO_A7),
                "RESET" => Ok(NANO_RESET),
                "AREF" => Ok(NANO_AREF),
                "GPIO_0" => Ok(RP_GPIO_0),
                "UART_Rx" => Ok(RP_UART_Rx),
                "UART_Tx" => Ok(RP_UART_Tx),

                "DAC 0" => Ok(DAC0),
                "DAC 1" => Ok(DAC1),
                "DAC_0" => Ok(DAC0),
                "DAC_1" => Ok(DAC1),
                "I_NEG" => Ok(ISENSE_MINUS),
                "I_POS" => Ok(ISENSE_PLUS),
                "ADC_0" => Ok(ADC0),
                "ADC_1" => Ok(ADC1),
                "ADC_2" => Ok(ADC2),
                "ADC_3" => Ok(ADC3),
                "GPIO_16" => Ok(RP_UART_Rx),
                "GPIO_17" => Ok(RP_UART_Tx),

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
