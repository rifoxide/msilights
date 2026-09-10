use crate::error::AppError;
use rusb::{Context, DeviceHandle, Direction, Recipient, RequestType, UsbContext};
use std::time::Duration;

pub const MSI_VENDOR_ID: u16 = 0x0db0;
pub const MSI_PRODUCT_ID: u16 = 0x0076;

const HID_SET_REPORT: u8 = 0x09;
const HID_FEATURE_REPORT_TYPE: u8 = 3;
const CONTROL_TRANSFER_TIMEOUT: Duration = Duration::from_millis(1000);

pub fn is_matching_device(vendor_id: u16, product_id: u16) -> bool {
    vendor_id == MSI_VENDOR_ID && product_id == MSI_PRODUCT_ID
}

pub fn open_matching(context: &Context, interface: u8) -> Result<HidTransport, AppError> {
    let devices = context.devices()?;
    for device in devices.iter() {
        let descriptor = device.device_descriptor()?;
        println!(
            "checking: {:#04x}::{:#04x}",
            descriptor.vendor_id(),
            descriptor.product_id()
        );
        if is_matching_device(descriptor.vendor_id(), descriptor.product_id()) {
            return Ok(HidTransport::open(device, interface)?);
        }
    }

    Err(AppError::DeviceNotFound {
        vendor_id: MSI_VENDOR_ID,
        product_id: MSI_PRODUCT_ID,
    })
}

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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn matches_only_the_supported_msi_device() {
        assert!(is_matching_device(MSI_VENDOR_ID, MSI_PRODUCT_ID));
        assert!(!is_matching_device(MSI_VENDOR_ID, MSI_PRODUCT_ID + 1));
        assert!(!is_matching_device(MSI_VENDOR_ID + 1, MSI_PRODUCT_ID));
    }
}
