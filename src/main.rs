use anyhow::Result;

use clap::Parser;

use config::{Args, Command};

pub mod commands;
pub mod config;

fn main() -> Result<()> {
    env_logger::try_init_from_env("JFB_LOG_LEVEL")?;

    let args = Args::parse();

    match &args.command {
        Command::New { opts } => commands::new::new(opts),
        Command::Build { opts } => commands::build::build(&args, opts.clone()),
        Command::Clean { opts } => commands::clean::clean(&args, opts),
        Command::Run { build_opts } => commands::run::run(&args, build_opts.clone()),
    }?;

    Ok(())
}
