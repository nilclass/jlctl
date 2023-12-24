use clap::{Parser, Subcommand};
use shadow_rs::shadow;
use anyhow::Context;

shadow!(build);

mod netlist;
mod parser;
mod device;
mod server;

#[derive(Debug, Parser)]
#[command(about = "CLI for the jumperless breadboard", version = build::CLAP_LONG_VERSION)]
struct Cli {
    #[arg(short, default_value = "/dev/ttyACM0")]
    port: String,
    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
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
    let args = Cli::parse();
    let mut device = device::Device::new(args.port)?;

    match args.command {
        Command::Netlist => {
            let netlist = device.netlist()?;
            serde_json::to_writer_pretty(std::io::stdout(), &netlist)?;
            println!("");
        }
        Command::Bridge(bridge_command) => {
            match bridge_command {
                BridgeCommand::Get => {
                    println!("{}", netlist::NodeFile::from(device.netlist()?));
                }
                BridgeCommand::Add { bridges } => {
                    let mut nodefile = netlist::NodeFile::from(device.netlist()?);
                    let parsed = netlist::NodeFile::parse(&bridges)
                        .with_context(|| format!("Parsing bridges from argument"))?;
                    nodefile.add_from(parsed);
                    device.send_nodefile(&nodefile)?;
                }
                BridgeCommand::Remove { bridges } => {
                    let mut nodefile = netlist::NodeFile::from(device.netlist()?);
                    let parsed = netlist::NodeFile::parse(&bridges)
                        .with_context(|| format!("Parsing bridges from argument"))?;
                    nodefile.remove_from(parsed);
                    device.send_nodefile(&nodefile)?;
                }
                BridgeCommand::Clear => {
                    device.clear_nodefile()?;
                }
            }
        }
        Command::Server { listen } => {
            server::start(device, &listen)?;
        },
    }
    Ok(())
}
