use crate::error::AppError;
use crate::hid::FeatureReportTransport;
use crate::protocol::{
    Color, FEATURE_PACKET_LEN, FEATURE_REPORT_ID, FeaturePacket185, MsiBrightness, MsiMode,
    MsiSpeed, ProtocolError, ZoneData,
};
use std::fmt;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Zone {
    JRgb1,
    JPipe1,
    JPipe2,
    JRainbow1,
    JRainbow2,
    JCorsairOuter,
    OnBoard(u8),
    JRgb2,
}

#[derive(Debug)]
pub enum ControllerError {
    InvalidZone(Zone),
    InvalidOnBoardIndex(u8),
    Protocol(ProtocolError),
    Transport(AppError),
    ShortTransfer { expected: usize, actual: usize },
}

impl fmt::Display for ControllerError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidZone(zone) => write!(f, "unsupported zone: {zone:?}"),
            Self::InvalidOnBoardIndex(index) => write!(f, "invalid onboard LED index: {index}"),
            Self::Protocol(error) => write!(f, "protocol error: {error}"),
            Self::Transport(error) => write!(f, "transport error: {error}"),
            Self::ShortTransfer { expected, actual } => {
                write!(f, "expected {expected} bytes written, got {actual}")
            }
        }
    }
}

impl std::error::Error for ControllerError {}

impl From<ProtocolError> for ControllerError {
    fn from(error: ProtocolError) -> Self {
        Self::Protocol(error)
    }
}

impl From<AppError> for ControllerError {
    fn from(error: AppError) -> Self {
        Self::Transport(error)
    }
}

pub struct MsiController<T> {
    transport: T,
    packet: FeaturePacket185,
}

impl<T: FeatureReportTransport> MsiController<T> {
    pub fn new(transport: T) -> Self {
        Self {
            transport,
            packet: FeaturePacket185::default(),
        }
    }

    pub fn packet(&self) -> &FeaturePacket185 {
        &self.packet
    }

    pub fn set_colors(
        &mut self,
        zone: Zone,
        primary: Color,
        secondary: Color,
    ) -> Result<(), ControllerError> {
        let zone = self.zone_mut(zone)?;
        zone.color = primary;
        zone.color2 = secondary;
        Ok(())
    }

    pub fn set_settings(
        &mut self,
        zone: Zone,
        effect: MsiMode,
        speed: MsiSpeed,
        brightness: MsiBrightness,
    ) -> Result<(), ControllerError> {
        let zone = self.zone_mut(zone)?;
        zone.effect = effect.encode();
        zone.set_speed(speed);
        zone.set_brightness(brightness);
        Ok(())
    }

    pub fn set_save(&mut self, save: bool) {
        self.packet.save_data = u8::from(save);
    }

    pub fn send_current(&mut self) -> Result<(), ControllerError> {
        let bytes = self.packet.encode();
        let written = self
            .transport
            .send_feature_report(FEATURE_REPORT_ID, &bytes)?;
        if written != FEATURE_PACKET_LEN {
            return Err(ControllerError::ShortTransfer {
                expected: FEATURE_PACKET_LEN,
                actual: written,
            });
        }
        Ok(())
    }

    fn zone_mut(&mut self, zone: Zone) -> Result<&mut ZoneData, ControllerError> {
        match zone {
            Zone::JRgb1 => Ok(&mut self.packet.j_rgb_1),
            Zone::JPipe1 => Ok(&mut self.packet.j_pipe_1),
            Zone::JPipe2 => Ok(&mut self.packet.j_pipe_2),
            Zone::JRainbow1 => Ok(&mut self.packet.j_rainbow_1.zone),
            Zone::JRainbow2 => Ok(&mut self.packet.j_rainbow_2.zone),
            Zone::JCorsairOuter => Ok(&mut self.packet.j_corsair_outerll120),
            Zone::JRgb2 => Ok(&mut self.packet.j_rgb_2),
            Zone::OnBoard(index) => self
                .packet
                .on_board_led
                .get_mut(index as usize)
                .ok_or(ControllerError::InvalidOnBoardIndex(index)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hid::RecordingTransport;

    #[test]
    fn controller_updates_packet_fields() {
        let mut controller = MsiController::new(RecordingTransport::default());
        controller
            .set_colors(Zone::JRgb1, Color::new(1, 2, 3), Color::new(4, 5, 6))
            .unwrap();
        controller
            .set_settings(
                Zone::JRgb1,
                MsiMode::Static,
                MsiSpeed::High,
                MsiBrightness::Level70,
            )
            .unwrap();
        controller.set_save(true);

        let zone = &controller.packet().j_rgb_1;
        assert_eq!(zone.color, Color::new(1, 2, 3));
        assert_eq!(zone.color2, Color::new(4, 5, 6));
        assert_eq!(zone.effect, MsiMode::Static.encode());
        assert_eq!(zone.speed(), Ok(MsiSpeed::High));
        assert_eq!(zone.brightness(), Ok(MsiBrightness::Level70));
        assert_eq!(controller.packet().save_data, 1);
    }

    #[test]
    fn controller_sends_complete_feature_packet() {
        let transport = RecordingTransport::default();
        let mut controller = MsiController::new(transport);
        controller.send_current().unwrap();
    }

    #[test]
    fn controller_rejects_invalid_onboard_zone() {
        let mut controller = MsiController::new(RecordingTransport::default());
        assert!(matches!(
            controller.set_colors(Zone::OnBoard(10), Color::default(), Color::default()),
            Err(ControllerError::InvalidOnBoardIndex(10))
        ));
    }
}
