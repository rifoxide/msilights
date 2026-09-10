use crate::boards::{BoardCapabilities, MsiZone};
use crate::error::AppError;
use crate::hid::FeatureReportTransport;
use crate::protocol::{
    Color, FEATURE_PACKET_LEN, FEATURE_REPORT_ID, FeaturePacket185, MsiBrightness, MsiMode,
    MsiSpeed, ProtocolError, SyncSetting, ZoneData,
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
    capabilities: BoardCapabilities,
}

impl Zone {
    fn to_msi_zone(self) -> MsiZone {
        match self {
            Self::JRgb1 => MsiZone::JRgb1,
            Self::JRgb2 => MsiZone::JRgb2,
            Self::JPipe1 => MsiZone::JPipe1,
            Self::JPipe2 => MsiZone::JPipe2,
            Self::JRainbow1 => MsiZone::JRainbow1,
            Self::JRainbow2 => MsiZone::JRainbow2,
            Self::JCorsairOuter => MsiZone::JCorsairOuterLl120,
            Self::OnBoard(_) => MsiZone::OnBoardLed0,
        }
    }
}

impl<T: FeatureReportTransport> MsiController<T> {
    pub fn new(transport: T) -> Self {
        Self::with_capabilities(transport, crate::boards::COMMON_185_CAPABILITIES)
    }

    pub fn with_capabilities(transport: T, capabilities: BoardCapabilities) -> Self {
        Self {
            transport,
            packet: FeaturePacket185::default(),
            capabilities,
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

    pub fn set_jrgb_sync(&mut self, zone: Zone, enabled: bool) -> Result<(), ControllerError> {
        self.zone_mut(zone)?.set_jrgb_sync_enabled(enabled);
        Ok(())
    }

    pub fn set_color_sync(
        &mut self,
        zone: Zone,
        setting: SyncSetting,
        enabled: bool,
    ) -> Result<(), ControllerError> {
        self.zone_mut(zone)?.set_sync_enabled(setting, enabled);
        Ok(())
    }

    pub fn set_custom_color(&mut self, zone: Zone, enabled: bool) -> Result<(), ControllerError> {
        self.zone_mut(zone)?.set_custom_color_enabled(enabled);
        Ok(())
    }

    pub fn set_rainbow_cycle(
        &mut self,
        zone: Zone,
        cycle_or_led_count: u8,
    ) -> Result<(), ControllerError> {
        match zone {
            Zone::JRainbow1 => self.packet.j_rainbow_1.cycle_or_led_count = cycle_or_led_count,
            Zone::JRainbow2 => self.packet.j_rainbow_2.cycle_or_led_count = cycle_or_led_count,
            _ => return Err(ControllerError::InvalidZone(zone)),
        }
        Ok(())
    }

    pub fn set_corsair_quantity(&mut self, quantity: u8) {
        self.packet.j_corsair.quantity = quantity;
    }

    pub fn read_current(&mut self) -> Result<(), ControllerError> {
        let bytes = self
            .transport
            .read_feature_report(FEATURE_REPORT_ID, FEATURE_PACKET_LEN)?;
        self.packet = FeaturePacket185::decode(&bytes)?;
        Ok(())
    }

    pub fn update(&mut self) -> Result<(), ControllerError> {
        let desired = self.packet;
        self.read_current()?;
        self.send_current()?;
        self.packet = desired;
        self.send_current()
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
        if !self.capabilities.supports_zone(zone.to_msi_zone()) {
            return Err(ControllerError::InvalidZone(zone));
        }
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

    #[test]
    fn controller_updates_sync_flags_without_losing_other_bits() {
        let mut controller = MsiController::new(RecordingTransport::default());
        controller
            .set_jrgb_sync(Zone::JRgb1, true)
            .expect("valid zone");
        controller
            .set_color_sync(Zone::JRgb1, SyncSetting::Jpipe2, true)
            .expect("valid zone");
        controller
            .set_custom_color(Zone::JRgb1, true)
            .expect("valid zone");

        let zone = &controller.packet().j_rgb_1;
        assert!(zone.jrgb_sync_enabled());
        assert!(zone.sync_enabled(SyncSetting::Jpipe2));
        assert!(zone.custom_color_enabled());
        assert_eq!(
            FeaturePacket185::decode(&controller.packet().encode()),
            Ok(*controller.packet())
        );
    }

    #[test]
    fn controller_limits_rainbow_cycle_to_rainbow_zones() {
        let mut controller = MsiController::new(RecordingTransport::default());
        controller
            .set_rainbow_cycle(Zone::JRainbow2, 40)
            .expect("valid zone");
        assert_eq!(controller.packet().j_rainbow_2.cycle_or_led_count, 40);
        assert!(matches!(
            controller.set_rainbow_cycle(Zone::JRgb1, 40),
            Err(ControllerError::InvalidZone(Zone::JRgb1))
        ));
    }
}
