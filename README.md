# jlctl - A Jumperless CLI

`jlctl` is a command line tool for controlling @Architeuthis-Flux's awesome [Jumperless breadboard](https://github.com/Architeuthis-Flux/Jumperless/).

## Features

- List and modify bridges (aka connections) via the command line
- HTTP server for doing the same

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

`jlctl` needs access to the serial port of the jumperless. By default it will try to find the port itself.
If that fails, or if you want to restrict `jlctl` to a specific serial port, use the `--port <port>` option.

### Help

```
$ jlctl help
CLI for the jumperless breadboard

Usage: jlctl [OPTIONS] <COMMAND>

Commands:
  list-ports  List serial ports
  netlist     Print current netlist
  bridge      Interact with bridges
  server      Start HTTP server
  help        Print this message or the help of the given subcommand(s)

Options:
  -p, --port <PORT>  Serial port where the Jumperless is connected. If omitted, the port is detected dynamically
  -h, --help         Print help
  -V, --version      Print version
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

### Troubleshooting

jlctl uses the [`env_logger`](https://docs.rs/env_logger/0.10.1/env_logger/) package to facilitate logging.
Check out it's documentation to find out about all the options.
The log level defaults to `info`. A good place to start is setting it to `debug`:

```
RUST_LOG=debug jlctl ...
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

When run as a server, jlctl will try to open the device once the first request comes in.
It then keeps that device open and uses it for subsequent requests.
If any request fails to communicate with the device, that request will fail (with status 502),
but subsequent requests will try to open the device again.

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
