use clap::{Parser, Subcommand};

use crate::boards::{BoardCapabilities, COMMON_185_CAPABILITIES, zone_from_name};
use crate::controller::{MsiController, Zone};
use crate::hid::{FeatureReportTransport, RecordingTransport};
use crate::protocol::{
    Color, FEATURE_PACKET_LEN, FEATURE_REPORT_ID, MsiBrightness, MsiMode, MsiSpeed,
};

#[derive(Debug, Parser)]
#[command(
    name = "msilights",
    about = "Control MSI Mystic Light USB lighting devices",
    long_about = "Control MSI Mystic Light USB lighting devices over HID feature reports.\n\nUse --dry-run to inspect a packet without connecting to hardware. Hardware commands require the intended MSI device and suitable USB permissions."
)]
pub struct Cli {
    #[arg(long, global = true)]
    pub json: bool,
    #[command(subcommand)]
    pub command: Option<Command>,
}

#[derive(Debug, Subcommand)]
pub enum Command {
    #[command(about = "List zones known for the common MSI 185-byte controller")]
    Zones,
    #[command(about = "List supported normal lighting effects")]
    Effects,
    #[command(
        about = "Apply one lighting configuration to every writable zone",
        long_about = "Apply the same color, effect, speed, and brightness to all currently writable zones: JRGB1, JRGB2, PIPE1, PIPE2, JRAINBOW1, JRAINBOW2, and ONBOARD. Use --dry-run to generate the 185-byte report without changing hardware."
    )]
    #[command(name = "setall")]
    SetAll {
        #[arg(long)]
        color: String,
        #[arg(long, help = "Secondary RGB color; defaults to the primary color")]
        secondary: Option<String>,
        #[arg(
            long,
            default_value = "static",
            help = "Lighting effect name or numeric value"
        )]
        effect: String,
        #[arg(
            long,
            default_value = "medium",
            help = "Effect speed: low, medium, or high"
        )]
        speed: String,
        #[arg(
            long,
            default_value = "100",
            help = "Brightness percentage: 0, 10, ..., 100"
        )]
        brightness: String,
        #[arg(long, help = "Save the configuration to device memory")]
        save: bool,
        #[arg(long, help = "Print the packet without connecting to hardware")]
        dry_run: bool,
    },
    #[command(
        about = "Apply a lighting configuration to one writable zone",
        long_about = "Apply a color, effect, speed, and brightness to one writable zone. Use --dry-run to generate the 185-byte report without changing hardware."
    )]
    Set {
        #[arg(long, help = "Writable zone name, for example JRGB1 or ONBOARD")]
        zone: String,
        #[arg(long, help = "Primary RGB color as RRGGBB or #RRGGBB")]
        color: String,
        #[arg(long, help = "Secondary RGB color; defaults to the primary color")]
        secondary: Option<String>,
        #[arg(
            long,
            default_value = "static",
            help = "Lighting effect name or numeric value"
        )]
        effect: String,
        #[arg(
            long,
            default_value = "medium",
            help = "Effect speed: low, medium, or high"
        )]
        speed: String,
        #[arg(
            long,
            default_value = "100",
            help = "Brightness percentage: 0, 10, ..., 100"
        )]
        brightness: String,
        #[arg(long, help = "Save the configuration to device memory")]
        save: bool,
        #[arg(long, help = "Print the packet without connecting to hardware")]
        dry_run: bool,
    },
}

pub fn render(cli: &Cli) -> Option<String> {
    match &cli.command {
        Some(Command::Zones) => Some(render_zones(cli.json, &COMMON_185_CAPABILITIES)),
        Some(Command::Effects) => Some(render_effects(cli.json)),
        Some(Command::SetAll {
            color,
            secondary,
            effect,
            speed,
            brightness,
            save,
            dry_run,
        }) if *dry_run => Some(render_all(
            cli.json,
            color,
            secondary.as_deref(),
            effect,
            speed,
            brightness,
            *save,
        )),
        Some(Command::SetAll { .. }) => None,
        Some(Command::Set {
            zone,
            color,
            secondary,
            effect,
            speed,
            brightness,
            save,
            dry_run,
        }) if *dry_run => Some(render_set(
            cli.json,
            zone,
            color,
            secondary.as_deref(),
            effect,
            speed,
            brightness,
            *save,
        )),
        Some(Command::Set { .. }) => None,
        None => None,
    }
}

