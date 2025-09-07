use std::io::Write;

use anyhow::Result;

use clap::Parser;

use config::{Args, Command};

pub mod commands;
pub mod config;

fn main() -> Result<()> {
    env_logger::Builder::new()
        .filter_level(log::LevelFilter::Info)
        .parse_env("JFB_LOG_LEVEL")
        .format_timestamp(None)
        .format_file(false)
        .format_module_path(false)
        .format_target(false)
        .format_level(true)
        .init();

    let args = Args::parse();

    match &args.command {
        Command::New { opts } => commands::new::new(opts),
        Command::Build { opts } => commands::build::build(&args, opts),
        Command::Clean { opts } => commands::clean::clean(&args, opts),
        Command::Run { build_opts } => commands::run::run(&args, build_opts),
    }?;

    Ok(())
}
