use xshell::Shell;

use crate::config::{Args, Config};

pub fn clean(args: &Args) -> anyhow::Result<()> {
    let config_path = &args.opts.config;
    let base_dir = config_path
        .parent()
        .unwrap_or_else(|| std::path::Path::new("."));

    let config = Config::load(config_path)?;
    let base_dir = base_dir.canonicalize()?;

    let build_dir = base_dir.join(&config.build.build_dir);

    let sh = Shell::new()?;
    if build_dir.exists() {
        log::info!("Removing build directory: {}", build_dir.display());
        sh.remove_path(&build_dir)?;
    } else {
        log::info!("Build directory does not exist: {}", build_dir.display());
    }

    Ok(())
}
