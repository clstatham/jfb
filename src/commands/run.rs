use crate::config::{Args, Config};
use xshell::{Shell, cmd};

use super::build::BuildOpts;

pub fn run(args: &Args, build_opts: &BuildOpts) -> anyhow::Result<()> {
    let config_path = &args.opts.config;
    let base_dir = config_path
        .parent()
        .unwrap_or_else(|| std::path::Path::new("."));

    let config = Config::load(config_path)?;
    let base_dir = base_dir.canonicalize()?;

    // First, build the project
    crate::commands::build::build(args, build_opts)?;

    // Then, run the executable target
    let build_dir = base_dir.join(&config.build.build_dir);
    let (executable_name, _executable_config) = config
        .targets
        .iter()
        .find(|(_, t)| matches!(t.target_type, crate::config::TargetType::Executable))
        .ok_or_else(|| anyhow::anyhow!("No executable target found in configuration"))?;

    let exe_path = build_dir.join(executable_name).join(executable_name);
    if !exe_path.exists() {
        return Err(anyhow::anyhow!(
            "Executable not found: {}",
            exe_path.display()
        ));
    }

    log::info!("Running executable: {}", exe_path.display());
    let sh = Shell::new()?;
    let _guard = sh.push_dir(&base_dir);
    cmd!(sh, "{exe_path}").run()?;

    Ok(())
}
