use std::path::Path;

use crate::config::{Args, Config, TargetType};
use anyhow::Result;
use xshell::{Shell, cmd};

use super::build::BuildOpts;

pub fn run(args: &Args, build_opts: BuildOpts) -> Result<()> {
    // build first
    crate::commands::build::build(args, build_opts)?;

    let config_path = &args.opts.config;
    let base_dir = config_path.parent().unwrap_or_else(|| Path::new("."));

    let base_dir = base_dir.canonicalize()?;
    let config = Config::load(config_path)?;

    let build_dir = base_dir.join(&config.build.build_dir);
    let executable = config
        .targets
        .iter()
        .find(|t| matches!(t.target_type, TargetType::Binary))
        .ok_or_else(|| anyhow::anyhow!("No executable target found in configuration"))?;

    let exe_path = build_dir.join(&executable.name).join(&executable.name);
    if !exe_path.exists() {
        return Err(anyhow::anyhow!(
            "Executable not found: {}",
            exe_path.display()
        ));
    }

    log::info!("Running executable: {}", exe_path.display());
    let sh = Shell::new()?;
    let _guard = sh.push_dir(&base_dir);
    cmd!(sh, "{exe_path}").quiet().run()?;

    Ok(())
}
