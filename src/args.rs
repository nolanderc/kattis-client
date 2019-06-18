use std::path::PathBuf;
use structopt::StructOpt;
use regex::Regex;

use crate::language::*;

#[derive(Debug, StructOpt)]
#[structopt(rename_all = "kebab-case")]
pub struct Args {
    #[structopt(subcommand)]
    pub command: SubCommand,
}

#[derive(Debug, StructOpt)]
#[structopt(rename_all = "kebab-case")]
pub enum SubCommand {
    /// Create a solution to a problem in a new directory.
    ///
    /// Creates a new test suite from the samples from the problem page and configures the
    /// submission.
    New(NewSolution),

    /// Downloads the samples from the problem page and stores them as separate files.
    Samples(DownloadSamples),

    /// Tests the solution in a directory against the problem samples.
    ///
    /// Builds the solution in a directory (defaults to the current working directory) and validates the solution
    /// against the problem samples.
    Test(TestSolution),

    /// Submit a solution to the judge.
    Submit(SubmitSolution),

    /// View, create and modify solution templates.
    Template(TemplateSubCommand),

    /// View and change configuration parameters.
    Config(ConfigSubCommand),
}

#[derive(Debug, StructOpt)]
#[structopt(rename_all = "kebab-case")]
pub struct NewSolution {
    /// The template to use. Can be configured.
    #[structopt(short = "t", long = "template")]
    pub template: Option<String>,

    /// The id of the problem.
    ///
    /// Downloads the appropriate samples and configures the submission.
    #[structopt(short = "p", long = "problem")]
    pub problem: String,

    /// The name of the new directory. Defaults to the id of the problem.
    #[structopt(short = "d", long = "dir")]
    pub directory: Option<PathBuf>,

    /// The hostname to download from. The default is `open.kattis.com`.
    ///
    /// May be configured to another default in the configuration file.
    #[structopt(long = "hostname")]
    pub hostname: Option<String>,
}

#[derive(Debug, StructOpt)]
#[structopt(rename_all = "kebab-case")]
pub struct DownloadSamples {
    /// The id of the problem.
    ///
    /// Downloads the appropriate samples.
    #[structopt(short = "p", long = "problem")]
    pub problem: String,

    /// The directory to store the samples within.
    #[structopt(short = "d", long = "dir", default_value = "./samples")]
    pub directory: PathBuf,

    /// The hostname to download from. The default is `open.kattis.com`.
    ///
    /// May be configured to another default in the configuration file.
    #[structopt(long = "hostname")]
    pub hostname: Option<String>,
}

#[derive(Debug, StructOpt)]
#[structopt(rename_all = "kebab-case")]
pub struct TestSolution {
    /// The name of directory containing the solution.
    #[structopt(short = "d", long = "dir", default_value = "./")]
    pub directory: PathBuf,

    /// Rerun the tests when any of the submission files or samples change.
    #[structopt(short = "w", long = "watch")]
    pub watch: bool,

    /// Clear the screen before building and printing the test results. Works well in combination with '--watch'
    #[structopt(short = "c", long = "clear")]
    pub clear: bool,

    /// Ignore samples matching a regex pattern. 
    #[structopt(short = "i", long = "ignore")]
    pub ignore: Option<Regex>,

    /// Only test samples matching a regex pattern. 
    #[structopt(short = "f", long = "filter")]
    pub filter: Option<Regex>,
}

#[derive(Debug, StructOpt)]
#[structopt(rename_all = "kebab-case")]
pub struct SubmitSolution {
    /// The name of directory containing the solution.
    #[structopt(short = "d", long = "dir", default_value = "./")]
    pub directory: PathBuf,

    /// Override the language type.
    #[structopt(long = "lang")]
    pub language: Option<Language>,

    /// Override the main class.
    #[structopt(long = "main")]
    pub mainclass: Option<String>,

    /// Don't ask for confirmation before submitting.
    #[structopt(short = "f", long = "force")]
    pub force: bool,

    /// The hostname to submit to. The default is `open.kattis.com`.
    ///
    /// May be configured to another default in the configuration file.
    #[structopt(long = "hostname")]
    pub hostname: Option<String>,
}

#[derive(Debug, StructOpt)]
#[structopt(rename_all = "kebab-case")]
pub enum TemplateSubCommand {
    /// Create a new template.
    ///
    /// Prints the path to the new template.
    New {
        /// The name of the new template.
        name: String,
    },

    /// Print the names and paths of all templates.
    #[structopt(name = "show", alias = "list")]
    List,
}

#[derive(Debug, StructOpt)]
#[structopt(rename_all = "kebab-case")]
pub enum ConfigSubCommand {
    /// Show the path to the global configuration file.
    Show,

    /// Manage credentials. Additional credentials can be downloaded from
    /// http://<kattis>/download/kattisrc.
    Credentials(CredentialsSubCommand),
}

#[derive(Debug, StructOpt)]
#[structopt(rename_all = "kebab-case")]
pub enum CredentialsSubCommand {
    /// Print the names and paths of all credentials. Additional credentials can be downloaded from
    /// http://<kattis>/download/kattisrc.
    #[structopt(name = "show", alias = "list")]
    List,
}
