use std::{
    collections::HashMap,
    path::{Path, PathBuf},
};

use clap::{Parser, Subcommand, ValueEnum};
use serde::{Deserialize, Serialize};

use crate::commands::{build::BuildOpts, clean::CleanOpts, new::NewOpts};

#[derive(Debug, Parser)]
#[command(author, version, about, long_about = None)]
pub struct Args {
    /// Command to execute
    #[command(subcommand)]
    pub command: Command,

    /// Global options
    #[clap(flatten)]
    pub opts: Opts,
}

#[derive(Debug, Parser)]
pub struct Opts {
    /// Path to configuration file to load
    #[arg(short, long, default_value = "jfb.toml")]
    pub config: PathBuf,
}

#[derive(Debug, Subcommand)]
pub enum Command {
    /// Initialize a new project
    New {
        #[clap(flatten)]
        opts: NewOpts,
    },

    /// Build the project
    Build {
        #[clap(flatten)]
        opts: BuildOpts,
    },

    /// Build and run the project
    Run {
        #[clap(flatten)]
        build_opts: BuildOpts,
    },

    /// Clean build artifacts
    Clean {
        #[clap(flatten)]
        opts: CleanOpts,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    /// Workspace configuration
    #[serde(rename = "workspace")]
    pub workspace: WorkspaceConfig,

    /// Global build configuration
    #[serde(rename = "build")]
    #[serde(default)]
    pub build: BuildConfig,

    /// Dependency configurations
    #[serde(rename = "dependencies")]
    #[serde(default)]
    pub dependencies: HashMap<String, DependencyConfig>,

    /// Target configurations
    #[serde(rename = "target")]
    #[serde(default)]
    pub targets: Vec<TargetConfig>,
}

impl Config {
    /// Load configuration from a TOML file
    pub fn load(path: impl AsRef<Path>) -> anyhow::Result<Self> {
        let config = config::Config::builder()
            .add_source(config::File::from(path.as_ref()).required(true))
            .build()?;

        let config = config.try_deserialize::<Config>()?;

        Ok(config)
    }

    /// Create a new default configuration with the given project name
    pub fn new(name: &str) -> Self {
        Self {
            workspace: WorkspaceConfig {
                name: name.to_string(),
            },
            build: BuildConfig::default(),
            dependencies: HashMap::new(),
            targets: Vec::new(),
        }
    }
}

/// Workspace configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkspaceConfig {
    /// Name of the project
    pub name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct BuildConfig {
    /// Directory to place build artifacts
    pub build_dir: PathBuf,

    /// Directory to store dependency source code
    pub dep_dir: PathBuf,

    /// Whether to output a `compile_commands.json` file
    pub output_compile_commands: bool,

    /// Optimization level (0, 1, 2, 3, s, z, etc)
    pub opt_level: String,

    /// C compiler to use
    pub c_compiler: String,

    /// C++ compiler to use
    pub cpp_compiler: String,

    /// C standard to use (c99, c11, c17, c23, etc)
    pub c_standard: String,

    /// C++ standard to use (c++11, c++14, c++17, c++20, c++23, etc)
    pub cpp_standard: String,

    /// Linker to use
    pub c_linker: String,

    /// C++ linker to use
    pub cpp_linker: String,

    /// Include debug symbols
    pub debug: bool,

    /// Treat warnings as errors
    pub warnings_as_errors: bool,

    /// Warning flags to enable
    pub warnings: Vec<String>,

    /// Additional compiler flags
    pub flags: Vec<String>,

    /// Preprocessor defines
    pub defines: Vec<String>,
}

impl Default for BuildConfig {
    fn default() -> Self {
        Self {
            build_dir: PathBuf::from("build"),
            dep_dir: PathBuf::from("deps"),
            output_compile_commands: true,
            opt_level: "0".to_string(),
            c_compiler: "gcc".to_string(),
            cpp_compiler: "g++".to_string(),
            c_standard: "c11".to_string(),
            cpp_standard: "c++11".to_string(),
            c_linker: "gcc".to_string(),
            cpp_linker: "g++".to_string(),
            debug: true,
            warnings_as_errors: false,
            warnings: vec![
                "all".to_string(),
                "extra".to_string(),
                "pedantic".to_string(),
                "shadow".to_string(),
                "format=2".to_string(),
            ],
            flags: vec![
                "-fdiagnostics-color=always".to_string(),
                "-fno-common".to_string(),
                "-fstack-protector-strong".to_string(),
                "-Wno-unused-parameter".to_string(),
            ],
            defines: vec![],
        }
    }
}

/// Overrides for build configuration on a per-target basis
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct BuildConfigOverrides {
    /// Directory to place build artifacts
    pub build_dir: Option<PathBuf>,

    /// Optimization level (0, 1, 2, 3, s, z)
    pub opt_level: Option<String>,

    /// C compiler to use
    pub c_compiler: Option<String>,

    /// C++ compiler to use
    pub cpp_compiler: Option<String>,

    /// C standard to use (c99, c11, c17, c23)
    pub c_standard: Option<String>,

    /// C++ standard to use (c++11, c++14, c++17, c++20, c++23)
    pub cpp_standard: Option<String>,

    /// C linker to use
    pub c_linker: Option<String>,

    /// C++ linker to use
    pub cpp_linker: Option<String>,

    /// Include debug symbols
    pub debug: Option<bool>,

    /// Treat warnings as errors
    pub warnings_as_errors: Option<bool>,

    /// Warning flags to enable
    pub warnings: Option<Vec<String>>,

    /// Additional compiler flags
    pub flags: Option<Vec<String>>,

    /// Preprocessor defines
    pub defines: Option<Vec<String>>,
}

/// Target programming language
#[derive(Copy, ValueEnum, Debug, Default, PartialEq, Clone, Serialize, Deserialize)]
pub enum TargetLanguage {
    /// C language
    #[default]
    #[clap(name = "c")]
    #[serde(rename = "c")]
    C,

