use std::fmt;

#[derive(Debug)]
pub enum AppError {
    Usb(rusb::Error),
    DeviceNotFound { vendor_id: u16, product_id: u16 },
}

impl fmt::Display for AppError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Usb(error) => write!(f, "USB error: {error}"),
            Self::DeviceNotFound {
                vendor_id,
                product_id,
            } => write!(f, "device {vendor_id:#06x}:{product_id:#06x} was not found"),
        }
    }
}

impl std::error::Error for AppError {}

impl From<rusb::Error> for AppError {
    fn from(error: rusb::Error) -> Self {
        Self::Usb(error)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn formats_device_not_found() {
        let error = AppError::DeviceNotFound {
            vendor_id: 0x0db0,
            product_id: 0x0076,
        };
        assert_eq!(error.to_string(), "device 0x0db0:0x0076 was not found");
    }
}
