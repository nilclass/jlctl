use clap::{Parser, Subcommand, ValueEnum};
use comfy_table::{modifiers::UTF8_ROUND_CORNERS, presets::UTF8_FULL, Table};
use device_manager::PortRole;
use env_logger::Env;
use shadow_rs::shadow;
use std::fs::File;
use log::info;

shadow!(build);

mod device;
mod device_manager;
mod server;
mod types;

mod new_parser;

#[derive(Debug, Parser)]
#[command(about = "CLI for the jumperless breadboard", version = build::CLAP_LONG_VERSION)]
struct Cli {
    /// Serial port where the Jumperless is connected. If omitted, the port is detected dynamically.
    #[arg(long, short)]
    port: Option<String>,

    /// Capture device log in this file
    #[arg(long, short, default_value = "log.txt")]
    log_path: String,

    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    /// List serial ports
    #[command()]
    ListPorts,

    /// Identify primary Jumperless port
    #[command()]
    IdentifyPort,

    /// Interact with nets
    #[command(subcommand, alias = "nets")]
    Net(NetCommand),

    /// Interact with bridges
    #[command(subcommand, alias = "bridges")]
    Bridge(BridgeCommand),

    /// Inform Jumperless about it's switch position
    #[command()]
    SupplySwitchPos {
        /// One of: 8V, 3.3V, 5V
        #[arg()]
        pos: device::SupplySwitchPos,
    },

    /// Set color for given light
    #[command()]
    Lightnet {
        /// Light to target (node name, or a special name like 'glow', 'logo', ...)
        #[arg()]
        name: String,

        /// Color. Must be 6-digit hex. Allowed prefixes: 0x, 0X, #
        #[arg()]
        color: String,
    },

    /// Start HTTP server
    #[command()]
    Server {
        #[arg(long, short, default_value = "localhost:8080")]
        listen: String,
    },
}

#[derive(Debug, Subcommand)]
enum NetCommand {
    /// Download list of nets from the Jumperless
    #[command()]
    List {
        /// Write to file instead of stdout
        #[arg(long, short)]
        file: Option<String>,

        /// Output format
        #[arg(long, short, value_enum, default_value = "table")]
        output_format: OutputFormat,
    },

    /// Upload list of nets to the Jumperless
    #[command()]
    Send {
        /// Read from file instead of stdin
        #[arg(long, short)]
        file: Option<String>,
    }
}

#[derive(ValueEnum, Copy, Clone, PartialEq, Debug)]
enum OutputFormat {
    #[value()]
    Table,
    #[value()]
    Json,
}

#[derive(Debug, Subcommand)]
enum BridgeCommand {
    /// Download list of bridges from the Jumperless
    #[command()]
    List {
        /// Write to file instead of stdout
        #[arg(long, short)]
        file: Option<String>,
    },

    /// Upload new list of bridges to the Jumperless
    ///
    /// Either `--file` or `--bridges` must be specified (but not both).
    #[command()]
    Set {
        /// Bridge(s) to add, e.g. "GND-17" or "12-17,14-29"
        #[arg(long, short)]
        bridges: Option<String>,

        /// Read bridges from file
        #[arg(long, short)]
        file: Option<String>
    },

    /// Upload empty list of bridges to the jumperless
    #[command()]
    Clear,
}

