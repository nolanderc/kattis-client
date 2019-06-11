#[macro_use]
mod macros;

use derive_more::*;
use failure::Fail;
use reqwest::StatusCode;
use serde_derive::*;
use std::env;
use std::fs;
use std::io::{Cursor, Read, Write};
use std::path::{Path, PathBuf};
use structopt::StructOpt;
use zip::ZipArchive;
use std::process::Command;
use std::collections::HashMap;
use std::str::from_utf8;

#[derive(Debug, Fail, From)]
pub enum Error {
    #[fail(
        display = "Could not find the configuration directory. Try setting the KATTIS_CONFIG_HOME
           environment variable"
    )]
    MissingConfigDirectory,

    #[fail(display = "Could not find the problem configuration file: {:?}", path)]
    ProblemConfigNotFound { path: PathBuf },

    #[fail(display = "Could not download the sample")]
    DownloadSample,

    #[fail(display = "The template was not found: {:?}", path)]
    TemplateNotFound { path: PathBuf },

    #[fail(
        display = "No templete was specified. Try running again with the -t flag or set the `default_template` in the configuration file."
    )]
    TemplateNotSpecified,

    #[fail(display = "The target directory does not exist: {:?}", path)]
    TargetDirectoryNotFound { path: PathBuf },

    #[fail(display = "The sample directory does not exist: {:?}", path)]
    SampleDirectoryNotFound { path: PathBuf },

    #[fail(display = "A solution with the same name already exists: {:?}", path)]
    SolutionDirectoryExists { path: PathBuf },

    #[fail(display = "A template with the same name already exists: {:?}", path)]
    TemplateDirectoryExists { path: PathBuf },

    #[fail(display = "Could not find a problem with the id \"{}\"", problem)]
    ProblemNotFound { problem: String },

    #[fail(display = "Build command failed: {}", command)]
    BuildCommandFailed { command: String },

    #[fail(display = "Run command failed: {}", command)]
    RunCommandFailed { command: String },

    #[fail(display = "No run commands provided")]
    RunCommandsMissing,

    #[fail(display = "Answer contained invalid UTF-8: {}", _0)]
    InvalidUtf8Answer(#[cause] std::str::Utf8Error),

    #[fail(display = "Kattis responded with an error: {}", code)]
    Kattis { code: StatusCode },

    #[fail(display = "{}", _0)]
    IoError(#[cause] std::io::Error),

    #[fail(display = "{}", _0)]
    YamlError(serde_yaml::Error),

    #[fail(display = "{}", _0)]
    Reqwest(reqwest::Error),

    #[fail(display = "{}", _0)]
    Zip(zip::result::ZipError),

    #[fail(display = "{}", _0)]
    FsExtra(fs_extra::error::Error),
}

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, StructOpt)]
#[structopt(rename_all = "kebab-case")]
struct Args {
    /// The domain to operate on. The default is `open.kattis.com`.
    ///
    /// May be configured in the configuration file.
    #[structopt(short = "d", long = "domain")]
    domain: Option<String>,

    #[structopt(subcommand)]
    command: SubCommand,
}

#[derive(Debug, StructOpt)]
#[structopt(rename_all = "kebab-case")]
enum SubCommand {
    /// Creates a new solution to a problem in a new directory.
    ///
    /// Creates a new test suite from the samples from the problem page and configures the
    /// submission.
    New(NewSolution),

    /// Downloads the solutions from the problem page and stores them as separate files.
    Samples(DownloadSamples),

    /// Tests the solution in a directory against the samples.
    ///
    /// Builds the solution in a directory (defaults to the current) and validates the solution
    /// against the problem samples.
    Test(TestSolution),

    /// View, create and modify templates.
    Template(TemplateSubCommand),
}

#[derive(Debug, StructOpt)]
#[structopt(rename_all = "kebab-case")]
struct NewSolution {
    /// The template to use.
    #[structopt(short = "t", long = "template")]
    template: Option<String>,

    /// The id of the problem.
    ///
    /// Downloads the appropriate samples and configures the submission.
    #[structopt(short = "p", long = "problem")]
    problem: String,

    /// The name of the new directory. Defaults to the id of the problem.
    #[structopt(short = "d", long = "dir")]
    directory: Option<PathBuf>,
}

#[derive(Debug, StructOpt)]
#[structopt(rename_all = "kebab-case")]
struct DownloadSamples {
    /// The id of the problem.
    ///
    /// Downloads the appropriate samples.
    #[structopt(short = "p", long = "problem")]
    problem: String,

