//! Checked codecs for MSI's 185-byte feature report.

use std::fmt;

pub const FEATURE_REPORT_ID: u8 = 0x52;
pub const FEATURE_PACKET_LEN: usize = 185;

const SPEED_MASK: u8 = 0x03;
const BRIGHTNESS_SHIFT: u8 = 2;
const BRIGHTNESS_MASK: u8 = 0x1f << BRIGHTNESS_SHIFT;
const SYNC_SETTING_JRGB: u8 = 1 << 7;
const SYNC_SETTING_ONBOARD: u8 = 1 << 0;
const SYNC_SETTING_JRAINBOW1: u8 = 1 << 1;
const SYNC_SETTING_JRAINBOW2: u8 = 1 << 2;
const SYNC_SETTING_JCORSAIR: u8 = 1 << 3;
const SYNC_SETTING_JPIPE1: u8 = 1 << 4;
const SYNC_SETTING_JPIPE2: u8 = 1 << 5;
const USE_CUSTOM_COLOR: u8 = 1 << 7;

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct Color {
    pub red: u8,
    pub green: u8,
    pub blue: u8,
}

impl Color {
    pub const fn new(red: u8, green: u8, blue: u8) -> Self {
        Self { red, green, blue }
    }

    fn encode(self, output: &mut [u8]) {
        output[..3].copy_from_slice(&[self.red, self.green, self.blue]);
    }

