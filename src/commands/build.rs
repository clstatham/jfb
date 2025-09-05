use std::{
    collections::HashMap,
    path::{Path, PathBuf},
    time::SystemTime,
};

use anyhow::Result;
use clap::Parser;
use serde::{Deserialize, Serialize};
use xshell::{Shell, cmd};

use crate::config::{Args, Config, TargetConfig, TargetLanguage, TargetType};

pub mod deps;

#[derive(Debug, Clone, Parser)]
pub struct BuildOpts {}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompileCommand {
    pub directory: String,
    pub arguments: Vec<String>,
    pub file: String,
}

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct FileUpdateCache {
    #[serde(flatten)]
    cache: HashMap<PathBuf, SystemTime>,
}

impl FileUpdateCache {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn is_updated(&mut self, path: &Path) -> Result<bool> {
        // check if the modified time is greater than our cached time
        let metadata = std::fs::metadata(path)?;
        let modified = metadata.modified()?;
        let cached_time = self.cache.get(path);
        let is_updated = match cached_time {
            Some(cached) => &modified > cached,
            None => true,
        };
        if is_updated {
            self.cache.insert(path.to_path_buf(), modified);
        }
        Ok(is_updated)
    }
}

pub struct Builder {
    config: Config,
    _opts: BuildOpts,
    sh: Shell,
    base_dir: PathBuf,
    compile_commands: HashMap<PathBuf, CompileCommand>,
    file_cache: FileUpdateCache,
    config_updated: bool,
}

impl Builder {
    fn new(args: &Args, config: Config, opts: BuildOpts, base_dir: &Path) -> Result<Self> {
        let sh = Shell::new()?;
        let base_dir = base_dir.canonicalize()?;

        // load or initialize our file update cache
        let cache_path = base_dir
            .join(&config.build.build_dir)
            .join("jfb_cache.json");
        let mut file_cache = if std::fs::exists(&cache_path)? {
            let data = std::fs::read_to_string(cache_path)?;
            serde_json::from_str(&data)?
        } else {
            FileUpdateCache::new()
        };

        // check if the config file has been updated
        let config_updated = file_cache.is_updated(&args.opts.config)?;

        // load existing compile_commands if available
        let compile_commands_path = base_dir
            .join(&config.build.build_dir)
            .join("compile_commands.json");
        let compile_commands_vec: Vec<CompileCommand> = if std::fs::exists(&compile_commands_path)?
        {
            let data = std::fs::read_to_string(compile_commands_path)?;
            serde_json::from_str(&data)?
        } else {
            Vec::new()
        };

        // rebuild our compile_commands map from the vector
        let mut compile_commands = HashMap::new();
        for compile_command in compile_commands_vec {
            let path = PathBuf::from(&compile_command.file);
            compile_commands.insert(path, compile_command);
        }

        Ok(Self {
            config,
            _opts: opts,
            sh,
            base_dir,
            compile_commands,
            file_cache,
            config_updated,
        })
    }

    pub fn build(mut self) -> Result<()> {
        let build_dir = self.base_dir.join(&self.config.build.build_dir);
        self.sh.create_dir(&build_dir)?;
        log::debug!("Using build directory: {}", build_dir.display());

        // fetch and build dependencies first
        self.fetch_dependencies()?;
        self.build_dependencies()?;

        let targets = self.config.targets.clone();
        // compile every target
        for target in targets {
            log::info!("Building target: {}", target.name);

            self.compile_target(&target, &build_dir)?;
        }

        if self.config.build.output_compile_commands {
            self.write_build_artifacts()?;
        }

        log::info!("All targets built successfully.");

        Ok(())
    }