fn parse_color(value: &str) -> Result<Color, String> {
    let value = value.strip_prefix('#').unwrap_or(value);
    if value.len() != 6 {
        return Err("color must be six hexadecimal digits".into());
    }
    let bytes = (0..3)
        .map(|index| u8::from_str_radix(&value[index * 2..index * 2 + 2], 16))
        .collect::<Result<Vec<_>, _>>()
        .map_err(|_| "color must be hexadecimal".to_string())?;
    Ok(Color::new(bytes[0], bytes[1], bytes[2]))
}

fn parse_mode(value: &str) -> Result<MsiMode, String> {
    match value.to_ascii_lowercase().as_str() {
        "static" | "1" => Ok(MsiMode::Static),
        "breathing" | "2" => Ok(MsiMode::Breathing),
        "flashing" | "3" => Ok(MsiMode::Flashing),
        "double-flashing" | "double_flashing" | "4" => Ok(MsiMode::DoubleFlashing),
        "lightning" | "5" => Ok(MsiMode::Lightning),
        "meteor" | "7" => Ok(MsiMode::Meteor),
        "color-ring" | "color_ring" | "15" => Ok(MsiMode::ColorRing),
        "planetary" | "16" => Ok(MsiMode::Planetary),
        "double-meteor" | "double_meteor" | "17" => Ok(MsiMode::DoubleMeteor),
        "energy" | "18" => Ok(MsiMode::Energy),
        "blink" | "19" => Ok(MsiMode::Blink),
        "clock" | "20" => Ok(MsiMode::Clock),
        "color-pulse" | "color_pulse" | "21" => Ok(MsiMode::ColorPulse),
        "color-shift" | "color_shift" | "22" => Ok(MsiMode::ColorShift),
        "color-wave" | "color_wave" | "23" => Ok(MsiMode::ColorWave),
        "marquee" | "24" => Ok(MsiMode::Marquee),
        "rainbow-wave" | "rainbow_wave" | "26" => Ok(MsiMode::RainbowWave),
        "visor" | "27" => Ok(MsiMode::Visor),
        "rainbow-flashing" | "rainbow_flashing" | "29" => Ok(MsiMode::RainbowFlashing),
        "color-ring-double-flashing" | "color_ring_double_flashing" | "35" => {
            Ok(MsiMode::ColorRingDoubleFlashing)
        }
        "stack" | "36" => Ok(MsiMode::Stack),
        "fire" | "38" => Ok(MsiMode::Fire),
        _ => Err(format!("unsupported effect: {value}")),
    }
}

fn parse_speed(value: &str) -> Result<MsiSpeed, String> {
    match value.to_ascii_lowercase().as_str() {
        "low" | "0" => Ok(MsiSpeed::Low),
        "medium" | "1" => Ok(MsiSpeed::Medium),
        "high" | "2" => Ok(MsiSpeed::High),
        _ => Err(format!("unsupported speed: {value}")),
    }
}

fn parse_brightness(value: &str) -> Result<MsiBrightness, String> {
    match value.to_ascii_lowercase().as_str() {
        "0" | "off" => Ok(MsiBrightness::Off),
        "10" | "level10" => Ok(MsiBrightness::Level10),
        "20" | "level20" => Ok(MsiBrightness::Level20),
        "30" | "level30" => Ok(MsiBrightness::Level30),
        "40" | "level40" => Ok(MsiBrightness::Level40),
        "50" | "level50" => Ok(MsiBrightness::Level50),
        "60" | "level60" => Ok(MsiBrightness::Level60),
        "70" | "level70" => Ok(MsiBrightness::Level70),
        "80" | "level80" => Ok(MsiBrightness::Level80),
        "90" | "level90" => Ok(MsiBrightness::Level90),
        "100" | "level100" => Ok(MsiBrightness::Level100),
        _ => Err(format!("unsupported brightness: {value}")),
    }
}

fn controller_zone(zone: &str) -> Result<Zone, String> {
    match zone_from_name(zone) {
        Some(crate::boards::MsiZone::JRgb1) => Ok(Zone::JRgb1),
        Some(crate::boards::MsiZone::JRgb2) => Ok(Zone::JRgb2),
        Some(crate::boards::MsiZone::JPipe1) => Ok(Zone::JPipe1),
        Some(crate::boards::MsiZone::JPipe2) => Ok(Zone::JPipe2),
        Some(crate::boards::MsiZone::JRainbow1) => Ok(Zone::JRainbow1),
        Some(crate::boards::MsiZone::JRainbow2) => Ok(Zone::JRainbow2),
        Some(crate::boards::MsiZone::JCorsair) => {
            Err("JCORSAIR mapping is not yet supported".into())
        }
        Some(crate::boards::MsiZone::JRainbow3) => {
            Err("JRAINBOW3 mapping is not yet supported".into())
        }
        Some(crate::boards::MsiZone::OnBoardLed0) => Ok(Zone::OnBoard(0)),
        None => Err(format!("unknown zone: {zone}")),
        _ => Err(format!("unsupported zone: {zone}")),
    }
}

