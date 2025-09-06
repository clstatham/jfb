use std::path::PathBuf;

use anyhow::Result;
use clap::Parser;

use crate::config::{Config, TargetConfig, TargetLanguage, TargetType};

macro_rules! template_gitignore {
    ($build_dir: expr, $deps_dir: expr) => {
        format!(
            r#"
# Ignore build artifacts
/{build_dir}/
/{deps_dir}/

# Ignore language server artifacts
/.cache/

# Ignore OS generated files
.DS_Store
Thumbs.db
"#,
            build_dir = $build_dir,
            deps_dir = $deps_dir
        )
        .trim_start()
    };
}

macro_rules! template_c_executable_main {
    () => {
        r#"
#include <stdio.h>

int main(void) {
    printf("Hello, World!\n");

    return 0;
}
"#
        .trim_start()
    };
}

macro_rules! template_c_library_lib {
    ($lib_name:expr) => {
        format!(
            r#"
#include "{lib_name}.h"
#include <stdio.h>

void {lib_name}_hello(void) {{
    printf("Hello from {lib_name}!\n");
}}
"#,
            lib_name = $lib_name
        )
        .trim_start()
    };
}

macro_rules! template_c_library_lib_h {
    ($lib_name:expr) => {
        format!(
            r#"
#ifndef {lib_name_upper}_H
#define {lib_name_upper}_H

void {lib_name}_hello(void);

#endif // {lib_name_upper}_H
"#,
            lib_name = $lib_name,
            lib_name_upper = $lib_name.to_uppercase()
        )
        .trim_start()
    };
}

macro_rules! template_cpp_executable_main {
    () => {
        r#"
#include <iostream>

int main() {
    std::cout << "Hello, World!" << std::endl;
    
    return 0;
}
"#
        .trim_start()
    };
}

macro_rules! template_cpp_library_lib {
    ($lib_name:expr) => {
        format!(
            r#"
#include "{lib_name}.h"
#include <iostream>
void {lib_name}_hello() {{
    std::cout << "Hello from {lib_name}!" << std::endl;
}}
"#,
            lib_name = $lib_name
        )
        .trim_start()
    };
}

macro_rules! template_cpp_library_lib_h {
    ($lib_name:expr) => {
        format!(
            r#"
#ifndef {lib_name_upper}_H
#define {lib_name_upper}_H

void {lib_name}_hello();

#endif // {lib_name_upper}_H
"#,
            lib_name = $lib_name,
            lib_name_upper = $lib_name.to_uppercase()
        )
        .trim_start()
    };
}

#[derive(Debug, Parser)]
pub struct NewOpts {
    /// Name of the project
    #[clap(short, long)]
    pub name: String,

    /// Language of the project
    #[clap(short, long, alias = "lang")]
    pub language: TargetLanguage,

    /// Add a binary (executable) target
    #[arg(long, action = clap::ArgAction::Append)]
    pub bin: Vec<String>,

    /// Add a library target
    #[arg(long, action = clap::ArgAction::Append)]
    pub lib: Vec<String>,

    /// Do not create sample starting files
    #[clap(long, default_value_t = false)]
    pub bare: bool,
}

pub fn new(opts: &NewOpts) -> Result<()> {
    let mut config = Config::new(&opts.name);

    for bin in opts.bin.iter() {
        config.targets.push(TargetConfig {
            name: bin.to_string(),
            target_type: TargetType::Binary,
            language: opts.language,
            source_dirs: vec![PathBuf::from(format!("{}/src", bin))],
            include_dirs: vec![PathBuf::from(format!("{}/include", bin))],
            ..Default::default()
        });
    }

    for lib in opts.lib.iter() {
        config.targets.push(TargetConfig {
            name: lib.to_string(),
            target_type: TargetType::StaticLibrary,
            language: opts.language,
            source_dirs: vec![PathBuf::from(format!("{}/src", lib))],
            include_dirs: vec![PathBuf::from(format!("{}/include", lib))],
            ..Default::default()
        });
    }

    let sh = xshell::Shell::new()?;
    sh.create_dir(&opts.name)?;
    {
        let _guard = sh.push_dir(&opts.name);

        let toml_str = toml::to_string_pretty(&config)?;
        sh.write_file("jfb.toml", &toml_str)?;

        sh.write_file(
            ".gitignore",
            template_gitignore!(
                config.workspace.build_dir.display(),
                config.workspace.dep_dir.display()
            ),
        )?;

        for target in config.targets.iter() {
            for dir in target.source_dirs.iter() {
                sh.create_dir(dir)?;
            }

            for dir in target.include_dirs.iter() {
                sh.create_dir(dir)?;
            }

            if !opts.bare {
                let target_name = &target.name;
                let _guard = sh.push_dir(&target.name);

                match (&target.target_type, &target.language) {
                    (TargetType::Binary, TargetLanguage::C) => {
                        sh.write_file("src/main.c", template_c_executable_main!())?;
                    }
                    (TargetType::Binary, TargetLanguage::Cpp) => {
                        sh.write_file("src/main.cpp", template_cpp_executable_main!())?;
                    }
                    (TargetType::StaticLibrary, TargetLanguage::C) => {
                        sh.write_file(
                            format!("src/{target_name}.c"),
                            template_c_library_lib!(target_name),
                        )?;
                        sh.write_file(
                            format!("include/{target_name}.h"),
                            template_c_library_lib_h!(target_name),
                        )?;
                    }
                    (TargetType::StaticLibrary, TargetLanguage::Cpp) => {
                        sh.write_file(
                            format!("src/{target_name}.cpp"),
                            template_cpp_library_lib!(target_name),
                        )?;
                        sh.write_file(
                            format!("include/{target_name}.h"),
                            template_cpp_library_lib_h!(target_name),
                        )?;
                    }
                }
            }
        }
    }

    Ok(())
}
