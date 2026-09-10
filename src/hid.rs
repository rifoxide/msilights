use crate::error::AppError;
use rusb::{Context, DeviceHandle, Direction, Recipient, RequestType, UsbContext};
use std::time::Duration;

pub const MSI_VENDOR_ID: u16 = 0x0db0;
pub const MSI_PRODUCT_ID: u16 = 0x0076;

#[allow(dead_code)]
pub trait FeatureReportTransport {
    fn send_feature_report(&mut self, report_id: u8, data: &[u8]) -> Result<usize, AppError>;
    fn read_feature_report(&mut self, report_id: u8, length: usize) -> Result<Vec<u8>, AppError>;
}

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

    pub fn read_feature_report(&mut self, report_id: u8, length: usize) -> rusb::Result<Vec<u8>> {
        let request_type =
            rusb::request_type(Direction::In, RequestType::Class, Recipient::Interface);
        let value = ((HID_FEATURE_REPORT_TYPE as u16) << 8) | u16::from(report_id);
        let mut data = vec![0; length];
        self.handle.read_control(
            request_type,
            0x01,
            value,
            u16::from(self.interface),
            &mut data,
            CONTROL_TRANSFER_TIMEOUT,
        )?;
        Ok(data)
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

impl FeatureReportTransport for HidTransport {
    fn send_feature_report(&mut self, report_id: u8, data: &[u8]) -> Result<usize, AppError> {
        self.send_feature_report(report_id, data)
            .map_err(AppError::from)
    }

    fn read_feature_report(&mut self, report_id: u8, length: usize) -> Result<Vec<u8>, AppError> {
        self.read_feature_report(report_id, length)
            .map_err(AppError::from)
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

#[allow(dead_code)]
#[derive(Debug, Default)]
pub struct RecordingTransport {
    reports: Vec<(u8, Vec<u8>)>,
    read_data: Option<Vec<u8>>,
    failure: Option<AppError>,
}

#[allow(dead_code)]
impl RecordingTransport {
    pub fn with_failure(failure: AppError) -> Self {
        Self {
            reports: Vec::new(),
            read_data: None,
            failure: Some(failure),
        }
    }

    pub fn with_read_data(read_data: Vec<u8>) -> Self {
        Self {
            reports: Vec::new(),
            read_data: Some(read_data),
            failure: None,
        }
    }

    pub fn reports(&self) -> &[(u8, Vec<u8>)] {
        &self.reports
    }
}

impl FeatureReportTransport for RecordingTransport {
    fn send_feature_report(&mut self, report_id: u8, data: &[u8]) -> Result<usize, AppError> {
        if let Some(failure) = &self.failure {
            return Err(match failure {
                AppError::Usb(error) => AppError::Usb(*error),
                AppError::DeviceNotFound {
                    vendor_id,
                    product_id,
                } => AppError::DeviceNotFound {
                    vendor_id: *vendor_id,
                    product_id: *product_id,
                },
            });
        }
        self.reports.push((report_id, data.to_vec()));
        Ok(data.len())
    }

    fn read_feature_report(&mut self, _report_id: u8, _length: usize) -> Result<Vec<u8>, AppError> {
        if let Some(failure) = &self.failure {
            return Err(match failure {
                AppError::Usb(error) => AppError::Usb(*error),
                AppError::DeviceNotFound {
                    vendor_id,
                    product_id,
                } => AppError::DeviceNotFound {
                    vendor_id: *vendor_id,
                    product_id: *product_id,
                },
            });
        }
        Ok(self.read_data.clone().unwrap_or_default())
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

    #[test]
    fn recording_transport_preserves_feature_reports() {
        let mut transport = RecordingTransport::default();
        assert_eq!(transport.send_feature_report(0x52, &[1, 2, 3]).unwrap(), 3);
        assert_eq!(transport.reports(), &[(0x52, vec![1, 2, 3])]);
    }

    #[test]
    fn recording_transport_propagates_failures() {
        let expected = AppError::DeviceNotFound {
            vendor_id: MSI_VENDOR_ID,
            product_id: MSI_PRODUCT_ID,
        };
        let expected_text = expected.to_string();
        let mut transport = RecordingTransport::with_failure(expected);
        assert_eq!(
            transport
                .send_feature_report(0x52, &[1, 2, 3])
                .unwrap_err()
                .to_string(),
            expected_text
        );
        assert!(transport.reports().is_empty());
    }
}