    fn compile_target(&mut self, target: &TargetConfig, build_dir: &Path) -> Result<()> {
        // create output directory for this target
        let out_dir = build_dir.join(&target.name);
        self.sh.create_dir(&out_dir)?;

        let mut src_files = vec![];
        let mut obj_files = vec![];

        // populate src_files and obj_files
        for src_dir in target.source_dirs.iter() {
            let src_dir = self.base_dir.join(src_dir);

            let entries = self.sh.read_dir(src_dir)?;
            for entry in entries {
                if entry.is_file()
                    && let Some(ext) = entry.extension()
                {
                    match target.language {
                        TargetLanguage::C if ext == "c" => {
                            src_files.push(entry.clone());
                            let obj_file =
                                out_dir.join(entry.with_extension("o").file_name().unwrap());
                            obj_files.push(obj_file);
                        }
                        TargetLanguage::Cpp if ext == "cpp" || ext == "cc" || ext == "cxx" => {
                            src_files.push(entry.clone());
                            let obj_file =
                                out_dir.join(entry.with_extension("o").file_name().unwrap());
                            obj_files.push(obj_file);
                        }
                        _ => {}
                    }
                }
            }
        }

        // compile our source files
        for (src, obj) in src_files.iter().zip(obj_files.iter()) {
            // check if the file has been updated compared to our cached update time
            if self.config_updated || self.file_cache.is_updated(src)? {
                self.compile_file(src, obj, target)?;
            } else {
                log::debug!("Skipping unchanged file: {}", src.display());
            }
        }

        match target.target_type {
            TargetType::Binary => {
                // link all object files into the final executable
                let output_exe = out_dir.join(&target.name);

                let linker = match target.language {
                    TargetLanguage::C => target
                        .build_overrides
                        .as_ref()
                        .and_then(|overrides| overrides.c_linker.as_ref())
                        .unwrap_or(&self.config.build.c_compiler),
                    TargetLanguage::Cpp => target
                        .build_overrides
                        .as_ref()
                        .and_then(|overrides| overrides.cpp_linker.as_ref())
                        .unwrap_or(&self.config.build.cpp_compiler),
                };

                let library_paths = target
                    .library_dirs
                    .iter()
                    .map(|dir| format!("-L{}", self.base_dir.join(dir).display()))
                    .collect::<Vec<_>>();

                let libraries = target
                    .libraries
                    .iter()
                    .map(|lib| {
                        let lib_name = lib.file_stem().unwrap().to_string_lossy();
                        format!("-l{}", lib_name.strip_prefix("lib").unwrap_or(&lib_name))
                    })
                    .collect::<Vec<_>>();

                cmd!(self.sh, "{linker}")
                    .args(&obj_files)
                    .args(&library_paths)
                    .args(&libraries)
                    .arg("-o")
                    .arg(&output_exe)
                    .quiet()
                    .run()?;

                log::debug!("Linked executable: {}", output_exe.display());
            }
            TargetType::StaticLibrary => {
                // archive all object files into a static library
                let output_lib = out_dir.join(format!("lib{}.a", &target.name));

                cmd!(self.sh, "ar")
                    .arg("rcs")
                    .arg(&output_lib)
                    .args(&obj_files)
                    .quiet()
                    .run()?;

                log::debug!("Created static library: {}", output_lib.display());
            }
        }

        Ok(())
    }