    /// The directory to store the samples within.
    #[structopt(short = "d", long = "dir", default_value = "./samples")]
    directory: PathBuf,
}

#[derive(Debug, StructOpt)]
#[structopt(rename_all = "kebab-case")]
struct TestSolution {
    /// The name of directory containing the solution.
    #[structopt(short = "d", long = "dir", default_value = "./")]
    directory: PathBuf,
}

#[derive(Debug, StructOpt)]
#[structopt(rename_all = "kebab-case")]
enum TemplateSubCommand {
    /// Create a new template.
    ///
    /// Prints the path to the new template.
    New {
        /// The name of the new template.
        name: String,
    },

    /// Print the names and paths of all templates.
    List,
}

#[derive(Debug, Clone)]
struct Sample {
    name: String,
    content: Vec<u8>,
}

#[derive(Debug, Clone)]
struct TestCase {
    name: String,
    input: PathBuf,
    answer: PathBuf,
}

#[derive(Debug, Serialize, Deserialize)]
struct Config {
    #[serde(default = "default_domain")]
    default_domain: String,

    default_template: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
struct ProblemConfig {
    /// The id of the problem
    problem: Option<String>,

    /// The directory that contains the samples.
    #[serde(default = "default_samples_dir")]
    samples: PathBuf,

    /// SubCommands to execute in order to build the solution.
    #[serde(default)]
    build: Vec<String>,

    /// SubCommands to execute in order to run the solution, the sample input will be piped into the
    /// last command.
    #[serde(default)]
    run: Vec<String>,
}

fn default_domain() -> String {
    "open.kattis.com".to_owned()
}

fn default_samples_dir() -> PathBuf {
    PathBuf::from("./samples")
}

fn main() {
    let args = Args::from_args();

    match args.execute() {
        Ok(()) => {
            println!("Done.");
        }
        Err(e) => {
            error!("Error: {}", e);
        }
    }
}

impl Config {
    pub fn home_directory() -> Result<PathBuf> {
        env::var("KATTIS_CONFIG_HOME")
            .ok()
            .map(PathBuf::from)
            .or_else(|| dirs::config_dir()?.join("kattis").into())
            .ok_or(Error::MissingConfigDirectory)
    }

    pub fn init_home_directory(home: impl AsRef<Path>) -> Result<()> {
        let home = home.as_ref();
        fs::create_dir(home)?;
        fs::create_dir(home.join("templates"))?;
        Ok(())
    }

