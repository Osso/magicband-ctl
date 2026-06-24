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
#[cfg_attr(coverage_nightly, coverage(off))]
pub async fn broadcast(packet: &[u8], duration_secs: u64) -> Result<()> {
    validate_disney_prefix(packet)?;
    print_packet(packet);
    let mfg_arg = build_manufacturer_arg(packet);
    let mut child = spawn_bluetoothctl()?;

    let mut stdin = child.stdin.take().context("failed to open stdin")?;
    configure_advertisement(&mut stdin, &mfg_arg).await?;
    eprintln!("Broadcasting for {duration_secs}s...");
    sleep(Duration::from_secs(duration_secs)).await;

    stop_advertising(&mut stdin).await?;

    let output = child.wait_with_output().await?;
    let stdout = String::from_utf8_lossy(&output.stdout);
    if stdout.contains("Failed") {
        anyhow::bail!("bluetoothctl error: {stdout}");
    }

    eprintln!("Done.");
    Ok(())
}

#[cfg_attr(coverage_nightly, coverage(off))]
fn print_packet(packet: &[u8]) {
    let hex: Vec<String> = packet.iter().map(|b| format!("{b:02x}")).collect();
    eprintln!("Packet: {}", hex.join(" "));
}

fn validate_disney_prefix(packet: &[u8]) -> Result<()> {
    let prefix_hi = (DISNEY_MANUFACTURER_ID >> 8) as u8;
    let prefix_lo = (DISNEY_MANUFACTURER_ID & 0xFF) as u8;
    if packet.len() < 2 || packet[0] != prefix_hi || packet[1] != prefix_lo {
        anyhow::bail!(
            "packet must start with Disney manufacturer prefix 0x{DISNEY_MANUFACTURER_ID:04x}"
        );
    }
    Ok(())
}

fn build_manufacturer_arg(packet: &[u8]) -> String {
    let payload_hex: Vec<String> = packet[2..].iter().map(|b| format!("0x{b:02x}")).collect();
    format!("0x{DISNEY_MANUFACTURER_ID:04x} {}", payload_hex.join(" "))
}

#[cfg_attr(coverage_nightly, coverage(off))]
fn spawn_bluetoothctl() -> Result<tokio::process::Child> {
    tokio::process::Command::new("bluetoothctl")
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .context("failed to run bluetoothctl — is bluez-utils installed?")
}

#[cfg_attr(coverage_nightly, coverage(off))]
async fn configure_advertisement(
    stdin: &mut tokio::process::ChildStdin,
    mfg_arg: &str,
) -> Result<()> {
    sleep(Duration::from_millis(500)).await;
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
    sleep(Duration::from_millis(500)).await;
    Ok(())
}

#[cfg_attr(coverage_nightly, coverage(off))]
async fn stop_advertising(stdin: &mut tokio::process::ChildStdin) -> Result<()> {
    stdin.write_all(b"advertise off\n").await?;
    stdin.write_all(b"quit\n").await?;
    stdin.shutdown().await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    const MANUFACTURER_ARG_PREFIX: &str = "0x0183";

    #[test]
    fn validate_disney_prefix_accepts_disney_packets() {
        assert!(validate_disney_prefix(&[0x01, 0x83]).is_ok());
        assert!(validate_disney_prefix(&[0x01, 0x83, 0xaa]).is_ok());
    }

    #[test]
    fn validate_disney_prefix_rejects_short_or_wrong_packets() {
        assert!(validate_disney_prefix(&[]).is_err());
        assert!(validate_disney_prefix(&[0x01]).is_err());
        assert!(validate_disney_prefix(&[0x83, 0x01]).is_err());
    }

    #[test]
    fn build_manufacturer_arg_uses_company_id_and_payload_bytes() {
        assert_eq!(
            build_manufacturer_arg(&[0x01, 0x83, 0xaa, 0x0f]),
            format!("{MANUFACTURER_ARG_PREFIX} 0xaa 0x0f")
        );
        assert_eq!(
            build_manufacturer_arg(&[0x01, 0x83]),
            format!("{MANUFACTURER_ARG_PREFIX} ")
        );
    }
}
