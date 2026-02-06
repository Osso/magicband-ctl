use anyhow::Result;
use clap::{Parser, Subcommand};

mod ble;
mod protocol;

#[derive(Parser)]
#[command(name = "magicband-ctl", about = "Control MagicBand+ LEDs and haptics over BLE")]
struct Cli {
    /// Broadcast duration in seconds
    #[arg(short, long, default_value_t = 3)]
    duration: u64,

    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Ping nearby MagicBands (triggers default response)
    Ping,

    /// Light up with a single color
    Color {
        /// Color name (red, blue, green, purple, cyan, pink, orange, white, lime, random, off)
        name: String,

        /// Vibration intensity (0-15, 0=none)
        #[arg(short, long, default_value_t = 0)]
        vib: u8,
    },

    /// Rainbow cycle through 5 colors
    Rainbow {
        /// Vibration intensity (0-15)
        #[arg(short, long, default_value_t = 0)]
        vib: u8,
    },

    /// Dual color (inner/outer LEDs)
    Dual {
        /// Inner color name
        inner: String,
        /// Outer color name
        outer: String,

        /// Vibration intensity (0-15)
        #[arg(short, long, default_value_t = 0)]
        vib: u8,
    },

    /// List available color names
    Colors,
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    if let Command::Colors = &cli.command {
        protocol::print_colors();
        return Ok(());
    }

    let packet = match &cli.command {
        Command::Ping => protocol::ping(),
        Command::Color { name, vib } => {
            let color = protocol::parse_color(name)?;
            protocol::single_color(color, *vib)
        }
        Command::Rainbow { vib } => protocol::rainbow_default(*vib),
        Command::Dual { inner, outer, vib } => {
            let c1 = protocol::parse_color(inner)?;
            let c2 = protocol::parse_color(outer)?;
            protocol::dual_color(c1, c2, *vib)
        }
        Command::Colors => unreachable!(),
    };

    ble::broadcast(&packet, cli.duration).await
}
