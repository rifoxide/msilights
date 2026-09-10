#[allow(dead_code)]
mod protocol;

use rusb::{Context, DeviceHandle, Direction, Recipient, RequestType, UsbContext};

const HID_SET_REPORT: u8 = 0x09;

#[derive(Debug)]
#[allow(dead_code)]
enum ReportType {
    Input = 1,
    Output = 2,
    Feature = 3,
}

fn send_hid_set_report(
    handle: &mut DeviceHandle<Context>,
    interface: u8,
    report_type: ReportType,
    report_id: u8,
    data: &[u8],
) -> rusb::Result<usize> {
    let bm_request_type =
        rusb::request_type(Direction::Out, RequestType::Class, Recipient::Interface);

    let w_value = ((report_type as u16) << 8) | (report_id as u16);
    let w_index = interface as u16;

    println!("bm_request_type: {bm_request_type:#x}");
    println!("w_value: {w_value:#x}");
    println!("w_index: {w_index:#x}");

    handle.write_control(
        bm_request_type,
        HID_SET_REPORT,
        w_value,
        w_index,
        data,
        std::time::Duration::from_millis(1000),
    )
}

#[allow(dead_code)]
enum LightMode {
    Off = 0,
    Stedy = 1,
    Metro = 7,
}

fn main() -> rusb::Result<()> {
    let vid = 0x0DB0;
    let pid = 0x0076;
    let interface = 0;

    let context = Context::new()?;
    let devices = context.devices()?;

    for device in devices.iter() {
        let desc = device.device_descriptor()?;
        println!(
            "checking: {:#04x}::{:#04x}",
            desc.vendor_id(),
            desc.product_id()
        );
        if desc.vendor_id() == vid && desc.product_id() == pid {
            let mut handle = device.open()?;

            // Some devices already have a kernel driver attached
            let iface = 0; // usually 0, check your device
            if handle.kernel_driver_active(iface)? {
                println!("freeing device!");
                handle.detach_kernel_driver(iface)?;
            }

            println!("open success!");
            handle.claim_interface(interface)?;
            println!("device claimed!");

            let report_id = 82;
            let report_data = [
                0x52, 0x1, 0x35, 0xff, 0x23, 0x9, 0x35, 0xff, 0x23, 0x80, 0x0, 0x1a, 0xff, 0x0,
                0x0, 0x28, 0x0, 0xff, 0x0, 0x80, 0x0, 0x1a, 0xff, 0x0, 0x0, 0x28, 0x0, 0xff, 0x0,
                0x80, 0x0, //
                1,   // mode 0 off
                // 255, 180, 100, // rgb
                255, 54, 0, // rgb
                9, // brightness (9 - 41) speed (28-2a)
                0x00, 0x00, 0xff, // rgb 2
                0x80, 0x0, //
                32,  // cycle
                0x1, 0xff, 0x1e, 0x39, 0x9, 0xff, 0x1e, 0x39, 0x80, 0x0, 0x64, 0x0, 0x0, 0x0, 0x0,
                0x28, 0x0, 0x0, 0x0, 0x82, 0x4c, 0xa, 0x1, 0xff, 0x0, 0x0, 0x28, 0x0, 0xff, 0x0,
                0x80, 0x0, 0x0, 0x0, 0x0, 0x0, 0x28, 0x0, 0x0, 0x0, 0x81, 0x0, 0x1, 0xff, 0x0, 0x0,
                0x28, 0x0, 0xff, 0x0, 0x80, 0x0, 0x1, 0xff, 0x0, 0x0, 0x28, 0x0, 0xff, 0x0, 0x80,
                0x0, 0x1, 0xff, 0x0, 0x0, 0x28, 0x0, 0xff, 0x0, 0x80, 0x0, 0x1, 0xff, 0x0, 0x0,
                0x28, 0x0, 0xff, 0x0, 0x80, 0x0, 0x0, 0x0, 0x0, 0x0, 0x28, 0x0, 0x0, 0x0, 0x80,
                0x14, 0x1, 0x6d, 0xff, 0x8f, 0x9, 0x6d, 0xff, 0x8f, 0x80, 0x0, 0x1, 0xff, 0x0, 0x0,
                0x28, 0x0, 0xff, 0x0, 0x80, 0x5, 0x1, 0xff, 0x0, 0x0, 0x28, 0x0, 0xff, 0x0, 0x80,
                0x5, 0x1, 0xff, 0x0, 0x0, 0x28, 0x0, 0xff, 0x0, 0x80, 0x5, 0x1, 0xff, 0x0, 0x0,
                0x28, 0x0, 0xff, 0x0, 0x80, 0x5, 0x0,
            ];

            let bytes_written = send_hid_set_report(
                &mut handle,
                interface,
                ReportType::Feature,
                report_id,
                &report_data,
            )?;

            println!("Sent {} bytes via SET_REPORT", bytes_written);
            // let bytes_written = send_hid_set_report(
            //     &mut handle,
            //     interface,
            //     ReportType::Feature,
            //     report_id,
            //     &report_data,
            // )?;

            // println!("Sent {} bytes via SET_REPORT", bytes_written);

            handle.release_interface(interface)?;
        }
    }

    Ok(())
}