    /// C++ language
    #[clap(name = "cpp", alias = "c++", alias = "cc", alias = "cxx")]
    #[serde(rename = "cpp", alias = "c++", alias = "cc", alias = "cxx")]
    Cpp,
}

/// Type of build target
#[derive(Copy, ValueEnum, Debug, Default, Clone, PartialEq, Serialize, Deserialize)]
pub enum TargetType {
    /// Binary executable target
    #[default]
    #[serde(rename = "binary", alias = "bin")]
    Binary,

    /// Static library target
    #[serde(rename = "staticlib", alias = "lib")]
    StaticLibrary,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct TargetConfig {
    /// Name of the target
    #[serde(rename = "name")]
    pub name: String,

    /// Type of the target (executable, staticlib, sharedlib)
    #[serde(rename = "type")]
    pub target_type: TargetType,

    /// Programming language of the target (c, cpp)
    pub language: TargetLanguage,

    /// Source files (supports glob patterns)
    pub source_dirs: Vec<PathBuf>,

    /// Include directories
    pub include_dirs: Vec<PathBuf>,

    /// Library directories
    pub library_dirs: Vec<PathBuf>,

    /// Libraries to link
    pub libraries: Vec<PathBuf>,

    /// External dependencies to link against
    pub dependencies: Vec<String>,

    /// Build configuration overrides for this target
    #[serde(rename = "build")]
    pub build_overrides: Option<BuildConfigOverrides>,
}

impl Default for TargetConfig {
    fn default() -> Self {
        Self {
            name: "mytarget".to_string(),
            target_type: TargetType::Binary,
            language: TargetLanguage::C,
            source_dirs: vec!["src".into()],
            include_dirs: vec!["include".into()],
            library_dirs: vec![],
            libraries: vec![],
            dependencies: vec![],
            build_overrides: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Hash)]
#[serde(default)]
pub struct DependencyConfig {
    /// URL to the Git repository (ending with '.git')
    pub git: String,

    /// Optional tag, branch, or commit to checkout
    pub tag: Option<String>,

    /// CMake configuration flags for this dependency
    pub cmake_flags: Vec<String>,
}

#[allow(clippy::derivable_impls)]
impl Default for DependencyConfig {
    fn default() -> Self {
        Self {
            git: String::new(),
            tag: None,
            cmake_flags: vec![],
        }
    }
}