const WRITABLE_ZONES: [Zone; 7] = [
    Zone::JRgb1,
    Zone::JRgb2,
    Zone::JPipe1,
    Zone::JPipe2,
    Zone::JRainbow1,
    Zone::JRainbow2,
    Zone::OnBoard(0),
];

#[derive(Clone, Debug)]
pub struct SetRequest {
    pub zone: Zone,
    pub primary: Color,
    pub secondary: Color,
    pub effect: MsiMode,
    pub speed: MsiSpeed,
    pub brightness: MsiBrightness,
    pub save: bool,
}

impl SetRequest {
    fn parse(
        zone: &str,
        color: &str,
        secondary: Option<&str>,
        effect: &str,
        speed: &str,
        brightness: &str,
        save: bool,
    ) -> Result<Self, String> {
        Ok(Self {
            zone: controller_zone(zone)?,
            primary: parse_color(color)?,
            secondary: parse_color(secondary.unwrap_or(color))?,
            effect: parse_mode(effect)?,
            speed: parse_speed(speed)?,
            brightness: parse_brightness(brightness)?,
            save,
        })
    }
}

pub fn parse_set_command(command: &Command) -> Result<SetRequest, String> {
    match command {
        Command::Set {
            zone,
            color,
            secondary,
            effect,
            speed,
            brightness,
            save,
            ..
        } => SetRequest::parse(
            zone,
            color,
            secondary.as_deref(),
            effect,
            speed,
            brightness,
            *save,
        ),
        _ => Err("expected set command".into()),
    }
}

fn parse_options(
    color: &str,
    secondary: Option<&str>,
    effect: &str,
    speed: &str,
    brightness: &str,
    save: bool,
) -> Result<(Color, Color, MsiMode, MsiSpeed, MsiBrightness, bool), String> {
    Ok((
        parse_color(color)?,
        parse_color(secondary.unwrap_or(color))?,
        parse_mode(effect)?,
        parse_speed(speed)?,
        parse_brightness(brightness)?,
        save,
    ))
}

pub fn apply_set_request<T: FeatureReportTransport>(
    controller: &mut MsiController<T>,
    request: &SetRequest,
) -> Result<(), String> {
    controller
        .set_colors(request.zone, request.primary, request.secondary)
        .map_err(|error| error.to_string())?;
    controller
        .set_settings(
            request.zone,
            request.effect,
            request.speed,
            request.brightness,
        )
        .map_err(|error| error.to_string())?;
    controller.set_save(request.save);
    Ok(())
}

pub fn apply_all_command<T: FeatureReportTransport>(
    controller: &mut MsiController<T>,
    command: &Command,
) -> Result<(), String> {
    let Command::SetAll {
        color,
        secondary,
        effect,
        speed,
        brightness,
        save,
        ..
    } = command
    else {
        return Err("expected setall command".into());
    };
    let (color, secondary, effect, speed, brightness, save) = parse_options(
        color,
        secondary.as_deref(),
        effect,
        speed,
        brightness,
        *save,
    )?;
    apply_all_request(
        controller, color, secondary, effect, speed, brightness, save,
    )
}

fn apply_all_request<T: FeatureReportTransport>(
    controller: &mut MsiController<T>,
    color: Color,
    secondary: Color,
    effect: MsiMode,
    speed: MsiSpeed,
    brightness: MsiBrightness,
    save: bool,
) -> Result<(), String> {
    for zone in WRITABLE_ZONES {
        let request = SetRequest {
            zone,
            primary: color,
            secondary,
            effect,
            speed,
            brightness,
            save,
        };
        apply_set_request(controller, &request)?;
    }
    Ok(())
}

fn build_set_packet(
    zone: &str,
    color: &str,
    secondary: Option<&str>,
    effect: &str,
    speed: &str,
    brightness: &str,
    save: bool,
) -> Result<[u8; FEATURE_PACKET_LEN], String> {
    let request = SetRequest::parse(zone, color, secondary, effect, speed, brightness, save)?;
    let mut controller = MsiController::new(RecordingTransport::default());
    apply_set_request(&mut controller, &request)?;
    Ok(controller.packet().encode())
}

