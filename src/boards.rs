#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(u8)]
pub enum MsiZone {
    None = 0,
    JRgb1 = 1,
    JRgb2 = 2,
    JPipe1 = 3,
    JPipe2 = 4,
    JPipe3 = 5,
    JPipe4 = 6,
    JPipe5 = 7,
    JRainbow1 = 8,
    JRainbow2 = 9,
    JRainbow3 = 10,
    JCorsair = 11,
    JCorsairOuterLl120 = 12,
    OnBoardLed0 = 13,
}

impl MsiZone {
    pub const fn name(self) -> &'static str {
        match self {
            Self::None => "NONE",
            Self::JRgb1 => "JRGB1",
            Self::JRgb2 => "JRGB2",
            Self::JPipe1 => "PIPE1",
            Self::JPipe2 => "PIPE2",
            Self::JPipe3 => "PIPE3",
            Self::JPipe4 => "PIPE4",
            Self::JPipe5 => "PIPE5",
            Self::JRainbow1 => "JRAINBOW1",
            Self::JRainbow2 => "JRAINBOW2",
            Self::JRainbow3 => "JRAINBOW3",
            Self::JCorsair => "JCORSAIR",
            Self::JCorsairOuterLl120 => "JCORSAIR_OUTERLL120",
            Self::OnBoardLed0 => "ONBOARD",
        }
    }

    pub const fn packet_zone(self) -> bool {
        matches!(
            self,
            Self::JRgb1
                | Self::JRgb2
                | Self::JPipe1
                | Self::JPipe2
                | Self::JRainbow1
                | Self::JRainbow2
                | Self::JCorsair
                | Self::JCorsairOuterLl120
                | Self::OnBoardLed0
        )
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DirectMode {
    Disabled,
    ZoneBased,
    PerLed,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct BoardCapabilities {
    pub product_id: u16,
    pub zones: &'static [MsiZone],
    pub direct_mode: DirectMode,
    pub max_direct_leds: u16,
}

impl BoardCapabilities {
    pub fn supports_zone(&self, zone: MsiZone) -> bool {
        self.zones.contains(&zone)
    }
}

pub const COMMON_185_ZONES: &[MsiZone] = &[
    MsiZone::JRgb1,
    MsiZone::JRgb2,
    MsiZone::JRainbow1,
    MsiZone::JRainbow2,
    MsiZone::JRainbow3,
    MsiZone::JCorsair,
    MsiZone::JPipe1,
    MsiZone::JPipe2,
    MsiZone::OnBoardLed0,
];

pub const COMMON_185_CAPABILITIES: BoardCapabilities = BoardCapabilities {
    product_id: 0x0076,
    zones: COMMON_185_ZONES,
    direct_mode: DirectMode::Disabled,
    max_direct_leds: 0,
};

pub fn capabilities_for_product(product_id: u16) -> Option<&'static BoardCapabilities> {
    (product_id == COMMON_185_CAPABILITIES.product_id).then_some(&COMMON_185_CAPABILITIES)
}

pub fn zone_from_name(name: &str) -> Option<MsiZone> {
    match name.to_ascii_uppercase().as_str() {
        "JRGB1" => Some(MsiZone::JRgb1),
        "JRGB2" => Some(MsiZone::JRgb2),
        "JRAINBOW1" => Some(MsiZone::JRainbow1),
        "JRAINBOW2" => Some(MsiZone::JRainbow2),
        "JRAINBOW3" => Some(MsiZone::JRainbow3),
        "JCORSAIR" => Some(MsiZone::JCorsair),
        "PIPE1" => Some(MsiZone::JPipe1),
        "PIPE2" => Some(MsiZone::JPipe2),
        "ONBOARD" => Some(MsiZone::OnBoardLed0),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn common_zone_values_match_openrgb() {
        assert_eq!(MsiZone::JRgb1 as u8, 1);
        assert_eq!(MsiZone::JRainbow1 as u8, 8);
        assert_eq!(MsiZone::JCorsair as u8, 11);
    }

    #[test]
    fn names_resolve_case_insensitively() {
        assert_eq!(zone_from_name("jrainbow2"), Some(MsiZone::JRainbow2));
        assert_eq!(zone_from_name("missing"), None);
    }

    #[test]
    fn common_capabilities_are_conservative() {
        assert_eq!(
            capabilities_for_product(0x0076),
            Some(&COMMON_185_CAPABILITIES)
        );
        assert_eq!(capabilities_for_product(0x1234), None);
        assert_eq!(COMMON_185_CAPABILITIES.direct_mode, DirectMode::Disabled);
        assert_eq!(COMMON_185_CAPABILITIES.max_direct_leds, 0);
    }
}
