use std::collections::HashMap;

use crate::types::{Net, Node};

const SPECIAL_NETS: [(u8, &str, Node); 7] = [
    (1, "GND", Node::GND),
    (2, "+5V", Node::SUPPLY_5V),
    (3, "+3.3V", Node::SUPPLY_3V3),
    (4, "DAC 0", Node::DAC0),
    (5, "DAC 1", Node::DAC1),
    (6, "I Sense +", Node::ISENSE_PLUS),
    (7, "I Sense -", Node::ISENSE_MINUS),
];

pub fn netlist(netlist: Vec<Net>) -> anyhow::Result<Vec<Net>> {
    let mut by_index = HashMap::new();

    for net in &netlist {
        if by_index.contains_key(&net.index) {
            return Err(anyhow::anyhow!("Duplicate index {}", net.index));
        }
        by_index.insert(net.index, net);
    }

    for (index, name, node) in &SPECIAL_NETS {
        if let Some(net) = by_index.get(index) {
            if net.name.as_str() != *name {
                return Err(anyhow::anyhow!(
                    "Special net {} (index: {}) cannot be renamed",
                    name,
                    index
                ));
            }
            if !net.nodes.contains(node) {
                return Err(anyhow::anyhow!(
                    "Special net {} (index: {}) is missing node {:?}",
                    name,
                    index,
                    node
                ));
            }
        } else {
            return Err(anyhow::anyhow!(
                "Special net {} (index: {}) missing",
                name,
                index
            ));
        }
    }

    Ok(netlist)
}

#[cfg(test)]
mod tests {}
