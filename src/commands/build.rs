use anyhow::Result;
use clap::Parser;
use serde::{Deserialize, Serialize};
use xshell::{Shell, cmd};

use crate::config::{Args, Config, TargetLanguage, TargetType};

#[derive(Debug, Parser)]
pub struct BuildOpts {}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompileCommand {
    pub directory: String,
    pub arguments: Vec<String>,
    pub file: String,
}

pub type CompileCommands = Vec<CompileCommand>;

pub fn build(args: &Args, _opts: &BuildOpts) -> Result<()> {
    let sh = Shell::new()?;
    let base_dir = args
        .opts
        .config
        .parent()
        .unwrap_or_else(|| std::path::Path::new("."));
    let base_dir = base_dir.canonicalize()?;

    let _guard = sh.push_dir(&base_dir);
    log::debug!("Set working directory to: {}", base_dir.display());

    let config = Config::load(&args.opts.config)?;
    log::debug!("Loaded config: {:#?}", config);

    let build_dir = base_dir.join(&config.build.build_dir);
    sh.create_dir(&build_dir)?;
    log::debug!("Using build directory: {}", build_dir.display());

    let mut compile_commands = vec![];

    // compile every target
    for target in config.targets.iter() {
        // skip static libraries for now
        if target.target_type == TargetType::StaticLibrary {
            log::info!("Skipping static library target: {}", target.name);
            continue;
        }

        let mut src_files = vec![];
        let mut obj_files = vec![];

        log::info!("Building target: {}", target.name);

        let target_dir = base_dir.join(&target.name);

        let out_dir = build_dir.join(&target.name);
        sh.create_dir(&out_dir)?;

        for src_dir in target.source_dirs.iter() {
            let src_dir = target_dir.join(src_dir);

            let entries = sh.read_dir(src_dir)?;
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

        for (src, obj) in src_files.iter().zip(obj_files.iter()) {
            let compiler = match target.language {
                TargetLanguage::C => target
                    .build_overrides
                    .as_ref()
                    .and_then(|overrides| overrides.c_compiler.as_ref())
                    .unwrap_or(&config.build.c_compiler),
                TargetLanguage::Cpp => target
                    .build_overrides
                    .as_ref()
                    .and_then(|overrides| overrides.cpp_compiler.as_ref())
                    .unwrap_or(&config.build.cpp_compiler),
            };

            let include_dirs = target
                .include_dirs
                .iter()
                .map(|dir| format!("-I{}", target_dir.join(dir).display()))
                .collect::<Vec<_>>();

            let defines = config
                .build
                .defines
                .iter()
                .map(|def| format!("-D{}", def))
                .collect::<Vec<_>>();

            let flags = config
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
                    .unwrap_or(&config.build.opt_level)
            );

            let command = cmd!(sh, "{compiler}")
                .args(&flags)
                .args(&defines)
                .args(&include_dirs)
                .arg(&opt_level)
                .arg("-c")
                .arg(src)
                .arg("-o")
                .arg(obj);

            if config.build.output_compile_commands {
                let command_str = command.to_string();
                let args: Vec<String> = command_str
                    .split_ascii_whitespace()
                    .map(String::from)
                    .collect();

                let compile_command = CompileCommand {
                    directory: base_dir.to_string_lossy().into_owned(),
                    arguments: args,
                    file: src.to_string_lossy().into_owned(),
                };

                compile_commands.push(compile_command);
            }

            command.run()?;

            log::info!("Compiled {} to {}", src.display(), obj.display());
        }

        // Link object files into the final executable
        let output_exe = out_dir.join(&target.name);
        let linker = target
            .build_overrides
            .as_ref()
            .and_then(|overrides| overrides.linker.as_ref())
            .unwrap_or(&config.build.linker);

        cmd!(sh, "{linker}")
            .args(&obj_files)
            .arg("-o")
            .arg(&output_exe)
            .run()?;

        log::info!("Linked executable: {}", output_exe.display());
    }

    if config.build.output_compile_commands {
        let compile_commands_path = build_dir.join("compile_commands.json");
        let json = serde_json::to_string_pretty(&compile_commands)?;
        sh.write_file(&compile_commands_path, json)?;
        log::info!(
            "Wrote compile commands to {}",
            compile_commands_path.display()
        );
    }

    log::info!("All targets built successfully.");

    Ok(())
}
