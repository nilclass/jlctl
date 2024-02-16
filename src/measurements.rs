use anyhow::Context;
use rusb::{Direction, TransferType};
use std::time::Duration;

pub fn dump_measurements() -> anyhow::Result<()> {
    let device = find_device().ok_or(anyhow::anyhow!("No matching USB device found"))?;
    let mut vendor_interface = None;
    let mut int_endpoint = None;
    for interface in device
        .active_config_descriptor()
        .with_context(|| "active config descriptor")?
        .interfaces()
    {
        for iface_desc in interface.descriptors() {
            if iface_desc.class_code() == 0xFF {
                vendor_interface = Some(iface_desc.interface_number());
                for ep_desc in iface_desc.endpoint_descriptors() {
                    if ep_desc.transfer_type() == TransferType::Interrupt
                        && ep_desc.direction() == Direction::In
                    {
                        int_endpoint = Some(ep_desc.address());
                    }
                }
            }
        }
    }
    let vendor_interface = vendor_interface.ok_or(anyhow::anyhow!(
        "Failed to identify vendor interface number"
    ))?;
    let int_endpoint = int_endpoint.ok_or(anyhow::anyhow!(
        "Failed to identify correct interrupt endpoint for vendor interface"
    ))?;
    let mut handle = device.open()?;
    handle
        .claim_interface(vendor_interface)
        .with_context(|| "claim interface")?;
    let mut buf = [0u8; 8];
    loop {
        handle
            .read_interrupt(int_endpoint, &mut buf, Duration::from_millis(300))
            .with_context(|| "read interrupt")?;
        for i in 0..4 {
            let bytes = [buf[i * 2], buf[i * 2 + 1]];
            print!("{}\t", u16::from_le_bytes(bytes));
        }
        println!();
    }
}

fn find_device() -> Option<rusb::Device<rusb::GlobalContext>> {
    for device in rusb::devices().unwrap().iter() {
        let device_desc = device.device_descriptor().unwrap();

        let vid = device_desc.vendor_id();
        let pid = device_desc.product_id();

        if (vid, pid) == (0x1d50, 0xacab) {
            return Some(device);
        }
    }
    None
}