#[allow(clippy::too_many_arguments)]
fn render_all(
    json: bool,
    color: &str,
    secondary: Option<&str>,
    effect: &str,
    speed: &str,
    brightness: &str,
    save: bool,
) -> String {
    let result = parse_options(color, secondary, effect, speed, brightness, save).and_then(
        |(primary, secondary, effect, speed, brightness, save)| {
            let mut controller = MsiController::new(RecordingTransport::default());
            apply_all_request(
                &mut controller,
                primary,
                secondary,
                effect,
                speed,
                brightness,
                save,
            )?;
            Ok(controller.packet().encode())
        },
    );
    render_packet(json, result)
}

fn render_packet(json: bool, result: Result<[u8; FEATURE_PACKET_LEN], String>) -> String {
    match result {
        Ok(packet) if json => format!(
            r#"{{"report_id":{},"length":{},"bytes":"{}"}}\n"#,
            FEATURE_REPORT_ID,
            packet.len(),
            packet
                .iter()
                .map(|byte| format!("{byte:02x}"))
                .collect::<String>()
        ),
        Ok(packet) => format!(
            "report_id: {FEATURE_REPORT_ID:#04x}\nlength: {}\nbytes: {}\n",
            packet.len(),
            packet
                .iter()
                .map(|byte| format!("{byte:02x}"))
                .collect::<Vec<_>>()
                .join(" ")
        ),
        Err(error) => format!("error: {error}\n"),
    }
}

#[allow(clippy::too_many_arguments)]
fn render_set(
    json: bool,
    zone: &str,
    color: &str,
    secondary: Option<&str>,
    effect: &str,
    speed: &str,
    brightness: &str,
    save: bool,
) -> String {
    match build_set_packet(zone, color, secondary, effect, speed, brightness, save) {
        Ok(packet) if json => format!(
            r#"{{"report_id":{},"length":{},"bytes":"{}"}}\n"#,
            FEATURE_REPORT_ID,
            packet.len(),
            packet
                .iter()
                .map(|byte| format!("{byte:02x}"))
                .collect::<String>()
        ),
        Ok(packet) => format!(
            "report_id: {FEATURE_REPORT_ID:#04x}\nlength: {}\nbytes: {}\n",
            packet.len(),
            packet
                .iter()
                .map(|byte| format!("{byte:02x}"))
                .collect::<Vec<_>>()
                .join(" ")
        ),
        Err(error) => format!("error: {error}\n"),
    }
}

fn render_zones(json: bool, capabilities: &BoardCapabilities) -> String {
    if json {
        let zones = capabilities
            .zones
            .iter()
            .map(|zone| {
                format!(
                    r#"{{"name":"{}","packet_zone":{}}}"#,
                    zone.name(),
                    zone.packet_zone()
                )
            })
            .collect::<Vec<_>>()
            .join(",");
        format!(
            r#"{{"product_id":"0x{:04x}","zones":[{}]}}"#,
            capabilities.product_id, zones
        )
    } else {
        let mut output = format!("MSI product 0x{:04x}\n", capabilities.product_id);
        for zone in capabilities.zones {
            output.push_str(&format!("{}\n", zone.name()));
        }
        output
    }
}

