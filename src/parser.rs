use std::io::{Read, BufReader, BufRead};
use nom::IResult;
use nom::combinator::map;
use nom::sequence::tuple;
use nom::bytes::complete::{take_while, take_till};
use crate::netlist::NetlistEntry;

pub fn parse_netlist<R: Read>(r: &mut R) -> Vec<NetlistEntry> {
    let mut lines = BufReader::new(r).lines().map(|l| {
        l.unwrap().trim().to_owned()
    }).peekable();
    lines.find(|l| {l == &"netlist"}).unwrap();
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

    return entries
}

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
