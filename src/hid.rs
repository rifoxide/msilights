use rusb::{Context, DeviceHandle, Direction, Recipient, RequestType};
use std::time::Duration;

const HID_SET_REPORT: u8 = 0x09;
const HID_FEATURE_REPORT_TYPE: u8 = 3;
const CONTROL_TRANSFER_TIMEOUT: Duration = Duration::from_millis(1000);

pub struct HidTransport {
    handle: DeviceHandle<Context>,
    interface: u8,
    detached_kernel_driver: bool,
}

impl HidTransport {
    pub fn open(device: rusb::Device<Context>, interface: u8) -> rusb::Result<Self> {
        let handle = device.open()?;
        let detached_kernel_driver = if handle.kernel_driver_active(interface)? {
            handle.detach_kernel_driver(interface)?;
            true
        } else {
            false
        };

        handle.claim_interface(interface)?;

        Ok(Self {
            handle,
            interface,
            detached_kernel_driver,
        })
    }

    pub fn send_feature_report(&mut self, report_id: u8, data: &[u8]) -> rusb::Result<usize> {
        let request_type =
            rusb::request_type(Direction::Out, RequestType::Class, Recipient::Interface);
        let value = ((HID_FEATURE_REPORT_TYPE as u16) << 8) | u16::from(report_id);

        println!("bm_request_type: {request_type:#x}");
        println!("w_value: {value:#x}");
        println!("w_index: {:#x}", self.interface);

        self.handle.write_control(
            request_type,
            HID_SET_REPORT,
            value,
            u16::from(self.interface),
            data,
            CONTROL_TRANSFER_TIMEOUT,
        )
    }
}

impl Drop for HidTransport {
    fn drop(&mut self) {
        let _ = self.handle.release_interface(self.interface);
        if self.detached_kernel_driver {
            let _ = self.handle.attach_kernel_driver(self.interface);
        }
    }
}
