use anyhow::{Context, Result};
use tokio::io::AsyncWriteExt;
use tokio::time::{Duration, sleep};

/// Disney BLE manufacturer ID (little-endian: bytes 0x83 0x01 in packets).
const DISNEY_MANUFACTURER_ID: u16 = 0x0183;

/// Broadcast a MagicBand+ manufacturer data packet as a BLE advertisement.
///
/// The packet must start with the Disney manufacturer prefix (0x83 0x01).
/// Uses bluetoothctl to register a non-connectable broadcast advertisement
/// with fast intervals (~32-48ms) matching Disney park beacons.
///
/// Requires: bluez-utils (bluetoothctl), bluetooth group membership.
pub async fn broadcast(packet: &[u8], duration_secs: u64) -> Result<()> {
    let prefix_hi = (DISNEY_MANUFACTURER_ID >> 8) as u8;
    let prefix_lo = (DISNEY_MANUFACTURER_ID & 0xFF) as u8;
    if packet.len() < 2 || packet[0] != prefix_hi || packet[1] != prefix_lo {
        anyhow::bail!("packet must start with Disney manufacturer prefix 0x{DISNEY_MANUFACTURER_ID:04x}");
    }

    print_packet(packet);

    // Company ID (little-endian in packet), payload is the rest
    let payload = &packet[2..];
    let payload_hex: Vec<String> = payload.iter().map(|b| format!("0x{b:02x}")).collect();
    let mfg_arg = format!("0x{DISNEY_MANUFACTURER_ID:04x} {}", payload_hex.join(" "));

    let mut child = tokio::process::Command::new("bluetoothctl")
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .context("failed to run bluetoothctl — is bluez-utils installed?")?;

    let mut stdin = child.stdin.take().context("failed to open stdin")?;

    // Wait for bluetoothctl to connect to bluetoothd
    sleep(Duration::from_millis(500)).await;

    // Configure advertisement: non-connectable broadcast, fast interval, no name
    let setup = format!(
        "menu advertise\n\
         clear\n\
         manufacturer {mfg_arg}\n\
         name off\n\
         interval 32 48\n\
         back\n\
         advertise broadcast\n"
    );
    stdin.write_all(setup.as_bytes()).await?;
    stdin.flush().await?;

    // Wait for bluetoothd to register the advertisement
    sleep(Duration::from_millis(500)).await;

    eprintln!("Broadcasting for {duration_secs}s...");
    sleep(Duration::from_secs(duration_secs)).await;

    // Stop advertising and exit
    stdin.write_all(b"advertise off\n").await?;
    stdin.write_all(b"quit\n").await?;
    stdin.shutdown().await?;

    let output = child.wait_with_output().await?;
    let stdout = String::from_utf8_lossy(&output.stdout);
    if stdout.contains("Failed") {
        anyhow::bail!("bluetoothctl error: {stdout}");
    }

    eprintln!("Done.");
    Ok(())
}

fn print_packet(packet: &[u8]) {
    let hex: Vec<String> = packet.iter().map(|b| format!("{b:02x}")).collect();
    eprintln!("Packet: {}", hex.join(" "));
}
