use crate::netlist::NetlistEntry;
use nom::bytes::complete::{take_till, take_while};
use nom::combinator::map;
use nom::sequence::tuple;
use nom::IResult;
use std::io::{BufRead, BufReader, Read};

pub fn parse_netlist<R: Read>(r: &mut R) -> Vec<NetlistEntry> {
    let mut lines = BufReader::new(r)
        .lines()
        .map(|l| l.unwrap().trim().to_owned())
        .peekable();
    lines.find(|l| l == &"netlist").unwrap();
    lines.find(|l| l.starts_with("Index")).unwrap();

    let mut entries = vec![];

    while let Some(line) = lines.next() {
        if line.len() == 0 {
            if let Some(l) = lines.peek() {
                if l.starts_with("Index") {
                    lines.next(); // consume!
                    continue;
                }
            }
            break;
        }

        let (_, entry) = parse_netlist_line(&line).unwrap();
        entries.push(entry);
    }

    return entries;
}

fn parse_tabs(input: &str) -> IResult<&str, &str> {
    take_while(|c| c == '\t')(input)
}

fn parse_string(input: &str) -> IResult<&str, String> {
    map(take_till(|c| c == '\t'), |s: &str| s.to_string())(input)
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
        |(index, _, name, _, number, _, nodes, _, bridges)| NetlistEntry {
            index,
            name,
            number,
            nodes,
            bridges,
        },
    )(input)
}

#[cfg(test)]
mod tests {
    use std::io::Cursor;

    use super::*;

    const INITIAL_NETLIST: &str = "
netlist


Index	Name		Number		Nodes			Bridges
0	Empty Net	127		EMPTY_NET		{0-0}
1	GND		1		GND			{0-0}
2	+5V		2		5V			{0-0}
3	+3.3V		3		3V3			{0-0}
4	DAC 0		4		DAC_0			{0-0}
5	DAC 1		5		DAC_1			{0-0}
6	I Sense +	6		I_POS			{0-0}
7	I Sense -	7		I_NEG			{0-0}



			Menu

	n = show netlist
	b = show bridge array
	w = waveGen
	f = load formatted nodeFile
	p = paste new Wokwi diagram
	l = LED brightness / test
	d = toggle debug flags
	r = reset Arduino
";

    const NETLIST_WITH_BRIDGES: &str = "

netlist


Index	Name		Number		Nodes			Bridges
0	Empty Net	127		EMPTY_NET		{0-0}
1	GND		1		GND,17			{GND-17}
2	+5V		2		5V,13			{5V-13}
3	+3.3V		3		3V3			{0-0}
4	DAC 0		4		DAC_0			{0-0}
5	DAC 1		5		DAC_1			{0-0}
6	I Sense +	6		I_POS			{0-0}
7	I Sense -	7		I_NEG			{0-0}

Index	Name		Number		Nodes			Bridges
8	Net 8		8		12,24			{12-24}
9	Net 9		9		3,60			{3-60}


			Menu

	n = show netlist
	b = show bridge array
	w = waveGen
	f = load formatted nodeFile
	p = paste new Wokwi diagram
	l = LED brightness / test
	d = toggle debug flags
	r = reset Arduino

";

    #[test]
    fn test_parse_netlist_initial() {
        let mut reader = Cursor::new(INITIAL_NETLIST);
        let netlist = parse_netlist(&mut reader);
        assert_eq!(
            netlist,
            vec![
                NetlistEntry {
                    index: 0,
                    name: "Empty Net".to_string(),
                    number: 127,
                    nodes: "EMPTY_NET".to_string(),
                    bridges: "{0-0}".to_string(),
                },
                NetlistEntry {
                    index: 1,
                    name: "GND".to_string(),
                    number: 1,
                    nodes: "GND".to_string(),
                    bridges: "{0-0}".to_string(),
                },
                NetlistEntry {
                    index: 2,
                    name: "+5V".to_string(),
                    number: 2,
                    nodes: "5V".to_string(),
                    bridges: "{0-0}".to_string(),
                },
                NetlistEntry {
                    index: 3,
                    name: "+3.3V".to_string(),
                    number: 3,
                    nodes: "3V3".to_string(),
                    bridges: "{0-0}".to_string(),
                },
                NetlistEntry {
                    index: 4,
                    name: "DAC 0".to_string(),
                    number: 4,
                    nodes: "DAC_0".to_string(),
                    bridges: "{0-0}".to_string(),
                },
                NetlistEntry {
                    index: 5,
                    name: "DAC 1".to_string(),
                    number: 5,
                    nodes: "DAC_1".to_string(),
                    bridges: "{0-0}".to_string(),
                },
                NetlistEntry {
                    index: 6,
                    name: "I Sense +".to_string(),
                    number: 6,
                    nodes: "I_POS".to_string(),
                    bridges: "{0-0}".to_string(),
                },
                NetlistEntry {
                    index: 7,
                    name: "I Sense -".to_string(),
                    number: 7,
                    nodes: "I_NEG".to_string(),
                    bridges: "{0-0}".to_string(),
                },
            ]
        )
    }

    #[test]
    fn test_parse_netlist_with_bridges() {
        let mut reader = Cursor::new(NETLIST_WITH_BRIDGES);
        let netlist = parse_netlist(&mut reader);

        assert_eq!(
            netlist,
            vec![
                NetlistEntry {
                    index: 0,
                    name: "Empty Net".to_string(),
                    number: 127,
                    nodes: "EMPTY_NET".to_string(),
                    bridges: "{0-0}".to_string(),
                },
                NetlistEntry {
                    index: 1,
                    name: "GND".to_string(),
                    number: 1,
                    nodes: "GND,17".to_string(),
                    bridges: "{GND-17}".to_string(),
                },
                NetlistEntry {
                    index: 2,
                    name: "+5V".to_string(),
                    number: 2,
                    nodes: "5V,13".to_string(),
                    bridges: "{5V-13}".to_string(),
                },
                NetlistEntry {
                    index: 3,
                    name: "+3.3V".to_string(),
                    number: 3,
                    nodes: "3V3".to_string(),
                    bridges: "{0-0}".to_string(),
                },
                NetlistEntry {
                    index: 4,
                    name: "DAC 0".to_string(),
                    number: 4,
                    nodes: "DAC_0".to_string(),
                    bridges: "{0-0}".to_string(),
                },
                NetlistEntry {
                    index: 5,
                    name: "DAC 1".to_string(),
                    number: 5,
                    nodes: "DAC_1".to_string(),
                    bridges: "{0-0}".to_string(),
                },
                NetlistEntry {
                    index: 6,
                    name: "I Sense +".to_string(),
                    number: 6,
                    nodes: "I_POS".to_string(),
                    bridges: "{0-0}".to_string(),
                },
                NetlistEntry {
                    index: 7,
                    name: "I Sense -".to_string(),
                    number: 7,
                    nodes: "I_NEG".to_string(),
                    bridges: "{0-0}".to_string(),
                },
                NetlistEntry {
                    index: 8,
                    name: "Net 8".to_string(),
                    number: 8,
                    nodes: "12,24".to_string(),
                    bridges: "{12-24}".to_string(),
                },
                NetlistEntry {
                    index: 9,
                    name: "Net 9".to_string(),
                    number: 9,
                    nodes: "3,60".to_string(),
                    bridges: "{3-60}".to_string(),
                },
            ]
        );
    }
}
