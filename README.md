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

### Help

```
$ jlctl help
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

To get help for a subcommand, run `jlctl help <command>`, e.g.
```
$ jlctl help bridge
Interact with bridges

Usage: jlctl bridge <COMMAND>

Commands:
  get     Get current list of bridges
  add     Add new bridge(s)
  remove  Remove given bridge(s)
  clear   Remove all bridges
  help    Print this message or the help of the given subcommand(s)

Options:
  -h, --help  Print help
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

## HTTP Server

`jlctl` includes an HTTP server, exposing the same functionality as the CLI itself.

To start it, run
```
jlctl server
```

By default the server listens on `localhost:8080`. To change that, pass `--listen`:
```
jlctl server --listen 0.0.0.0:12345
```

### Netlist

#### `GET /netlist`

Retrieve current netlist

Example (output adjusted here for readability):
```
$ curl http://localhost:8080/netlist
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
    "nodes": "GND",
    "bridges": "{0-0}"
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

### Bridges

#### Format

The endpoints document below produce and consume JSON formatted bridges.

The JSON format is pretty simple:
- A **Node** is either:
  - a **number**: must be in the 1..60 (inclusive) range. Specifies one of the columns on the breadboard
    - Examples: `7`, `24`
  - a **string**: must be one of the recognized special node names. Check the implementation of `Node::parse` in `src/netlist.rs` for a list of supported values.
    - Examples: `GND`, `5V`, `SUPPLY_5V`
- A **Bridge** is an array containing exactly two Nodes
  - Examples: `[7, 24]`, `["GND", 14]`, `["SUPPLY_5V", "GND"]` (🤯)

#### `GET /bridges`

Retrieve current list of bridges

Example:
```
$ curl http://localhost:8080/bridges
[[14,"GND"],[15,"GND"],[17,23]]
```

#### `PUT /bridges`

Add the specified bridges

Example:
```
$ curl http://localhost:8080/bridges -XPUT -H content-type:application/json --data '[[17,23]]'
[[17,23]]
```

#### `DELETE /bridges`

Remove the specified bridges

Example:
```
$ curl http://localhost:8080/bridges -XDELETE -H content-type:application/json --data '[[17,23]]'
[]
```

#### `POST /bridges/clear`

Remove *all* bridges

Example:
```
$ curl http://localhost:8080/bridges/clear -XPOST
true
```
