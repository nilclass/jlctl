use anyhow::Context;
use clap::{Parser, Subcommand};
use comfy_table::{modifiers::UTF8_ROUND_CORNERS, presets::UTF8_FULL, Table};
use env_logger::Env;
use shadow_rs::shadow;

shadow!(build);

mod device;
mod device_manager;
mod netlist;
mod parser;
mod server;

#[derive(Debug, Parser)]
#[command(about = "CLI for the jumperless breadboard", version = build::CLAP_LONG_VERSION)]
struct Cli {
    /// Serial port where the Jumperless is connected. If omitted, the port is detected dynamically.
    #[arg(long, short)]
    port: Option<String>,

    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    /// List serial ports
    #[command()]
    ListPorts,

    /// Print current netlist
    #[command()]
    Netlist,

    /// Interact with bridges
    #[command(subcommand)]
    Bridge(BridgeCommand),

    /// Start HTTP server
    #[command()]
    Server {
        #[arg(long, short, default_value = "localhost:8080")]
        listen: String,
    },
}

#[derive(Debug, Subcommand)]
enum BridgeCommand {
    /// Get current list of bridges
    #[command()]
    Get,

    /// Add new bridge(s)
    #[command()]
    Add {
        /// Bridge(s) to add, e.g. "GND-17" or "12-17,14-29"
        #[arg()]
        bridges: String,
    },

    /// Remove given bridge(s)
    #[command()]
    Remove {
        /// Bridge(s) to remove, e.g. "GND-17" or "12-17,14-29"
        #[arg()]
        bridges: String,
    },

    /// Remove all bridges
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
                format!("{:4x}:{:4x}", vid, pid),
                format!("{:?}", port.role),
            ]);
        }
        println!("{table}");
        return Ok(());
    }

    if let Command::Server { listen } = args.command {
        server::start(device_manager, &listen).expect("Start server");
        return Ok(());
    }

    device_manager.with_device(|device| {
        match args.command {
            Command::Netlist => {
                let netlist = device.netlist()?;
                serde_json::to_writer_pretty(std::io::stdout(), &netlist)?;
                println!();
            }
            Command::Bridge(bridge_command) => match bridge_command {
                BridgeCommand::Get => {
                    println!("{}", netlist::NodeFile::from(device.netlist()?));
                }
                BridgeCommand::Add { bridges } => {
                    let mut nodefile = netlist::NodeFile::from(device.netlist()?);
                    let parsed = netlist::NodeFile::parse(&bridges)
                        .with_context(|| "Parsing bridges from argument")?;
                    nodefile.add_from(parsed);
                    device.send_nodefile(&nodefile)?;
                }
                BridgeCommand::Remove { bridges } => {
                    let mut nodefile = netlist::NodeFile::from(device.netlist()?);
                    let parsed = netlist::NodeFile::parse(&bridges)
                        .with_context(|| "Parsing bridges from argument")?;
                    nodefile.remove_from(parsed);
                    device.send_nodefile(&nodefile)?;
                }
                BridgeCommand::Clear => {
                    device.clear_nodefile()?;
                }
            },
            _ => unreachable!(),
        }
        Ok(())
    })
}
