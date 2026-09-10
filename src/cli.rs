use clap::{Parser, Subcommand};

use crate::boards::{BoardCapabilities, COMMON_185_CAPABILITIES};

#[derive(Debug, Parser)]
#[command(name = "msilights")]
pub struct Cli {
    #[arg(long, global = true)]
    pub json: bool,
    #[command(subcommand)]
    pub command: Option<Command>,
}

#[derive(Debug, Subcommand)]
pub enum Command {
    Zones,
    Effects,
}

pub fn render(cli: &Cli) -> Option<String> {
    match cli.command {
        Some(Command::Zones) => Some(render_zones(cli.json, &COMMON_185_CAPABILITIES)),
        Some(Command::Effects) => Some(render_effects(cli.json)),
        None => None,
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
