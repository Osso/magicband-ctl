#![cfg_attr(coverage_nightly, feature(coverage_attribute))]

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
#[cfg_attr(coverage_nightly, coverage(off))]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    if let Command::Colors = &cli.command {
        protocol::print_colors();
        return Ok(());
    }

    let packet = build_packet(&cli.command)?;

    ble::broadcast(&packet, cli.duration).await
}

fn build_packet(command: &Command) -> Result<Vec<u8>> {
    let packet = match command {
        Command::Ping => protocol::ping(),
        Command::Color { name, vib } => {
            let color = protocol::parse_color(name)?;
            protocol::single_color(color, *vib)
        }
        Command::Dual { inner, outer, vib } => {
            let inner_color = protocol::parse_color(inner)?;
            let outer_color = protocol::parse_color(outer)?;
            protocol::dual_color(inner_color, outer_color, *vib)
        }
        Command::FiveColor {
            center,
            top_right,
            bottom_right,
            bottom_left,
            top_left,
            vib,
        } => build_five_color_packet(center, top_right, bottom_right, bottom_left, top_left, *vib)?,
        Command::Circle { vib } => protocol::circle(*vib),
        Command::Crossfade { c1, c2, vib } => {
            let first = protocol::parse_color(c1)?;
            let second = protocol::parse_color(c2)?;
            protocol::crossfade(first, second, *vib)
        }
        Command::Colors => unreachable!(),
    };
    Ok(packet)
}

fn build_five_color_packet(
    center: &str,
    top_right: &str,
    bottom_right: &str,
    bottom_left: &str,
    top_left: &str,
    vib: u8,
) -> Result<Vec<u8>> {
    Ok(protocol::five_color(
        protocol::parse_color(center)?,
        protocol::parse_color(top_right)?,
        protocol::parse_color(bottom_right)?,
        protocol::parse_color(bottom_left)?,
        protocol::parse_color(top_left)?,
        vib,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::CommandFactory;

    #[test]
    fn build_packet_maps_command_variants_to_protocol_packets() {
        assert_eq!(
            build_packet(&Command::Ping).expect("ping"),
            protocol::ping()
        );
        assert_eq!(
            build_packet(&Command::Color {
                name: "red".to_string(),
                vib: 1,
            })
            .expect("color"),
            protocol::single_color(protocol::Color::Red, 1)
        );
        assert_eq!(
            build_packet(&Command::Dual {
                inner: "red".to_string(),
                outer: "blue".to_string(),
                vib: 2,
            })
            .expect("dual"),
            protocol::dual_color(protocol::Color::Red, protocol::Color::Blue, 2)
        );
        assert_eq!(
            build_packet(&Command::Circle { vib: 3 }).expect("circle"),
            protocol::circle(3)
        );
    }

    #[test]
    fn build_packet_handles_five_color_and_crossfade() {
        assert_eq!(
            build_packet(&Command::FiveColor {
                center: "red".to_string(),
                top_right: "blue".to_string(),
                bottom_right: "green".to_string(),
                bottom_left: "white".to_string(),
                top_left: "off".to_string(),
                vib: 4,
            })
            .expect("five color"),
            protocol::five_color(
                protocol::Color::Red,
                protocol::Color::Blue,
                protocol::Color::Green,
                protocol::Color::White,
                protocol::Color::Off,
                4,
            )
        );
        assert_eq!(
            build_packet(&Command::Crossfade {
                c1: "red".to_string(),
                c2: "blue".to_string(),
                vib: 5,
            })
            .expect("crossfade"),
            protocol::crossfade(protocol::Color::Red, protocol::Color::Blue, 5)
        );
    }

    #[test]
    fn build_packet_reports_unknown_color() {
        assert!(
            build_packet(&Command::Color {
                name: "not-a-color".to_string(),
                vib: 0,
            })
            .is_err()
        );
    }

    #[test]
    fn clap_definition_is_valid() {
        Cli::command().debug_assert();
    }
}
