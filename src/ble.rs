use anyhow::{Result, Context};
use tokio::time::{Duration, sleep};

/// Broadcast a MagicBand+ manufacturer data packet as a BLE advertisement.
///
/// Uses BlueZ hcitool to set raw advertising data and enable non-connectable
/// advertising for `duration_secs`, then disables it.
///
/// The packet must start with the 0x8301 Disney manufacturer prefix.
/// Requires: bluez-utils (hcitool), root or bluetooth group.
pub async fn broadcast(packet: &[u8], duration_secs: u64) -> Result<()> {
    print_packet(packet);

    // Build AD structure: flags + manufacturer specific data
    let mfg_len = packet.len() as u8 + 1; // +1 for the AD type byte
    let mut adv_data = Vec::with_capacity(31);
    adv_data.extend_from_slice(&[0x02, 0x01, 0x06]); // Flags: LE General + BR/EDR Not Supported
    adv_data.push(mfg_len);
    adv_data.push(0xFF); // AD type: Manufacturer Specific Data
    adv_data.extend_from_slice(packet);
    adv_data.resize(31, 0x00); // Pad to max legacy advertisement length

    let hex: String = adv_data.iter().map(|b| format!("{b:02X}")).collect();

    // Set advertising data (OGF=0x08 OCF=0x0008 = LE Set Advertising Data)
    run_hci(&["cmd", "0x08", "0x0008", &hex]).await?;

    // Set advertising parameters: ADV_NONCONN_IND, fast interval
    // OGF=0x08 OCF=0x0006 = LE Set Advertising Parameters
    run_hci(&[
        "cmd", "0x08", "0x0006",
        "2000",          // min interval 0x0020 (20ms) little-endian
        "4000",          // max interval 0x0040 (40ms) little-endian
        "03",            // ADV_NONCONN_IND (non-connectable undirected)
        "00",            // own address type: public
        "00",            // peer address type
        "000000000000",  // peer address (unused)
        "07",            // channel map: all three advertising channels
        "00",            // filter policy: allow any
    ])
    .await?;

    // Enable advertising (OGF=0x08 OCF=0x000A)
    run_hci(&["cmd", "0x08", "0x000A", "01"]).await?;
    eprintln!("Broadcasting for {duration_secs}s...");

    sleep(Duration::from_secs(duration_secs)).await;

    // Disable advertising
    run_hci(&["cmd", "0x08", "0x000A", "00"]).await?;
    eprintln!("Done.");

    Ok(())
}

async fn run_hci(args: &[&str]) -> Result<()> {
    let output = tokio::process::Command::new("hcitool")
        .args(args)
        .output()
        .await
        .context("failed to run hcitool — is bluez-utils installed?")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("hcitool failed: {stderr}");
    }
    Ok(())
}

fn print_packet(packet: &[u8]) {
    let hex: Vec<String> = packet.iter().map(|b| format!("{b:02x}")).collect();
    eprintln!("Packet: {}", hex.join(" "));
}
