use serde_derive::*;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};

use crate::error::*;
use crate::language::*;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub default_template: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SolutionConfig {
    /// The hostname to upload the solution to.
    pub hostname: String,

    /// The id of the problem
    pub problem: String,

    /// The submission.
    #[serde(flatten)]
    pub submission: Submission,

    /// SubCommands to execute in order to build the solution.
    #[serde(default)]
    pub build: Vec<String>,

    /// SubCommands to execute in order to run the solution, the sample input will be piped into the
    /// last command.
    #[serde(default)]
    pub run: Vec<String>,

    /// The directory that contains the samples.
    #[serde(default = "default_samples_dir")]
    pub samples: PathBuf,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TemplateSolutionConfig {
    /// The directory that contains the samples.
    #[serde(default = "default_samples_dir")]
    pub samples: PathBuf,

    /// The submission.
    #[serde(flatten)]
    pub submission: Submission,

    /// SubCommands to execute in order to build the solution.
    #[serde(default)]
    pub build: Vec<String>,

    /// SubCommands to execute in order to run the solution, the sample input will be piped into the
    /// last command.
    #[serde(default)]
    pub run: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Submission {
    /// Files that should be submitted to the judge.
    pub files: Vec<PathBuf>,

    /// Set the language used in submission.
    #[serde(with = "crate::util::serde_string")]
    pub language: Language,

    /// Set the main class/file used in submission.
    pub mainclass: Option<String>,
}

impl Default for Config {
    fn default() -> Config {
        Config {
            default_template: None,
        }
    }
}

impl Default for TemplateSolutionConfig {
    fn default() -> TemplateSolutionConfig {
        TemplateSolutionConfig {
            samples: default_samples_dir(),
            submission: Submission::default(),
            build: Vec::new(),
            run: Vec::new(),
        }
    }
}

impl Default for Submission {
    fn default() -> Submission {
        Submission {
            files: Vec::new(),
            language: Language::CPlusPlus,
            mainclass: None,
        }
    }
}

fn default_samples_dir() -> PathBuf {
    PathBuf::from("./samples")
}

impl Config {
    pub fn home_directory() -> Result<PathBuf> {
        env::var("KATTIS_CONFIG_HOME")
            .ok()
            .map(PathBuf::from)
            .or_else(|| dirs::config_dir()?.join("kattis").into())
            .ok_or(Error::MissingConfigDirectory)
    }

    pub fn file_path() -> Result<PathBuf> {
        let path = Self::home_directory()?.join("kattis-global.yml");
        Ok(path)
    }

    pub fn init_home_directory(home: impl AsRef<Path>) -> Result<()> {
        let home = home.as_ref();
        fs::create_dir(home)?;
        fs::create_dir(home.join("templates"))?;
        fs::create_dir(home.join("credentials"))?;
        Ok(())
    }

    pub fn load(home: impl AsRef<Path>) -> Result<Config> {
        let home = home.as_ref();

        if !home.exists() {
            Self::init_home_directory(&home)?;
        }

        let config_file = home.join("kattis-global.yml");

        let config = if !config_file.exists() {
            let config = Config::default();
            let file = fs::File::create(&config_file)?;
            serde_yaml::to_writer(file, &config)?;
            config
        } else {
            let file = fs::File::open(&config_file)?;
            serde_yaml::from_reader(file)?
        };

        Ok(config)
    }
}

impl SolutionConfig {
    pub fn from_template(
        template: TemplateSolutionConfig,
        problem: String,
        hostname: String,
    ) -> SolutionConfig {
        SolutionConfig {
            problem,
            hostname,
            submission: template.submission,
            build: template.build,
            run: template.run,
            samples: template.samples,
        }
    }

    pub fn load(directory: impl AsRef<Path>) -> Result<SolutionConfig> {
        let config_file = directory.as_ref().join("kattis.yml");

        if !config_file.is_file() {
            Err(Error::SolutionConfigNotFound { path: config_file })
        } else {
            let file = fs::File::open(&config_file)?;
            let config = serde_yaml::from_reader(file)?;
            Ok(config)
        }
    }

    pub fn save_in(&self, directory: impl AsRef<Path>) -> Result<()> {
        let config_file = directory.as_ref().join("kattis.yml");
        let file = fs::File::create(config_file)?;
        serde_yaml::to_writer(file, self)?;
        Ok(())
    }
}

impl TemplateSolutionConfig {
    pub fn load(directory: impl AsRef<Path>) -> Result<TemplateSolutionConfig> {
        let config_file = directory.as_ref().join("kattis.yml");

        if !config_file.is_file() {
            Err(Error::SolutionConfigNotFound { path: config_file })
        } else {
            let file = fs::File::open(&config_file)?;
            let config = serde_yaml::from_reader(file)?;
            Ok(config)
        }
    }

    /// Returns the default configuration if the file did not already exist
    pub fn load_or_default(directory: impl AsRef<Path>) -> Result<TemplateSolutionConfig> {
        match TemplateSolutionConfig::load(&directory) {
            Ok(config) => Ok(config),
            Err(Error::SolutionConfigNotFound { path }) => {
                warn!(
                    "The template did not contain a configuration file ({:?}). Using default...",
                    path
                );
                Ok(TemplateSolutionConfig::default())
            }
            Err(e) => Err(e),
        }
    }

    pub fn save_in(&self, directory: impl AsRef<Path>) -> Result<()> {
        let config_file = directory.as_ref().join("kattis.yml");
        let file = fs::File::create(config_file)?;
        serde_yaml::to_writer(file, self)?;
        Ok(())
    }
}

