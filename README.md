# jlctl - A Jumperless CLI

`jlctl` is a command line tool for controlling @Architeuthis-Flux's awesome [Jumperless breadboard](https://github.com/Architeuthis-Flux/Jumperless/).

## Installation

Installation is currently from source.

**Prerequisite**:
- a working Rust toolchain. If you don't have one, visit https://rustup.rs/ to get one.

**Building**:
```bash
cargo build --release
```

You'll find the binary in `./target/release/jlctl`. Copy it wherever you like!

## Usage

```
$ jlctl
CLI for the jumperless breadboard

Usage: jlctl [OPTIONS] <COMMAND>

Commands:
  netlist  Print current netlist
  bridge   Interact with bridges
  help     Print this message or the help of the given subcommand(s)

Options:
  -p <PORT>      [default: /dev/ttyACM0]
  -h, --help     Print help
  -V, --version  Print version
```

### `jlctl netlist`

Sends `n` to the jumperless, parses the output and prints it to stdout in JSON format.

Example:
```
$ jlctl netlist
[
  {
    "index": 0,
    "name": "Empty Net",
    "number": 127,
    "nodes": "EMPTY_NET",
    "bridges": "{0-0}"
  },
  {
    "index": 1,
    "name": "GND",
    "number": 1,
    "nodes": "GND,17",
    "bridges": "{GND-17}"
  },
  {
    "index": 2,
    "name": "+5V",
    "number": 2,
    "nodes": "5V",
    "bridges": "{0-0}"
  },
  ...
]
```

### `jlctl bridge get`

Prints current bridges in "nodefile" format.

Example:
```
$ jlctl bridge get
3-60,GND-17
```

### `jlctl bridge add <bridges>`

Adds bridges (connections). This takes the current nodefile, merges it with what's passed on the command line and sends it to the jumperless.

Example:
```
$ jlctl bridge add 3-12,4-19
$ jlctl bridge get
3-12,3-60,GND-17,4-19
```

### `jlctl bridge remove <bridges>`

Remove existing bridges. Opposite of add.
```
$ jlctl bridge get
3-12,3-60,GND-17,4-19
$ jlctl bridge remove 17-GND,4-19
$ jlctl bridge get
3-12,3-60
```

### `jlctl bridge clear`

Remove all bridges. Sends an empty nodefile to the jumperless.

```
$ jlctl bridge clear
$ jlctl bridge get

```
