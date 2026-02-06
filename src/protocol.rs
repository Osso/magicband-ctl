use anyhow::{Result, bail};

/// Disney manufacturer prefix for all MagicBand+ BLE packets.
const MFG_PREFIX: [u8; 2] = [0x83, 0x01];

/// 5-bit color palette indices.
#[derive(Debug, Clone, Copy)]
#[repr(u8)]
pub enum Color {
    Cyan = 0x00,
    Purple = 0x01,
    Blue = 0x02,
    MidnightBlue = 0x03,
    BrightPurple = 0x05,
    Lavender = 0x06,
    Pink = 0x08,
    YellowOrange = 0x0F,
    OffYellow = 0x10,
    Lime = 0x12,
    Orange = 0x13,
    RedOrange = 0x14,
    Red = 0x15,
    Green = 0x19,
    LimeGreen = 0x1A,
    White = 0x1B,
    Off = 0x1D,
    Random = 0x1F,
}

const COLORS: &[(Color, &str)] = &[
    (Color::Red, "red"),
    (Color::Blue, "blue"),
    (Color::Green, "green"),
    (Color::Purple, "purple"),
    (Color::BrightPurple, "bright-purple"),
    (Color::Cyan, "cyan"),
    (Color::Pink, "pink"),
    (Color::Orange, "orange"),
    (Color::RedOrange, "red-orange"),
    (Color::YellowOrange, "yellow-orange"),
    (Color::OffYellow, "off-yellow"),
    (Color::White, "white"),
    (Color::Lime, "lime"),
    (Color::LimeGreen, "lime-green"),
    (Color::Lavender, "lavender"),
    (Color::MidnightBlue, "midnight-blue"),
    (Color::Off, "off"),
    (Color::Random, "random"),
];

pub fn parse_color(name: &str) -> Result<Color> {
    let lower = name.to_lowercase();
    for (color, label) in COLORS {
        if *label == lower {
            return Ok(*color);
        }
    }
    bail!(
        "unknown color '{}'. Run `magicband-ctl colors` to see available colors",
        name
    );
}

pub fn print_colors() {
    println!("Available colors:");
    for (color, label) in COLORS {
        println!("  {label:<16} (0x{:02X})", *color as u8);
    }
}

fn vib_byte(vib: u8) -> u8 {
    0xB0 | (vib & 0x0F)
}

fn color_byte_single(color: Color) -> u8 {
    0xE0 | (color as u8 & 0x1F)
}

fn color_byte_multi(color: Color) -> u8 {
    0xA0 | (color as u8 & 0x1F)
}

fn color_byte_dual(color: Color) -> u8 {
    0x80 | (color as u8 & 0x1F)
}

/// CC03 ping — triggers default band response.
pub fn ping() -> Vec<u8> {
    let mut p = MFG_PREFIX.to_vec();
    p.extend_from_slice(&[0xCC, 0x03, 0x00, 0x00, 0x00]);
    p
}

/// E9 05 — single color from palette.
pub fn single_color(color: Color, vib: u8) -> Vec<u8> {
    let mut p = MFG_PREFIX.to_vec();
    // timing 0x2E = ~12s on-time, mask 0x0E = all LEDs
    p.extend_from_slice(&[
        0xE9, 0x05, 0x00, 0x2E, 0x0E,
        color_byte_single(color),
        vib_byte(vib),
    ]);
    p
}

/// E9 09 — 5-color rainbow cycle.
pub fn rainbow_default(vib: u8) -> Vec<u8> {
    rainbow(
        Color::YellowOrange,
        Color::Red,
        Color::Green,
        Color::Blue,
        Color::Purple,
        vib,
    )
}

pub fn rainbow(c1: Color, c2: Color, c3: Color, c4: Color, c5: Color, vib: u8) -> Vec<u8> {
    let mut p = MFG_PREFIX.to_vec();
    p.extend_from_slice(&[
        0xE9, 0x09, 0x00, 0x2E, 0x0F,
        color_byte_multi(c1),
        color_byte_multi(c2),
        color_byte_multi(c3),
        color_byte_multi(c4),
        color_byte_multi(c5),
        vib_byte(vib),
    ]);
    p
}

/// E9 06 — dual color (inner + outer LEDs).
pub fn dual_color(inner: Color, outer: Color, vib: u8) -> Vec<u8> {
    let mut p = MFG_PREFIX.to_vec();
    p.extend_from_slice(&[
        0xE9, 0x06, 0x00, 0x22, 0x0F,
        color_byte_dual(inner),
        color_byte_dual(outer),
        vib_byte(vib),
    ]);
    p
}