    fn decode(bytes: &[u8]) -> Self {
        Self::new(bytes[0], bytes[1], bytes[2])
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct ZoneData {
    pub effect: u8,
    pub color: Color,
    pub speed_and_brightness: u8,
    pub color2: Color,
    pub color_flags: u8,
    pub padding: u8,
}

impl ZoneData {
    pub const LEN: usize = 10;

    fn encode(self, output: &mut [u8]) {
        self.color.encode(&mut output[1..4]);
        output[0] = self.effect;
        output[4] = self.speed_and_brightness;
        self.color2.encode(&mut output[5..8]);
        output[8] = self.color_flags;
        output[9] = self.padding;
    }

    fn decode(bytes: &[u8]) -> Self {
        Self {
            effect: bytes[0],
            color: Color::decode(&bytes[1..4]),
            speed_and_brightness: bytes[4],
            color2: Color::decode(&bytes[5..8]),
            color_flags: bytes[8],
            padding: bytes[9],
        }
    }

    pub fn speed(&self) -> Result<MsiSpeed, ProtocolError> {
        MsiSpeed::decode(self.speed_and_brightness & SPEED_MASK)
    }

    pub fn brightness(&self) -> Result<MsiBrightness, ProtocolError> {
        MsiBrightness::decode((self.speed_and_brightness & BRIGHTNESS_MASK) >> BRIGHTNESS_SHIFT)
    }

    pub fn set_speed(&mut self, speed: MsiSpeed) {
        self.speed_and_brightness = (self.speed_and_brightness & !SPEED_MASK) | speed.encode();
    }

    pub fn set_brightness(&mut self, brightness: MsiBrightness) {
        self.speed_and_brightness = (self.speed_and_brightness & !BRIGHTNESS_MASK)
            | (brightness.encode() << BRIGHTNESS_SHIFT);
    }

    pub fn jrgb_sync_enabled(&self) -> bool {
        self.speed_and_brightness & SYNC_SETTING_JRGB != 0
    }

    pub fn set_jrgb_sync_enabled(&mut self, enabled: bool) {
        set_flag(&mut self.speed_and_brightness, SYNC_SETTING_JRGB, enabled);
    }

    pub fn sync_enabled(&self, setting: SyncSetting) -> bool {
        self.color_flags & setting.mask() != 0
    }

    pub fn set_sync_enabled(&mut self, setting: SyncSetting, enabled: bool) {
        set_flag(&mut self.color_flags, setting.mask(), enabled);
    }

    pub fn custom_color_enabled(&self) -> bool {
        self.color_flags & USE_CUSTOM_COLOR != 0
    }

    pub fn set_custom_color_enabled(&mut self, enabled: bool) {
        set_flag(&mut self.color_flags, USE_CUSTOM_COLOR, enabled);
    }
}

fn set_flag(byte: &mut u8, mask: u8, enabled: bool) {
    if enabled {
        *byte |= mask;
    } else {
        *byte &= !mask;
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SyncSetting {
    Onboard,
    Jrainbow1,
    Jrainbow2,
    Jcorsair,
    Jpipe1,
    Jpipe2,
}

impl SyncSetting {
    const fn mask(self) -> u8 {
        match self {
            Self::Onboard => SYNC_SETTING_ONBOARD,
            Self::Jrainbow1 => SYNC_SETTING_JRAINBOW1,
            Self::Jrainbow2 => SYNC_SETTING_JRAINBOW2,
            Self::Jcorsair => SYNC_SETTING_JCORSAIR,
            Self::Jpipe1 => SYNC_SETTING_JPIPE1,
            Self::Jpipe2 => SYNC_SETTING_JPIPE2,
        }
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct RainbowZoneData {
    pub zone: ZoneData,
    pub cycle_or_led_count: u8,
}

impl RainbowZoneData {
    const LEN: usize = 11;

    fn encode(self, output: &mut [u8]) {
        self.zone.encode(&mut output[..ZoneData::LEN]);
        output[ZoneData::LEN] = self.cycle_or_led_count;
    }

    fn decode(bytes: &[u8]) -> Self {
        Self {
            zone: ZoneData::decode(&bytes[..ZoneData::LEN]),
            cycle_or_led_count: bytes[ZoneData::LEN],
        }
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct CorsairZoneData {
    pub effect: u8,
    pub color: Color,
    pub fan_flags: u8,
    pub quantity: u8,
    pub padding: [u8; 4],
    pub individual: u8,
}

impl CorsairZoneData {
    const LEN: usize = 11;

    fn encode(self, output: &mut [u8]) {
        output[0] = self.effect;
        self.color.encode(&mut output[1..4]);
        output[4] = self.fan_flags;
        output[5] = self.quantity;
        output[6..10].copy_from_slice(&self.padding);
        output[10] = self.individual;
    }

    fn decode(bytes: &[u8]) -> Self {
        Self {
            effect: bytes[0],
            color: Color::decode(&bytes[1..4]),
            fan_flags: bytes[4],
            quantity: bytes[5],
            padding: bytes[6..10].try_into().expect("fixed-size slice"),
            individual: bytes[10],
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(u8)]
pub enum MsiMode {
    Disable = 0,
    Static = 1,
    Breathing = 2,
    Flashing = 3,
    DoubleFlashing = 4,
    Lightning = 5,
    MsiMarquee = 6,
    Meteor = 7,
    WaterDrop = 8,
    MsiRainbow = 9,
    Pop = 10,
    Rap = 11,
    Jazz = 12,
    Play = 13,
    Movie = 14,
    ColorRing = 15,
    Planetary = 16,
    DoubleMeteor = 17,
    Energy = 18,
    Blink = 19,
    Clock = 20,
    ColorPulse = 21,
    ColorShift = 22,
    ColorWave = 23,
    Marquee = 24,
    Rainbow = 25,
    RainbowWave = 26,
    Visor = 27,
    Jrainbow = 28,
    RainbowFlashing = 29,
    RainbowDoubleFlashing = 30,
    Random = 31,
    FanControl = 32,
    Disable2 = 33,
    ColorRingFlashing = 34,
    ColorRingDoubleFlashing = 35,
    Stack = 36,
    CorsairQue = 37,
    Fire = 38,
    Lava = 39,
    DirectDummy = 100,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(u8)]
pub enum MsiSpeed {
    Low = 0,
    Medium = 1,
    High = 2,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(u8)]
pub enum MsiBrightness {
    Off = 0,
    Level10 = 1,
    Level20 = 2,
    Level30 = 3,
    Level40 = 4,
    Level50 = 5,
    Level60 = 6,
    Level70 = 7,
    Level80 = 8,
    Level90 = 9,
    Level100 = 10,
}

impl MsiSpeed {
    pub const fn encode(self) -> u8 {
        self as u8
    }

    fn decode(value: u8) -> Result<Self, ProtocolError> {
        match value {
            0 => Ok(Self::Low),
            1 => Ok(Self::Medium),
            2 => Ok(Self::High),
            value => Err(ProtocolError::InvalidSpeed { value }),
        }
    }
}

impl MsiBrightness {
    pub const fn encode(self) -> u8 {
        self as u8
    }

    fn decode(value: u8) -> Result<Self, ProtocolError> {
        match value {
            0 => Ok(Self::Off),
            1 => Ok(Self::Level10),
            2 => Ok(Self::Level20),
            3 => Ok(Self::Level30),
            4 => Ok(Self::Level40),
            5 => Ok(Self::Level50),
            6 => Ok(Self::Level60),
            7 => Ok(Self::Level70),
            8 => Ok(Self::Level80),
            9 => Ok(Self::Level90),
            10 => Ok(Self::Level100),
            value => Err(ProtocolError::InvalidBrightness { value }),
        }
    }
}

impl MsiMode {
    pub const fn encode(self) -> u8 {
        self as u8
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ProtocolError {
    WrongLength { expected: usize, actual: usize },
    WrongReportId { expected: u8, actual: u8 },
    InvalidSpeed { value: u8 },
    InvalidBrightness { value: u8 },
}

impl fmt::Display for ProtocolError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::WrongLength { expected, actual } => {
                write!(f, "expected {expected}-byte report, got {actual} bytes")
            }
            Self::WrongReportId { expected, actual } => {
                write!(f, "expected report ID {expected:#04x}, got {actual:#04x}")
            }
            Self::InvalidSpeed { value } => write!(f, "invalid speed value {value}"),
            Self::InvalidBrightness { value } => write!(f, "invalid brightness value {value}"),
        }
    }
}

impl std::error::Error for ProtocolError {}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct FeaturePacket185 {
    pub j_rgb_1: ZoneData,
    pub j_pipe_1: ZoneData,
    pub j_pipe_2: ZoneData,
    pub j_rainbow_1: RainbowZoneData,
    pub j_rainbow_2: RainbowZoneData,
    pub j_corsair: CorsairZoneData,
    pub j_corsair_outerll120: ZoneData,
    pub on_board_led: [ZoneData; 10],
    pub j_rgb_2: ZoneData,
    pub save_data: u8,
}

impl FeaturePacket185 {
    pub fn encode(&self) -> [u8; FEATURE_PACKET_LEN] {
        let mut output = [0; FEATURE_PACKET_LEN];
        output[0] = FEATURE_REPORT_ID;
        self.j_rgb_1.encode(&mut output[1..11]);
        self.j_pipe_1.encode(&mut output[11..21]);
        self.j_pipe_2.encode(&mut output[21..31]);
        self.j_rainbow_1.encode(&mut output[31..42]);
        self.j_rainbow_2.encode(&mut output[42..53]);
        self.j_corsair.encode(&mut output[53..64]);
        self.j_corsair_outerll120.encode(&mut output[64..74]);
        for (index, zone) in self.on_board_led.iter().enumerate() {
            let start = 74 + index * ZoneData::LEN;
            zone.encode(&mut output[start..start + ZoneData::LEN]);
        }
        self.j_rgb_2.encode(&mut output[174..184]);
        output[184] = self.save_data;
        output
    }

    pub fn decode(bytes: &[u8]) -> Result<Self, ProtocolError> {
        if bytes.len() != FEATURE_PACKET_LEN {
            return Err(ProtocolError::WrongLength {
                expected: FEATURE_PACKET_LEN,
                actual: bytes.len(),
            });
        }
        if bytes[0] != FEATURE_REPORT_ID {
            return Err(ProtocolError::WrongReportId {
                expected: FEATURE_REPORT_ID,
                actual: bytes[0],
            });
        }
        let mut on_board_led = [ZoneData::default(); 10];
        for (index, zone) in on_board_led.iter_mut().enumerate() {
            let start = 74 + index * ZoneData::LEN;
            *zone = ZoneData::decode(&bytes[start..start + ZoneData::LEN]);
        }
        Ok(Self {
            j_rgb_1: ZoneData::decode(&bytes[1..11]),
            j_pipe_1: ZoneData::decode(&bytes[11..21]),
            j_pipe_2: ZoneData::decode(&bytes[21..31]),
            j_rainbow_1: RainbowZoneData::decode(&bytes[31..42]),
            j_rainbow_2: RainbowZoneData::decode(&bytes[42..53]),
            j_corsair: CorsairZoneData::decode(&bytes[53..64]),
            j_corsair_outerll120: ZoneData::decode(&bytes[64..74]),
            on_board_led,
            j_rgb_2: ZoneData::decode(&bytes[174..184]),
            save_data: bytes[184],
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_packet_has_protocol_size_and_id() {
        let packet = FeaturePacket185::default().encode();
        assert_eq!(packet.len(), FEATURE_PACKET_LEN);
        assert_eq!(packet[0], FEATURE_REPORT_ID);
    }

    #[test]
    fn packet_round_trips_without_truncating_fields() {
        let packet = FeaturePacket185 {
            j_rgb_1: ZoneData {
                effect: 7,
                color: Color::new(1, 2, 3),
                speed_and_brightness: 0xaa,
                color2: Color::new(4, 5, 6),
                color_flags: 0xbb,
                padding: 0xcc,
            },
            j_rainbow_2: RainbowZoneData {
                cycle_or_led_count: 42,
                ..Default::default()
            },
            j_corsair: CorsairZoneData {
                quantity: 17,
                ..Default::default()
            },
            on_board_led: {
                let mut zones = [ZoneData::default(); 10];
                zones[9].color = Color::new(9, 8, 7);
                zones
            },
            j_rgb_2: ZoneData {
                effect: 38,
                ..Default::default()
            },
            save_data: 1,
            ..Default::default()
        };
        assert_eq!(FeaturePacket185::decode(&packet.encode()), Ok(packet));
    }

    #[test]
    fn decode_rejects_invalid_report() {
        let mut bytes = [0; FEATURE_PACKET_LEN];
        bytes[0] = 0x53;
        assert_eq!(
            FeaturePacket185::decode(&bytes),
            Err(ProtocolError::WrongReportId {
                expected: FEATURE_REPORT_ID,
                actual: 0x53,
            })
        );
    }

    #[test]
    fn zone_bitfields_round_trip_and_preserve_unrelated_bits() {
        let mut zone = ZoneData {
            speed_and_brightness: 0x80,
            color_flags: 0x40,
            ..Default::default()
        };
        zone.set_speed(MsiSpeed::High);
        zone.set_brightness(MsiBrightness::Level70);
        zone.set_jrgb_sync_enabled(true);
        zone.set_custom_color_enabled(true);
        zone.set_sync_enabled(SyncSetting::Jpipe2, true);

        assert_eq!(zone.speed(), Ok(MsiSpeed::High));
        assert_eq!(zone.brightness(), Ok(MsiBrightness::Level70));
        assert!(zone.jrgb_sync_enabled());
        assert!(zone.custom_color_enabled());
        assert!(zone.sync_enabled(SyncSetting::Jpipe2));
        assert_eq!(zone.speed_and_brightness, 0x9e);
        assert_eq!(zone.color_flags, 0xe0);

        zone.set_jrgb_sync_enabled(false);
        zone.set_custom_color_enabled(false);
        zone.set_sync_enabled(SyncSetting::Jpipe2, false);
        assert_eq!(zone.speed_and_brightness, 0x1e);
        assert_eq!(zone.color_flags, 0x40);
    }

    #[test]
    fn zone_bitfields_reject_invalid_values() {
        let zone = ZoneData {
            speed_and_brightness: 0x7f,
            ..Default::default()
        };
        assert_eq!(zone.speed(), Err(ProtocolError::InvalidSpeed { value: 3 }));
        assert_eq!(
            zone.brightness(),
            Err(ProtocolError::InvalidBrightness { value: 31 })
        );
    }
}
