use anyhow::Result;
use clap::{Parser, Subcommand};

mod ble;
mod protocol;

#[derive(Parser)]
#[command(
    name = "magicband-ctl",
    about = "Control MagicBand+ LEDs and haptics over BLE"
)]
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

    /// Light up with a single color (all LEDs)
    Color {
        /// Color name
        name: String,

        /// Vibration intensity (0-15, 0=none)
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

    /// Set 5 individual LED colors (static, center/TR/BR/BL/TL)
    FiveColor {
        /// Center LED color
        center: String,
        /// Top-right LED color
        top_right: String,
        /// Bottom-right LED color
        bottom_right: String,
        /// Bottom-left LED color
        bottom_left: String,
        /// Top-left LED color
        top_left: String,

        /// Vibration intensity (0-15)
        #[arg(short, long, default_value_t = 0)]
        vib: u8,
    },

    /// Rotating circle animation
    Circle {
        /// Vibration intensity (0-15)
        #[arg(short, long, default_value_t = 0)]
        vib: u8,
    },

    /// Crossfade animation between two colors
    Crossfade {
        /// First color
        c1: String,
        /// Second color
        c2: String,

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
        Command::Dual { inner, outer, vib } => {
            let c1 = protocol::parse_color(inner)?;
            let c2 = protocol::parse_color(outer)?;
            protocol::dual_color(c1, c2, *vib)
        }
        Command::FiveColor {
            center,
            top_right,
            bottom_right,
            bottom_left,
            top_left,
            vib,
        } => protocol::five_color(
            protocol::parse_color(center)?,
            protocol::parse_color(top_right)?,
            protocol::parse_color(bottom_right)?,
            protocol::parse_color(bottom_left)?,
            protocol::parse_color(top_left)?,
            *vib,
        ),
        Command::Circle { vib } => protocol::circle(*vib),
        Command::Crossfade { c1, c2, vib } => {
            protocol::crossfade(protocol::parse_color(c1)?, protocol::parse_color(c2)?, *vib)
        }
        Command::Colors => unreachable!(),
    };

    ble::broadcast(&packet, cli.duration).await
}
