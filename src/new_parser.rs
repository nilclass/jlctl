use nom::{
    branch::alt,
    bytes::complete::{tag, take_till},
    character::complete::u8,
    combinator::{map, map_res, value, all_consuming},
    multi::{separated_list1, separated_list0},
    sequence::{tuple, separated_pair},
    IResult,
};
use crate::types::{Message, Net, Node, Color, Bridgelist};

pub fn message(input: &str) -> IResult<&str, Message> {
    use Message::*;
    all_consuming(
        alt((
            map(ok_response, |_| Ok),
            map(error_response, |_| Error),
            map(netlist_begin, |_| NetlistBegin),
            map(netlist_end, |_| NetlistEnd),
            map(net, Net),
            map(bridgelist, Bridgelist),
        ))
    )(input)
}

pub fn ok_response(input: &str) -> IResult<&str, ()> {
    value((), tag("::ok"))(input)
}

pub fn error_response(input: &str) -> IResult<&str, ()> {
    value((), tag("::error"))(input)
}

pub fn netlist_begin(input: &str) -> IResult<&str, ()> {
    value((), tag("::netlist-begin"))(input)
}

pub fn netlist_end(input: &str) -> IResult<&str, ()> {
    value((), tag("::netlist-end"))(input)
}

pub fn net(input: &str) -> IResult<&str, Net> {
    map(
        tuple((
            tag("::net["),
            u8, // index
            tag(","),
            u8, // number
            tag(","),
            nodes,
            tag(","),
            boolean, // special
            tag(","),
            color,
            tag(","),
            boolean, // machine
            tag(","),
            name,
            tag("]"),
        )),
        |(_, index, _, number, _, nodes, _, special, _, color, _, machine, _, name, _)| Net {
            index,
            number,
            nodes,
            special,
            color,
            machine,
            name,
        },
    )(input)
}

fn boolean(input: &str) -> IResult<&str, bool> {
    alt((value(true, tag("true")), value(false, tag("false"))))(input)
}

pub fn color(input: &str) -> IResult<&str, Color> {
    map(tuple((color_part, color_part, color_part)), |(r, g, b)| {
        Color([r, g, b])
    })(input)
}

fn color_part(input: &str) -> IResult<&str, u8> {
    match u8::from_str_radix(&input[0..2], 16) {
        Ok(value) => Ok((&input[2..], value)),
        Err(err) => {
            eprintln!("WARNING: ignoring error parsing color part. Input: {:?}, Error: {:?}", &input[0..2], err);
            Ok((&input[2..], 0))
        }
    }
}

fn nodes(input: &str) -> IResult<&str, Vec<Node>> {
    separated_list1(tag(";"), node)(input)
}

fn node(input: &str) -> IResult<&str, Node> {
    map_res(take_till(|c| c == ';' || c == ',' || c == '-' || c == ']'), |s: &str| {
        Node::parse(s)
    })(input)
}

fn name(input: &str) -> IResult<&str, String> {
    map(take_till(|c| c == ']'), |s: &str| s.to_string())(input)
}

fn bridgelist(input: &str) -> IResult<&str, Bridgelist> {
    map(
        tuple((tag("::bridgelist["), bridges, tag("]"))),
        |(_, bridges, _)| bridges,
    )(input)
}

pub fn bridges(input: &str) -> IResult<&str, Bridgelist> {
    separated_list0(tag(","), bridge)(input)
}