fn render_effects(json: bool) -> String {
    let effects = [
        ("Static", 1),
        ("Breathing", 2),
        ("Flashing", 3),
        ("Double flashing", 4),
        ("Lightning", 5),
        ("Meteor", 7),
        ("Color ring", 15),
        ("Planetary", 16),
        ("Double meteor", 17),
        ("Energy", 18),
        ("Blink", 19),
        ("Clock", 20),
        ("Color pulse", 21),
        ("Color shift", 22),
        ("Color wave", 23),
        ("Marquee", 24),
        ("Rainbow wave", 26),
        ("Visor", 27),
        ("Rainbow flashing", 29),
        ("Color ring double flashing", 35),
        ("Stack", 36),
        ("Fire", 38),
    ];
    if json {
        let values = effects
            .iter()
            .map(|(name, value)| format!(r#"{{"name":"{}","value":{}}}"#, name, value))
            .collect::<Vec<_>>()
            .join(",");
        format!(r#"{{"effects":[{}]}}"#, values)
    } else {
        effects
            .iter()
            .map(|(name, value)| format!("{value:>3} {name}"))
            .collect::<Vec<_>>()
            .join("\n")
            + "\n"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::Parser;

    #[test]
    fn parses_metadata_commands() {
        assert!(matches!(
            Cli::try_parse_from(["msilights", "zones"]),
            Ok(Cli {
                command: Some(Command::Zones),
                ..
            })
        ));
        assert!(matches!(
            Cli::try_parse_from(["msilights", "--json", "effects"]),
            Ok(Cli {
                json: true,
                command: Some(Command::Effects)
            })
        ));
    }

    #[test]
    fn parses_all_advertised_effects() {
        for (value, expected) in [
            ("double-flashing", MsiMode::DoubleFlashing),
            ("lightning", MsiMode::Lightning),
            ("color-ring", MsiMode::ColorRing),
            ("planetary", MsiMode::Planetary),
            ("double-meteor", MsiMode::DoubleMeteor),
            ("energy", MsiMode::Energy),
            ("blink", MsiMode::Blink),
            ("clock", MsiMode::Clock),
            ("color-pulse", MsiMode::ColorPulse),
            ("color-shift", MsiMode::ColorShift),
            ("color-wave", MsiMode::ColorWave),
            ("marquee", MsiMode::Marquee),
            ("rainbow-wave", MsiMode::RainbowWave),
            ("visor", MsiMode::Visor),
            ("rainbow-flashing", MsiMode::RainbowFlashing),
            (
                "color-ring-double-flashing",
                MsiMode::ColorRingDoubleFlashing,
            ),
            ("stack", MsiMode::Stack),
            ("fire", MsiMode::Fire),
        ] {
            assert_eq!(parse_mode(value), Ok(expected));
        }
    }

    #[test]
    fn parses_and_applies_set_request_without_hardware() {
        let command = Cli::try_parse_from([
            "msilights",
            "set",
            "--zone",
            "JRGB1",
            "--color",
            "#102030",
            "--effect",
            "breathing",
            "--speed",
            "high",
            "--brightness",
            "70",
            "--save",
            "--dry-run",
        ])
        .unwrap()
        .command
        .unwrap();
        let request = parse_set_command(&command).unwrap();
        let mut controller = MsiController::new(RecordingTransport::default());
        apply_set_request(&mut controller, &request).unwrap();

        assert_eq!(
            controller.packet().j_rgb_1.color,
            Color::new(0x10, 0x20, 0x30)
        );
        assert_eq!(
            controller.packet().j_rgb_1.effect,
            MsiMode::Breathing.encode()
        );
        assert_eq!(controller.packet().j_rgb_1.speed(), Ok(MsiSpeed::High));
        assert_eq!(
            controller.packet().j_rgb_1.brightness(),
            Ok(MsiBrightness::Level70)
        );
        assert_eq!(controller.packet().save_data, 1);
    }

    #[test]
    fn applies_all_command_to_every_writable_zone() {
        let command = Cli::try_parse_from([
            "msilights",
            "setall",
            "--color",
            "FF3600",
            "--effect",
            "static",
            "--brightness",
            "90",
            "--dry-run",
        ])
        .unwrap()
        .command
        .unwrap();
        let mut controller = MsiController::new(RecordingTransport::default());
        apply_all_command(&mut controller, &command).unwrap();

        assert_eq!(
            controller.packet().j_rgb_1.color,
            Color::new(0xff, 0x36, 0x00)
        );
        assert_eq!(
            controller.packet().j_rgb_2.color,
            Color::new(0xff, 0x36, 0x00)
        );
        assert_eq!(
            controller.packet().j_pipe_1.color,
            Color::new(0xff, 0x36, 0x00)
        );
        assert_eq!(
            controller.packet().j_pipe_2.color,
            Color::new(0xff, 0x36, 0x00)
        );
        assert_eq!(
            controller.packet().j_rainbow_1.zone.color,
            Color::new(0xff, 0x36, 0x00)
        );
        assert_eq!(
            controller.packet().j_rainbow_2.zone.color,
            Color::new(0xff, 0x36, 0x00)
        );
        assert_eq!(
            controller.packet().on_board_led[0].color,
            Color::new(0xff, 0x36, 0x00)
        );
    }

    #[test]
    fn renders_all_dry_run_packet() {
        let cli =
            Cli::try_parse_from(["msilights", "setall", "--color", "FF3600", "--dry-run"]).unwrap();
        let output = render(&cli).unwrap();
        assert!(output.contains("report_id: 0x52"));
        assert!(output.contains("length: 185"));
    }

    #[test]
    fn renders_text_and_json_metadata() {
        let text = render(&Cli {
            json: false,
            command: Some(Command::Zones),
        })
        .unwrap();
        let json = render(&Cli {
            json: true,
            command: Some(Command::Zones),
        })
        .unwrap();
        assert!(text.contains("JRGB1"));
        assert!(json.contains("\"zones\""));
        assert!(
            render(&Cli {
                json: false,
                command: Some(Command::Effects)
            })
            .unwrap()
            .contains("Static")
        );
    }
}
