# jlctl - A Jumperless CLI

`jlctl` is a command line tool for controlling @Architeuthis-Flux's awesome [Jumperless breadboard](https://github.com/Architeuthis-Flux/Jumperless/).

## Installation

### Binary release

Check the [Releases](https://github.com/nilclass/jlctl/releases) page for binary releases, and follow instructions from there.

### From source

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

### Quick start

List serial ports to see if the board is detected:

```
$ jlctl list-ports
╭──────────────┬───────────┬───────────────────╮
│ Port Name    ┆ USB ID    ┆ Role              │
╞══════════════╪═══════════╪═══════════════════╡
│ /dev/ttyACM0 ┆ acab:1312 ┆ JumperlessPrimary │
├╌╌╌╌╌╌╌╌╌╌╌╌╌╌┼╌╌╌╌╌╌╌╌╌╌╌┼╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌┤
│ /dev/ttyACM1 ┆ acab:1312 ┆ JumperlessArduino │
╰──────────────┴───────────┴───────────────────╯
```

If you see a port with role "JumperlessPrimary", you're good to go.


Connect some rows together:
```
$ jlctl bridge set 3-7,14-2
```

Rows 3, 7, 14 and 2 should be lit up on your board now.

Retrieve the current list of bridges:
```
$ jlctl bridge list
3-7,14-2
```

Retrieve the list of nets:
```
$ jlctl net list
```

### Built-in Help

Use the `help` command to get a list of commands and options:

```
$ jlctl help
CLI for the jumperless breadboard

Usage: jlctl [OPTIONS] <COMMAND>

Commands:
  list-ports         List serial ports
  identify-port      Identify primary Jumperless port
  raw                Send a raw command to the Jumperless
  net                Interact with nets
  bridge             Interact with bridges
  supply-switch-pos  Inform Jumperless about it's switch position
  lightnet           Set color for given light
  server             Start HTTP server
  help               Print this message or the help of the given subcommand(s)

Options:
  -p, --port <PORT>          Serial port where the Jumperless is connected. If omitted, the port is detected dynamically
  -l, --log-path <LOG_PATH>  Capture device log in this file [default: log.txt]
  -h, --help                 Print help
  -V, --version              Print version
```

To get help for a specific command, run `jlctl help <command>`, e.g.
```
$ jlctl help bridge
Interact with bridges

Usage: jlctl bridge <COMMAND>

Commands:
  list   Download list of bridges from the Jumperless
  set    Upload new list of bridges to the Jumperless
  clear  Upload empty list of bridges to the jumperless
  help   Print this message or the help of the given subcommand(s)

Options:
  -h, --help  Print help
```

This works for subcommands as well:
```
$ jctl help bridge list
Download list of bridges from the Jumperless

Usage: jlctl bridge list [OPTIONS]

Options:
  -f, --file <FILE>  Write to file instead of stdout
  -h, --help         Print help
```

etc.

### Troubleshooting

jlctl uses the [`env_logger`](https://docs.rs/env_logger/0.10.1/env_logger/) package to facilitate logging.
Check out it's documentation to find out about all the options.
The log level defaults to `info`. A good place to start is setting it to `debug`:

```
RUST_LOG=debug jlctl ...
```

### Usage from scripts

Many of the commands support JSON input and output. Check `help` for details.

If you are missing some feature, please open an issue or a PR.

Some examples (using [`jq`](https://github.com/jqlang/jq) for JSON processing):

- Identify the Arduino Port (e.g. to pass it to avrdude):
  ```
  $ jlctl list-ports -o json | jq -r '.[] | select(.role == "JumperlessArduino") | .info.port_name'
  /dev/ttyACM1
  ```
- Print color of the `GND` node:
  ```
  $ jlctl net list -ojson | jq -r '.[] | select(.name == "GND") | .color'
  #001c04
  ```


## HTTP Server

`jlctl` includes an HTTP server, used by the [jumperlab UI](https://github.com/nilclass/jumperlab).

To start it, run
```
jlctl server
```

By default the server listens on `localhost:8080`. To change that, pass `--listen`:
```
jlctl server --listen 0.0.0.0:12345
```

Pass `0` as the port, to let the operating system choose a random one (e.g. `--listen localhost:0`).

When run as a server, jlctl will try to open the device once the first request comes in.
It then keeps that device open and uses it for subsequent requests.
If any request fails to communicate with the device, that request will fail (with status 502),
but subsequent requests will try to open the device again.

## Embedded Jumperlab

`jlctl` can be built with the [jumperlab UI](https://github.com/nilclass/jumperlab) included.
The UI will be pre-built, and included within the `jlctl` binary as a ZIP. This way both projects can be distributed as a single binary.

To build jlctl with jumperlab, take these steps:
1. Check out / update the jumperlab submodule, with:
   ```
   git submodule update --init
   ```
2. Build jlctl with the `jumperlab` feature
   ```
   cargo build --features jumperlab
   ```

The resulting build will server jumperlab on the `/jumperlab` path, when running in server mode.
A link to open jumperlab is also printed during server startup:
```
$ target/release/jlctl server
[2024-01-28T20:36:25Z INFO  jlctl::server] Starting HTTP server, listening on localhost:8080
[2024-01-28T20:36:25Z INFO  actix_server::builder] starting 1 workers
[2024-01-28T20:36:25Z INFO  actix_server::server] Actix runtime found; starting in Actix runtime

    To open Jumperlab, visit: http://localhost:8080/jumperlab

```