    pub fn load(home: impl AsRef<Path>) -> Result<Config> {
        let home = home.as_ref();

        if !home.exists() {
            Self::init_home_directory(&home)?;
        }

        let config_file = home.join("kattis.yml");

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

impl ProblemConfig {
    pub fn load(directory: impl AsRef<Path>) -> Result<ProblemConfig> {
        let config_file = directory.as_ref().join("kattis.yml");

        if !config_file.is_file() {
            Err(Error::ProblemConfigNotFound { path: config_file })
        } else {
            let file = fs::File::open(&config_file)?;
            let config = serde_yaml::from_reader(file)?;
            Ok(config)
        }
    }

    /// Returns the default configuration if the file did not already exist
    pub fn load_or_default(directory: impl AsRef<Path>) -> Result<ProblemConfig> {
        match ProblemConfig::load(&directory) {
            Ok(config) => Ok(config),
            Err(Error::ProblemConfigNotFound { path }) => {
                warn!(
                    "The template did not contain a configuration file ({:?}). Using default...",
                    path
                );
                Ok(ProblemConfig::default())
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

impl Default for Config {
    fn default() -> Config {
        Config {
            default_domain: default_domain(),
            default_template: None,
        }
    }
}

impl Default for ProblemConfig {
    fn default() -> ProblemConfig {
        ProblemConfig {
            problem: None,
            samples: default_samples_dir(),
            build: Vec::new(),
            run: Vec::new(),
        }
    }
}

impl Args {
    pub fn execute(self) -> Result<()> {
        let config_home = Config::home_directory()?;
        let config = Config::load(&config_home)?;

        let domain = self.domain.as_ref().unwrap_or(&config.default_domain);

        match self.command {
            SubCommand::Samples(command) => {
                assert_problem_exists(domain, &command.problem)?;

                let samples = download_samples(domain, &command.problem)?;

                for sample in samples {
                    sample.save_in(&command.directory)?;
                }
            }

            SubCommand::New(command) => {
                let template = command
                    .template
                    .or_else(|| config.default_template.clone())
                    .ok_or(Error::TemplateNotSpecified)?;
                let template_dir = config_home.join("templates").join(template);

                if !template_dir.is_dir() {
                    return Err(Error::TemplateNotFound { path: template_dir });
                }

                let directory = match command.directory {
                    Some(dir) => dir,
                    None => PathBuf::new().join(&command.problem),
                };

                if directory.is_dir() {
                    return Err(Error::SolutionDirectoryExists { path: directory });
                }

                // before we do any visible changes to the user, make sure the problem actually
                // exists
                assert_problem_exists(domain, &command.problem)?;

                fs::create_dir(&directory)?;

                // Copy from template
                let mut template_items = Vec::new();
                for entry in fs::read_dir(template_dir)? {
                    template_items.push(entry?.path());
                }

                let options = fs_extra::dir::CopyOptions {
                    overwrite: false,
                    skip_exist: true,
                    buffer_size: 64000,
                    copy_inside: false,
                    depth: 0,
                };
                fs_extra::copy_items(&template_items, &directory, &options)?;

                let mut problem_config = ProblemConfig::load_or_default(&directory)?;
                problem_config.problem = Some(command.problem.to_owned());
                problem_config.save_in(&directory)?;

                let samples = download_samples(domain, &command.problem)?;

                let sample_dir = if problem_config.samples.is_relative() {
                    directory.join(&problem_config.samples)
                } else {
                    problem_config.samples
                };

                if !sample_dir.is_dir() {
                    fs::create_dir(&sample_dir)?;
                }

                for sample in samples {
                    sample.save_in(&sample_dir)?;
                }
            }

            SubCommand::Test(TestSolution { directory }) => {
                let problem_config = ProblemConfig::load(&directory)?;

                build_solution(&directory, &problem_config.build)?;

                let sample_dir = if problem_config.samples.is_relative() {
                    directory.join(&problem_config.samples)
                } else {
                    problem_config.samples
                };

                if !sample_dir.is_dir() {
                    return Err(Error::SampleDirectoryNotFound { path: sample_dir });
                }

                let samples = load_test_cases(&sample_dir)?;

                test_solution(&directory, &problem_config.run, &samples)?;
            }

            SubCommand::Template(TemplateSubCommand::New { name }) => {
                let template_dir = config_home.join("templates").join(name);

                if template_dir.exists() {
                    return Err(Error::TemplateDirectoryExists { path: template_dir });
                }

                fs::create_dir(&template_dir)?;

                let config = ProblemConfig::default();
                config.save_in(&template_dir)?;

                if let Some(text) = template_dir.to_str() {
                    eprint!("Created template: ");
                    println!("{}", text);
                } else {
                    warn!(
                        "Template path does not contain valid unicode, printing lossy version..."
                    );

                    println!("{}", template_dir.to_string_lossy());
                }
            }

            SubCommand::Template(TemplateSubCommand::List) => {
                let templates_dir = config_home.join("templates");

                let mut templates = Vec::new();
                for entry in fs::read_dir(&templates_dir)? {
                    let entry = entry?;
                    let path = entry.path();
                    if path.is_dir() {
                        let file_name = entry.file_name().into_string();
                        let path_name = path.into_os_string().into_string();

                        if let (Ok(name), Ok(path)) = (file_name, path_name) {
                            templates.push((name, path));
                        }
                    }
                }

                if templates.len() > 0 {
                    let max_len = templates
                        .iter()
                        .map(|(name, _)| name.chars().count())
                        .max()
                        .unwrap();

                    for (name, path) in templates {
                        let chars = name.chars().count();
                        print!("{}", name);
                        for _ in chars..max_len + 2 {
                            print!(" ")
                        }
                        println!("{}", path)
                    }
                }
            }
        }

        Ok(())
    }
}

fn assert_problem_exists(domain: &str, problem: &str) -> Result<()> {
    if problem_exists(domain, problem)? {
        Ok(())
    } else {
        // TODO: list problems with similar names
        Err(Error::ProblemNotFound {
            problem: problem.to_owned(),
        })
    }
}

fn problem_exists(domain: &str, problem: &str) -> Result<bool> {
    let url = format!(
        "https://{domain}/problems/{problem}",
        domain = domain,
        problem = problem
    );

    let res = reqwest::get(&url)?;

    match res.status() {
        StatusCode::OK => Ok(true),
        StatusCode::NOT_FOUND => Ok(false),
        code => Err(Error::Kattis { code }),
    }
}

fn build_solution(directory: impl AsRef<Path>, build_commands: &[String]) -> Result<()> {
    let current_dir = directory.as_ref().canonicalize()?;

    for command in build_commands {
        let status = Command::new("sh")
            .arg("-c")
            .arg(command)
            .current_dir(&current_dir)
            .status()?;

        if !status.success() {
            Err(Error::BuildCommandFailed { command: command.clone() })?;
        }
    }

    Ok(())
}

fn load_test_cases(path: impl AsRef<Path>) -> Result<Vec<TestCase>> {
    let mut sets = HashMap::new();

    for entry in fs::read_dir(path)? {
        let entry = entry?;
        let path = entry.path();

        if path.is_file() {
            if let Some(name) = path.file_stem().and_then(|n| n.to_str()) {
                let name = name.to_owned();

                let extension = path.extension();
                let extension_is = |ext: &str| extension.filter(|e| *e == ext).is_some();

                if  extension_is("in") {
                    sets.entry(name).or_insert((None, None)).0 = Some(path);
                } else if extension_is("ans") {
                    sets.entry(name).or_insert((None, None)).1 = Some(path);
                }
            }
        }
    }

    let mut test_cases: Vec<_> = sets.into_iter()
        .filter_map(|(name, pair)| match pair {
            (Some(input), Some(answer)) => Some(TestCase {
                name,
                input,
                answer
            }),
            _ => None
        })
        .collect();

    test_cases.sort_by(|a, b| a.name.cmp(&b.name));

    Ok(test_cases)
}

fn test_solution(directory: impl AsRef<Path>, run_commands: &[String], cases: &[TestCase]) -> Result<()> {
    let current_dir = directory.as_ref().canonicalize()?;

    let n_commands = run_commands.len();
    if n_commands == 0 {
        Err(Error::RunCommandsMissing)?;
    }

    for case in cases {
        println!("Running test case: {}", case.name);

        if n_commands > 1 {
            for command in run_commands[..n_commands - 1].iter() {
                let status = Command::new("sh")
                    .arg("-c")
                    .arg(command)
                    .current_dir(&current_dir)
                    .status()?;

                if !status.success() {
                    Err(Error::BuildCommandFailed { command: command.clone() })?;
                }
            }
        }

        let final_run_command = &run_commands[n_commands - 1];
        let output = Command::new("sh")
            .arg("-c")
            .arg(final_run_command)
            .current_dir(&current_dir)
            .stdin(fs::File::open(&case.input)?)
            .output()?;

        if !output.status.success() {
            let error = Error::RunCommandFailed { command: final_run_command.clone() }; 
            error!("{}", error);
        } else {
            let mut answer = fs::File::open(&case.answer)?;
            let mut expected = Vec::new();
            answer.read_to_end(&mut expected)?;

            let mut term = term::stdout().unwrap();
            if output.stdout == expected {
                term.fg(term::color::GREEN).unwrap();
                println!("Correct");
                term.reset().unwrap();
            } else {
                term.fg(term::color::RED).unwrap();
                println!("Wrong Answer");
                term.reset().unwrap();

                let out = from_utf8(&output.stdout).map_err(Error::InvalidUtf8Answer)?;
                let ans = from_utf8(&expected).map_err(Error::InvalidUtf8Answer)?;

                println!("Found:\n{}", out);
                println!("Expected:\n{}", ans);
            }
        }
    }

    Ok(())
}

fn download_samples(domain: &str, problem: &str) -> Result<Vec<Sample>> {
    let url = format!(
        "https://{domain}/problems/{problem}/file/statement/samples.zip",
        domain = domain,
        problem = problem
    );

    let mut res = reqwest::get(&url)?;

    let mut archive = if res.status().is_success() {
        let mut buffer = Vec::new();
        res.read_to_end(&mut buffer)?;
        ZipArchive::new(Cursor::new(buffer))?
    } else {
        Err(Error::DownloadSample)?
    };

    let mut samples = Vec::new();
    for i in 0..archive.len() {
        let mut file = archive.by_index(i)?;

        let name = file.name().to_owned();
        let mut content = Vec::new();
        file.read_to_end(&mut content)?;

        samples.push(Sample { name, content });
    }

    Ok(samples)
}

impl Sample {
    fn save_in(&self, path: impl AsRef<Path>) -> Result<()> {
        let path = path.as_ref();

        if !path.exists() {
            Err(Error::TargetDirectoryNotFound { path: path.into() })?;
        }

        let file_path = path.join(&self.name);

        let mut file = fs::File::create(file_path)?;
        file.write(&self.content)?;

        Ok(())
    }
}
