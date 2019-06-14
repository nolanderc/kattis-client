use derive_more::*;
use failure::Fail;
use reqwest::StatusCode;
use std::path::PathBuf;

#[derive(Debug, Fail, From)]
pub enum Error {
    #[fail(
        display = "Could not find the configuration directory. Try setting the KATTIS_CONFIG_HOME
           environment variable"
    )]
    MissingConfigDirectory,

    #[fail(display = "Could not find the solution configuration file: {:?}", path)]
    SolutionConfigNotFound { path: PathBuf },

    #[fail(display = "Could not download the sample: {}", code)]
    DownloadSample { code: StatusCode },

    #[fail(
        display = "No templates match the name '{}'. Run 'kattis template show' to show a list of all templates",
        name
    )]
    NoMatchingTemplate { name: String },

    #[fail(
        display = "Multiple templates match the name '{}'. Run 'kattis template show' to show a list of all templates",
        name
    )]
    MultipleTemplateCandidates { name: String },

    #[fail(display = "Found template, but it was not a directory: {:?}", path)]
    TemplateNotDirectory { path: PathBuf },

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

    #[fail(display = "Failed to login to Kattis: {}", code)]
    LoginFailed { code: StatusCode },

    #[fail(display = "Failed to submit to Kattis: {}", code)]
    SubmitFailed { code: StatusCode },

    #[fail(display = "No credentials match the hostname '{}'", name)]
    NoMatchingCredentials { name: String },

    #[fail(display = "Multiple credentials match the hostname '{}'", name)]
    MultipleCredentialCandidates { name: String },

    #[fail(display = "When parsing credentials: {}", _0)]
    CredentialsParse(#[cause] crate::credentials::CredentailsParseError),

    #[fail(
        display = "Failed to extract submission id from string: {:?}",
        response
    )]
    SubmissionIdExtractFailed { response: String },

    #[fail(display = "Failed to read submission status: {}", _0)]
    SubmissionRowParse(crate::session::ParseSubmissionRowError),

    #[fail(display = "{}", _0)]
    LanguageParse(#[cause] crate::language::LanguageParseError),

    #[fail(display = "{}", _0)]
    IoError(#[cause] std::io::Error),

    #[fail(display = "Failed to compile regex: {}", _0)]
    RegexError(#[cause] regex::Error),

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
