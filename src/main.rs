#[allow(dead_code)]
mod boards;
mod cli;
#[allow(dead_code)]
mod controller;
mod error;
mod hid;
#[allow(dead_code)]
mod protocol;

use clap::Parser;
use cli::{Cli, Command, apply_set_request, parse_set_command};
use controller::MsiController;
use error::AppError;
use hid::open_matching;
use rusb::Context;

fn main() -> Result<(), AppError> {
    let cli = Cli::parse();

    if let Some(Command::Set { dry_run: false, .. }) = &cli.command {
        let command = cli.command.as_ref().expect("set command was matched");
        let request = parse_set_command(command).map_err(AppError::Cli)?;
        let context = Context::new()?;
        let transport = open_matching(&context, 0)?;
        let mut controller = MsiController::new(transport);
        apply_set_request(&mut controller, &request).map_err(AppError::Cli)?;
        controller
            .update()
            .map_err(|error| AppError::Cli(error.to_string()))?;
        return Ok(());
    }

    if let Some(Command::All { dry_run: false, .. }) = &cli.command {
        let context = Context::new()?;
        let transport = open_matching(&context, 0)?;
        let mut controller = MsiController::new(transport);
        cli::apply_all_command(&mut controller, cli.command.as_ref().unwrap())
            .map_err(AppError::Cli)?;
        controller
            .update()
            .map_err(|error| AppError::Cli(error.to_string()))?;
        return Ok(());
    }

    if let Some(output) = cli::render(&cli) {
        print!("{output}");
        return Ok(());
    }

    Err(AppError::Cli(
        "a command is required; use --help for usage".into(),
    ))
}