    fn compile_file(&mut self, src: &Path, obj: &Path, target: &TargetConfig) -> Result<()> {
        let compiler = match target.language {
            TargetLanguage::C => target
                .build_overrides
                .as_ref()
                .and_then(|overrides| overrides.c_compiler.as_ref())
                .unwrap_or(&self.config.build.c_compiler),
            TargetLanguage::Cpp => target
                .build_overrides
                .as_ref()
                .and_then(|overrides| overrides.cpp_compiler.as_ref())
                .unwrap_or(&self.config.build.cpp_compiler),
        };

        let standard = match target.language {
            TargetLanguage::C => target
                .build_overrides
                .as_ref()
                .and_then(|overrides| overrides.c_standard.as_ref())
                .unwrap_or(&self.config.build.c_standard),
            TargetLanguage::Cpp => target
                .build_overrides
                .as_ref()
                .and_then(|overrides| overrides.cpp_standard.as_ref())
                .unwrap_or(&self.config.build.cpp_standard),
        };
        let standard_arg = format!("-std={standard}");

        let include_dirs = target
            .include_dirs
            .iter()
            .map(|dir| format!("-I{}", self.base_dir.join(dir).display()))
            .collect::<Vec<_>>();

        let defines = self
            .config
            .build
            .defines
            .iter()
            .map(|def| format!("-D{}", def))
            .collect::<Vec<_>>();

        let flags = self
            .config
            .build
            .flags
            .iter()
            .chain(
                target
                    .build_overrides
                    .as_ref()
                    .and_then(|overrides| overrides.flags.as_ref())
                    .unwrap_or(&vec![]),
            )
            .map(|flag| flag.to_string())
            .collect::<Vec<_>>();

        let opt_level = format!(
            "-O{}",
            target
                .build_overrides
                .as_ref()
                .and_then(|overrides| overrides.opt_level.as_ref())
                .unwrap_or(&self.config.build.opt_level)
        );

        let warnings = self
            .config
            .build
            .warnings
            .iter()
            .chain(
                target
                    .build_overrides
                    .as_ref()
                    .and_then(|overrides| overrides.warnings.as_ref())
                    .unwrap_or(&vec![]),
            )
            .map(|warn| format!("-W{}", warn))
            .collect::<Vec<_>>();

        let mut extra_args = vec![];
        if target
            .build_overrides
            .as_ref()
            .and_then(|overrides| overrides.debug)
            .unwrap_or(self.config.build.debug)
        {
            extra_args.push("-g".to_string());
        }

        if target
            .build_overrides
            .as_ref()
            .and_then(|overrides| overrides.warnings_as_errors)
            .unwrap_or(self.config.build.warnings_as_errors)
        {
            extra_args.push("-Werror".to_string());
        }

        let command = cmd!(self.sh, "{compiler}")
            .arg(&standard_arg)
            .args(&flags)
            .args(&defines)
            .args(&include_dirs)
            .args(&warnings)
            .args(&extra_args)
            .arg(&opt_level)
            .arg("-c")
            .arg(src)
            .arg("-o")
            .arg(obj);

        if self.config.build.output_compile_commands {
            let command_str = command.to_string();
            let args: Vec<_> = command_str.split_whitespace().map(String::from).collect();

            let compile_command = CompileCommand {
                directory: self.base_dir.to_string_lossy().into_owned(),
                arguments: args,
                file: src.to_string_lossy().into_owned(),
            };

            self.compile_commands
                .insert(src.to_path_buf(), compile_command);
        }

        command.quiet().run()?;

        log::info!("Compiled {} to {}", src.display(), obj.display());

        Ok(())
    }

    fn write_build_artifacts(&self) -> Result<()> {
        let build_dir = self.base_dir.join(&self.config.build.build_dir);
        let compile_commands_path = build_dir.join("compile_commands.json");
        let compile_commands_vec: Vec<CompileCommand> = self
            .compile_commands
            .values()
            .map(ToOwned::to_owned)
            .collect();
        let compile_commands_json = serde_json::to_string_pretty(&compile_commands_vec)?;
        self.sh
            .write_file(&compile_commands_path, compile_commands_json)?;
        log::debug!(
            "Wrote compile commands to {}",
            compile_commands_path.display()
        );

        let file_update_cache_json = serde_json::to_string_pretty(&self.file_cache)?;
        let cache_path = build_dir.join("jfb_cache.json");
        self.sh.write_file(&cache_path, file_update_cache_json)?;
        log::debug!("Wrote file cache to {}", cache_path.display());

        Ok(())
    }
}

pub fn build(args: &Args, opts: BuildOpts) -> Result<()> {
    let base_dir = args
        .opts
        .config
        .parent()
        .unwrap_or_else(|| std::path::Path::new("."));
    let base_dir = base_dir.canonicalize()?;

    let config = Config::load(&args.opts.config)?;
    log::debug!("Loaded config: {:#?}", config);

    Builder::new(args, config, opts, &base_dir)?.build()?;

    Ok(())
}