fn bridge(input: &str) -> IResult<&str, (Node, Node)> {
    separated_pair(node, tag("-"), node)(input)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_netlist_begin() {
        assert_eq!(netlist_begin("::netlist-begin"), Ok(("", ())));
    }

    #[test]
    fn test_net() {
        assert_eq!(net("::net[1,1,GND,true,001c04,false,GND]"), Ok(("", Net {
            index: 1,
            number: 1,
            nodes: vec![Node::GND],
            special: true,
            color: Color([0x00, 0x1c, 0x04]),
            machine: false,
            name: "GND".to_string(),
        })));
    }

    #[test]
    fn test_color() {
        assert_eq!(color("000000"), Ok(("", Color([0, 0, 0]))));
        assert_eq!(color("00AA00"), Ok(("", Color([0, 0xAA, 0]))));
        assert_eq!(color("123456"), Ok(("", Color([0x12, 0x34, 0x56]))));
    }

    #[test]
    fn test_boolean() {
        assert_eq!(boolean("true"), Ok(("", true)));
        assert_eq!(boolean("false"), Ok(("", false)));
    }

    #[test]
    fn test_nodes() {
        assert_eq!(nodes("GND;17;23;3V3"), Ok(("", vec![
            Node::GND,
            Node::Column(17),
            Node::Column(23),
            Node::SUPPLY_3V3,
        ])));
    }

    #[test]
    fn test_bridgelist() {
        let input = "::bridgelist[GND-17,GND-5,GND-50,GND-32,5V-7,5V-15,5V-A7,3V3-55,3V3-A4,27-8,27-11,27-20]";

        use Node::*;

        assert_eq!(bridgelist(input), Ok(("", vec![
            (GND, Column(17)),
            (GND, Column(5)),
            (GND, Column(50)),
            (GND, Column(32)),
            (SUPPLY_5V, Column(7)),
            (SUPPLY_5V, Column(15)),
            (SUPPLY_5V, A7),
            (SUPPLY_3V3, Column(55)),
            (SUPPLY_3V3, A4),
            (Column(27), Column(8)),
            (Column(27), Column(11)),
            (Column(27), Column(20)),
        ])));
    }

    const INITIAL_NETLIST: [&str; 9] = [
        "::netlist-begin",
        "::net[1,1,GND,true,001c04,false,GND]",
        "::net[2,2,5V,true,1c0702,false,+5V]",
        "::net[3,3,3V3,true,1c0107,false,+3.3V]",
        "::net[4,4,DAC_0,true,231111,false,DAC 0]",
        "::net[5,5,DAC_1,true,230913,false,DAC 1]",
        "::net[6,6,I_POS,true,232323,false,I Sense +]",
        "::net[7,7,I_NEG,true,232323,false,I Sense -]",
        "::netlist-end",
    ];

    #[test]
    fn test_initial_netlist() {
        let result: Vec<Message> = INITIAL_NETLIST.iter().map(|line| {
            let (rest, msg) = message(line).unwrap();
            assert_eq!(rest, "");
            msg
        }).collect();
        assert_eq!(result, vec![
            Message::NetlistBegin,
            Message::Net(Net {
                index: 1,
                number: 1,
                nodes: vec![Node::GND],
                special: true,
                color: Color([0x00, 0x1c, 0x04]),
                machine: false,
                name: "GND".to_string(),
            }),
            Message::Net(Net {
                index: 2,
                number: 2,
                nodes: vec![Node::SUPPLY_5V],
                special: true,
                color: Color([0x1c, 0x07, 0x02]),
                machine: false,
                name: "+5V".to_string(),
            }),
            Message::Net(Net {
                index: 3,
                number: 3,
                nodes: vec![Node::SUPPLY_3V3],
                special: true,
                color: Color([0x1c, 0x01, 0x07]),
                machine: false,
                name: "+3.3V".to_string(),
            }),
            Message::Net(Net {
                index: 4,
                number: 4,
                nodes: vec![Node::DAC_0_5V],
                special: true,
                color: Color([0x23, 0x11, 0x11]),
                machine: false,
                name: "DAC 0".to_string(),
            }),
            Message::Net(Net {
                index: 5,
                number: 5,
                nodes: vec![Node::DAC_1_8V],
                special: true,
                color: Color([0x23, 0x09, 0x13]),
                machine: false,
                name: "DAC 1".to_string(),
            }),
            Message::Net(Net {
                index: 6,
                number: 6,
                nodes: vec![Node::I_P],
                special: true,
                color: Color([0x23, 0x23, 0x23]),
                machine: false,
                name: "I Sense +".to_string(),
            }),
            Message::Net(Net {
                index: 7,
                number: 7,
                nodes: vec![Node::I_N],
                special: true,
                color: Color([0x23, 0x23, 0x23]),
                machine: false,
                name: "I Sense -".to_string(),
            }),
            Message::NetlistEnd,
        ]);
    }
}