fn main() -> anyhow::Result<()> {
    env_logger::init_from_env(Env::default().default_filter_or("info"));

    let args = Cli::parse();

    let mut device_manager = device_manager::DeviceManager::new(args.port);

    if let Command::ListPorts = args.command {
        let ports = device_manager.list_ports()?;
        let mut table = Table::new();
        table
            .load_preset(UTF8_FULL)
            .apply_modifier(UTF8_ROUND_CORNERS)
            .set_header(vec!["Port Name", "USB ID", "Role"]);
        for port in ports {
            let (vid, pid) = port.usb_id();
            table.add_row(vec![
                port.info.port_name,
                format!("{:04x}:{:04x}", vid, pid),
                format!("{:?}", port.role),
            ]);
        }
        println!("{table}");
        return Ok(());
    }

    if let Command::IdentifyPort = args.command {
        match device_manager.list_ports()?.into_iter().find(|port| port.role == PortRole::JumperlessPrimary) {
            Some(primary) => {
                println!("{}", primary.info.port_name);
            },
            None => return Err(anyhow::anyhow!("No matching ports")),
        }
        return Ok(())
    }
    
    if let Command::Server { listen } = args.command {
        server::start(device_manager, &listen).expect("Start server");
        return Ok(());
    }

    device_manager.with_device(|device| {
        match args.command {
            Command::SupplySwitchPos { pos } => {
                device.set_supply_switch(pos)?;
            },

            Command::Lightnet { name, color } => {
                device.lightnet(name, color.try_into()?)?;
            },

            Command::Net(net_command) => match net_command {
                NetCommand::List { file, output_format } => {
                    let mut output = file_or_stdout(file)?;
                    let netlist = device.netlist()?;
                    match output_format {
                        OutputFormat::Table => {
                            let mut table = Table::new();
                            table
                                .load_preset(UTF8_FULL)
                                .apply_modifier(UTF8_ROUND_CORNERS)
                                .set_header(vec![
                                    "Index",
                                    "Number",
                                    "Nodes",
                                    "Special",
                                    "Color",
                                    "Machine",
                                    "Name",
                                ]);
                            for net in netlist {
                                table.add_row(vec![
                                    net.index.to_string(),
                                    net.number.to_string(),
                                    net.nodes.iter().map(|n| n.to_string()).collect::<Vec<String>>().join(", "),
                                    net.special.to_string(),
                                    net.color.to_string(),
                                    net.machine.to_string(),
                                    net.name,
                                ]);
                            }
                            writeln!(&mut output, "{}", table)?;
                        }
                        OutputFormat::Json => {
                            serde_json::to_writer_pretty(&mut output, &netlist)?;
                            output.write_all(b"\n")?;
                        }
                    }
                }

                NetCommand::Send { file } => {
                    let mut input = file_or_stdin(file)?;
                    device.set_netlist(serde_json::from_reader(&mut input)?)?;
                }
            },

            Command::Bridge(bridge_command) => match bridge_command {
                BridgeCommand::List { file } => {
                    let mut output = file_or_stdout(file)?;
                    let bridgelist = device.bridgelist()?;
                    serde_json::to_writer_pretty(&mut output, &bridgelist)?;
                    output.write_all(b"\n")?;
                }
                BridgeCommand::Set { bridges, file } => {
                    let source = match (bridges, file) {
                        (None, None) => return Err(anyhow::anyhow!("Either `--bridges` or `--file` must be given")),
                        (Some(_), Some(_)) => return Err(anyhow::anyhow!("Cannot accept `--bridges` together with `--file`")),
                        (Some(bridges), _) => bridges,
                        (_, Some(file)) => std::fs::read_to_string(file)?,
                    };

                    let bridgelist = if source.starts_with("[") {
                        serde_json::from_str(&source).expect("parse bridgelist as JSON")
                    } else {
                        let (_, bridgelist) = nom::combinator::all_consuming(new_parser::bridges)(&source).expect("parse bridgelist");
                        bridgelist
                    };
                    device.set_bridgelist(bridgelist)?;
                },
                _ => todo!(),
            },
            _ => unreachable!(),
        }
        Ok(())
    })?;

    device_manager.close_device();

    Ok(())
}

fn file_or_stdout(file_path: Option<String>) -> std::io::Result<Box<dyn std::io::Write>> {
    Ok(
        match file_path {
            Some(file_path) => {
                info!("Writing output to {:?}", file_path);
                Box::new(File::create(file_path)?)
            },
            None => Box::new(std::io::stdout()),
        }
    )
}

fn file_or_stdin(file_path: Option<String>) -> std::io::Result<Box<dyn std::io::Read>> {
    Ok(
        match file_path {
            Some(file_path) => {
                info!("Reading input from {:?}", file_path);
                Box::new(File::open(file_path)?)
            },
            None => Box::new(std::io::stdin()),
        }
    )
}
