use anyhow::Result;
use xshell::cmd;

use crate::{commands::build::Builder, config::DependencyConfig};

impl<'a> Builder<'a> {
    pub fn download_dependency(&self, dep_name: &str, dep: &DependencyConfig) -> Result<()> {
        let dep_dir = self.base_dir.join(&self.config.workspace.dep_dir);
        if !dep_dir.exists() {
            self.sh.create_dir(&dep_dir)?;
        }

        let target_path = dep_dir.join(dep_name);
        if target_path.exists() {
            log::info!(
                "Dependency `{}` already exists, skipping download",
                dep_name
            );
        } else {
            log::info!("Cloning dependency `{}` from {}", dep_name, &dep.git);
            let mut git_cmd = cmd!(self.sh, "git clone");
            if let Some(tag) = &dep.tag {
                git_cmd = git_cmd.arg("--branch").arg(tag);
            }
            git_cmd = git_cmd.arg(&dep.git).arg(dep_name);
            let _guard = self.sh.push_dir(&dep_dir);
            git_cmd.run()?;
        }

        Ok(())
    }

    pub fn fetch_dependencies(&self) -> Result<()> {
        for (dep_name, dep) in self.config.dependencies.iter() {
            self.download_dependency(dep_name, dep)?;
        }
        Ok(())
    }

    pub fn build_dependency(&self, dep_name: &str, dep: &DependencyConfig) -> Result<()> {
        let dep_dir = self.base_dir.join(&self.config.workspace.dep_dir);
        let target_path = dep_dir.join(dep_name);
        if !target_path.exists() {
            return Err(anyhow::anyhow!(
                "Dependency `{}` not found at {}",
                dep_name,
                target_path.display()
            ));
        }

        let build_path = target_path.join("build");
        if !build_path.exists() {
            self.sh.create_dir(&build_path)?;
        }

        log::info!("Configuring dependency `{}`", dep_name);
        let _guard = self.sh.push_dir(&build_path);
        let mut cmake_cmd = cmd!(self.sh, "cmake ..");
        for flag in &dep.cmake_flags {
            cmake_cmd = cmake_cmd.arg(flag);
        }
        for flag in &self.build_profile().cmake_flags {
            cmake_cmd = cmake_cmd.arg(flag);
        }
        cmake_cmd.run()?;

        log::info!("Building dependency `{}`", dep_name);
        cmd!(self.sh, "cmake --build .").run()?;

        Ok(())
    }

    pub fn build_dependencies(&self) -> Result<()> {
        for (dep_name, dep) in self.config.dependencies.iter() {
            self.build_dependency(dep_name, dep)?;
        }
        Ok(())
    }
}
