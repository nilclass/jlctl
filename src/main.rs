use clap::{Parser, Subcommand};
use shadow_rs::shadow;

shadow!(build);

mod netlist;
mod parser;
mod device;

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

    #[command()]
    Server {
        #[arg(short, default_value = ":8080")]
        listen: String,
    },
}

#[derive(Debug, Subcommand)]
enum BridgeCommand {
    /// Get current list of bridges
    #[command()]
    Get,

    /// Add a new bridge
    #[command()]
    Add {
        #[arg()]
        bridge: String,
    },

    /// Remove given bridge
    #[command()]
    Remove {
        #[arg()]
        bridge: String,
    },

    /// Remove all bridges
    #[command()]
    Clear,
}

fn main() -> anyhow::Result<()> {
    let args = Cli::parse();
    let mut device = device::Device::new(args.port)?;

    match args.command {
        // "connect" => {
        //     node_file.write_to(&mut serialport).unwrap();
        //     serialport.flush().unwrap();
        // }
        Command::Netlist => {
            let netlist = device.netlist()?;
            println!("{netlist:#?}");
        }
        Command::Bridge(bridge_command) => {
            match bridge_command {
                BridgeCommand::Get => {
                    println!("{}", netlist::NodeFile::from(device.netlist()?));
                }
                BridgeCommand::Add { bridge } => {
                    let mut nodefile = netlist::NodeFile::from(device.netlist()?);
                    nodefile.add_connection(netlist::Connection::parse(&bridge)?);
                    device.send_nodefile(nodefile)?;
                }
                BridgeCommand::Remove { bridge } => {
                    let mut nodefile = netlist::NodeFile::from(device.netlist()?);
                    nodefile.remove_connection(netlist::Connection::parse(&bridge)?);
                    device.send_nodefile(nodefile)?;
                }
                BridgeCommand::Clear => {
                    device.send_nodefile(netlist::NodeFile::empty())?;
                }
            }
        }
        Command::Server { .. } => todo!(),
    }
    Ok(())
}
