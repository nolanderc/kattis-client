#[macro_use]
mod macros;

mod args;
mod config;
mod credentials;
mod error;
mod language;
mod query;
mod session;
mod util;

use crossterm::{style, Color, Colorize, Styler};
use notify::{watcher, RecursiveMode, Watcher};
use reqwest::StatusCode;
use std::collections::{HashMap, HashSet};
use std::fs;
use std::io::{Cursor, Read, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::str::from_utf8;
use std::sync::mpsc::channel;
use std::time::{Duration, Instant};
use structopt::StructOpt;
use zip::ZipArchive;

use crate::args::*;
use crate::config::*;
use crate::credentials::Credentials;
use crate::error::*;
use crate::query::{Response as QueryResponse, *};
use crate::session::*;

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

#[derive(Debug, Clone)]
struct Template {
    name: String,
    path: PathBuf,
}

fn main() {
    let args = Args::from_args();

    match execute(args) {
        Ok(()) => {}
        Err(e) => {
            error!("{}", e);
        }
    }
}

fn execute(args: Args) -> Result<()> {
    let config_home = Config::home_directory()?;
    let config = Config::load(&config_home)?;

    match args.command {
        SubCommand::Samples(command) => {
            let hostname = command
                .hostname
                .as_ref()
                .unwrap_or(&config.default_hostname);

            assert_problem_exists(hostname, &command.problem)?;

            let samples = Sample::download(hostname, &command.problem)?;

            for sample in samples {
                sample.save_in(&command.directory)?;
            }
        }

        SubCommand::New(command) => {
            let hostname = command
                .hostname
                .as_ref()
                .unwrap_or(&config.default_hostname);

            let template_name = command
                .template
                .or_else(|| config.default_template.clone())
                .ok_or(Error::TemplateNotSpecified)?;
            let template = Template::find(template_name)?;

            let directory = match command.directory {
                Some(dir) => dir,
                None => PathBuf::new().join(&command.problem),
            };

            if directory.is_dir() {
                return Err(Error::SolutionDirectoryExists { path: directory });
            }

            // Before we do any visible changes to the user, make sure the problem actually
            // exists and that the template files are valid.
            assert_problem_exists(hostname, &command.problem)?;
            let template_config = TemplateSolutionConfig::load_or_default(&template.path)?;

            fs::create_dir(&directory)?;

            template.init_dir(&directory)?;

            let solution_config = SolutionConfig::from_template(
                template_config,
                command.problem.to_owned(),
                hostname.to_owned(),
            );
            solution_config.save_in(&directory)?;

            match Sample::download(hostname, &command.problem) {
                Err(Error::DownloadSample {
                    code: StatusCode::NOT_FOUND,
                }) => warn!("No samples found for problem."),
                Err(e) => warn!("{}", e),
                Ok(samples) => {
                    let sample_dir = if solution_config.samples.is_relative() {
                        directory.join(&solution_config.samples)
                    } else {
                        solution_config.samples
                    };

                    if !sample_dir.is_dir() {
                        fs::create_dir(&sample_dir)?;
                    }

                    for sample in samples {
                        sample.save_in(&sample_dir)?;
                    }
                }
            }
        }

        SubCommand::Test(TestSolution {
            directory,
            watch,
            clear,
            ignore,
            filter,
        }) => {
            let solution_config = SolutionConfig::load(&directory)?;

            let sample_dir = if solution_config.samples.is_relative() {
                directory.join(&solution_config.samples)
            } else {
                solution_config.samples.clone()
            };

            if !sample_dir.is_dir() {
                return Err(Error::SampleDirectoryNotFound { path: sample_dir });
            }

            let test_samples = || -> Result<()> {
                let samples = TestCase::load(&sample_dir, |name| {
                    let pass_filter = filter.as_ref().map(|f| f.is_match(name)).unwrap_or(true);
                    let is_ignored = ignore.as_ref().map(|i| i.is_match(name)).unwrap_or(false);

                    pass_filter && !is_ignored
                })?;

                if clear {
                    Command::new("clear").status()?;
                }

                build_solution(&directory, &solution_config.build)?;

                if clear {
                    Command::new("clear").status()?;
                }

                test_solution(&directory, &solution_config.run, &samples)?;

                Ok(())
            };

            if watch {
                let (tx, rx) = channel();
                let mut watcher = watcher(tx, Duration::from_secs(1))?;

                for file in &solution_config.submission.files {
                    watcher.watch(file, RecursiveMode::NonRecursive)?;
                }

                watcher.watch(&sample_dir, RecursiveMode::Recursive)?;

                loop {
                    if let Err(e) = test_samples() {
                        error!("{}", e);
                    }

                    match rx.recv() {
                        Ok(_) => {}
                        Err(_) => break,
                    }
                }
            } else {
                test_samples()?;
            }
        }

        SubCommand::Template(TemplateSubCommand::New { name }) => {
            let template_dir = Template::dir()?.join(name);

            if template_dir.exists() {
                return Err(Error::TemplateDirectoryExists { path: template_dir });
            }

            fs::create_dir(&template_dir)?;

            let config = TemplateSolutionConfig::default();
            config.save_in(&template_dir)?;

            if let Some(text) = template_dir.to_str() {
                eprint!("Created template: ");
                println!("{}", text);
            } else {
                warn!("Template path does not contain valid unicode, printing lossy version...");

                println!("{}", template_dir.to_string_lossy());
            }
        }

        SubCommand::Template(TemplateSubCommand::List) => {
            let templates_dir = Template::dir()?;

            let matches = util::file_name_matches(".*", templates_dir)?;
            let templates = matches.iter().filter(|path| path.is_dir());

            list_path_filenames(templates);
        }

        SubCommand::Submit(submit) => {
            let solution_config = SolutionConfig::load(&submit.directory)?;

            let problem = solution_config.problem;
            let files = solution_config
                .submission
                .files
                .iter()
                .map(|path| submit.directory.join(path))
                .collect::<Vec<_>>();

            // TODO: guess language and mainclass from files
            let language = submit
                .language
                .unwrap_or(solution_config.submission.language);
            let mainclass = submit.mainclass.or(solution_config.submission.mainclass);

            let submission = Submission {
                files,
                language,
                mainclass,
            };

            print_submission(&submission);

            if submit.force || confirm_submission() == QueryResponse::Yes {
                let hostname = submit.hostname.unwrap_or(solution_config.hostname);
                let mut session = Session::new(&hostname)?;

                let submission_id = session.submit(&problem, submission)?;
                println!("Submission ID: {}", submission_id);

                // TODO: if configured, (ask to) open in browser instead
                track_submission_progress(&mut session, submission_id)?;
            } else {
                println!("Cancelled submission.");
            }
        }

        SubCommand::Config(ConfigSubCommand::Show) => {
            println!("{}", Config::file_path()?.display())
        }

        SubCommand::Config(ConfigSubCommand::Credentials(CredentialsSubCommand::List)) => {
            let dir = Credentials::directory()?;

            let matches = util::file_name_matches(".*", dir)?;
            let files = matches.iter().filter(|path| path.is_file());

            list_path_filenames(files);
        }
    }

    Ok(())
}

fn print_submission(submission: &Submission) {
    println!("Language: {}", submission.language);

    println!("Files:");
    for file in &submission.files {
        println!("  - {}", file.display());
    }

    let main = submission
        .mainclass
        .as_ref()
        .map(|m| m.as_str())
        .unwrap_or("");
    println!("Main Class: {}", main);
}

fn confirm_submission() -> QueryResponse {
    let response = Query::new("Proceed with the submission?")
        .default(QueryResponse::No)
        .confirm();

    response
}

/// Track the submission process by repeatedly polling the submissions page and printing the result
/// until either:
/// - One of the test cases fail
/// - All test cases are successful
fn track_submission_progress(session: &mut Session, id: SubmissionId) -> Result<()> {
    let mut displayed_cases = HashSet::new();

    let display_status = |status: Status| {
        let color = if status == Status::Accepted {
            Color::Green
        } else {
            Color::Red
        };

        eprintln!("{}", style(status).bold().with(color));
    };

    loop {
        let submission = session.submission_status(id)?;

        for test_case in &submission.test_cases {
            let checked = test_case.status != Status::NotChecked;
            let not_displayed = !displayed_cases.contains(test_case);

            if checked && not_displayed {
                eprint!(
                    "Test Case {id}/{count}: ",
                    id = test_case.id,
                    count = submission.test_cases.len()
                );

                displayed_cases.insert(test_case.clone());
                display_status(test_case.status);
            }
        }

        if displayed_cases.is_empty() {
            eprintln!("{}...", submission.status);
        }

        if submission.is_terminated() {
            eprintln!();

            eprint!("Submission Status: ");
            display_status(submission.status);

            eprintln!("Time: {}", submission.date);
            eprintln!("CPU: {}", submission.cpu_time);

            // TODO: if there was a compile error, get the build log.

            break;
        }

        std::thread::sleep(std::time::Duration::from_millis(100));
    }

    Ok(())
}

fn assert_problem_exists(hostname: &str, problem: &str) -> Result<()> {
    if problem_exists(hostname, problem)? {
        Ok(())
    } else {
        // TODO: list problems with similar names
        Err(Error::ProblemNotFound {
            problem: problem.to_owned(),
        })
    }
}

fn problem_exists(hostname: &str, problem: &str) -> Result<bool> {
    let url = format!(
        "https://{hostname}/problems/{problem}",
        hostname = hostname,
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
            Err(Error::BuildCommandFailed {
                command: command.clone(),
            })?;
        }
    }

    Ok(())
}

fn test_solution(
    directory: impl AsRef<Path>,
    run_commands: &[String],
    cases: &[TestCase],
) -> Result<()> {
    let current_dir = directory.as_ref().canonicalize()?;

    let n_commands = run_commands.len();
    if n_commands == 0 {
        Err(Error::RunCommandsMissing)?;
    }

    for case in cases {
        println!("Running test case: {}", style(&case.name).bold());

        if n_commands > 1 {
            for command in run_commands[..n_commands - 1].iter() {
                let status = Command::new("sh")
                    .arg("-c")
                    .arg(command)
                    .current_dir(&current_dir)
                    .status()?;

                if !status.success() {
                    Err(Error::BuildCommandFailed {
                        command: command.clone(),
                    })?;
                }
            }
        }

        let final_run_command = &run_commands[n_commands - 1];

        // TODO: measure CPU time instead of real time.
        let before = Instant::now();
        let output = Command::new("sh")
            .arg("-c")
            .arg(final_run_command)
            .current_dir(&current_dir)
            .stdin(fs::File::open(&case.input)?)
            .stderr(Stdio::inherit())
            .output()?;
        let after = Instant::now();

        let duration = after - before;
        let seconds = duration.as_micros() as f64 * 1e-6;

        if !output.status.success() {
            let error = Error::RunCommandFailed {
                command: final_run_command.clone(),
            };
            error!("{}", error);
        } else {
            let answer = from_utf8(&output.stdout).map_err(Error::InvalidUtf8Answer)?;
            let expected = util::read_file(&case.answer)?;

            println!("Time: {:.6}", seconds);

            if fuzzy_str_eq(&answer, &expected) {
                println!("{}", "Correct".green());
            } else {
                println!("{}", "Wrong Answer".red());

                let input = util::read_file(&case.input)?;

                println!();
                println!("Input:\n{}", input);
                println!("Found:\n{}", answer);
                println!("Expected:\n{}", expected);
            }
        }
    }

    Ok(())
}

/// Compare two strings, returning true if they are equal when all whitespace is stripped from the
/// end of all lines.
fn fuzzy_str_eq(a: &str, b: &str) -> bool {
    let trim = str::trim_end;

    let lines_a = trim(a).lines().map(trim);
    let lines_b = trim(b).lines().map(trim);

    lines_a.eq(lines_b)
}

fn list_path_filenames<'a>(paths: impl IntoIterator<Item = &'a PathBuf>) {
    let paths = paths
        .into_iter()
        .map(|path| (path.file_name().and_then(|name| name.to_str()), path))
        .collect::<Vec<_>>();

    if paths.len() > 0 {
        let max_len = paths
            .iter()
            .filter_map(|(name, _)| name.as_ref())
            .map(|name| name.chars().count())
            .max()
            .unwrap();

        for (name, path) in paths {
            let chars = if let Some(name) = name {
                print!("{}", name);
                name.chars().count()
            } else {
                0
            };

            for _ in chars..max_len + 2 {
                print!(" ")
            }

            println!("{}", path.display());
        }
    }
}

impl Sample {
    fn download(hostname: &str, problem: &str) -> Result<Vec<Sample>> {
        let url = format!(
            "https://{hostname}/problems/{problem}/file/statement/samples.zip",
            hostname = hostname,
            problem = problem
        );

        let mut res = reqwest::get(&url)?;

        let mut archive = if res.status().is_success() {
            let mut buffer = Vec::new();
            res.read_to_end(&mut buffer)?;
            ZipArchive::new(Cursor::new(buffer))?
        } else {
            Err(Error::DownloadSample { code: res.status() })?
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

    pub fn save_in(&self, path: impl AsRef<Path>) -> Result<()> {
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

impl TestCase {
    /// Load samples which names pass a predicate.
    pub fn load<F>(path: impl AsRef<Path>, mut predicate: F) -> Result<Vec<TestCase>>
    where
        F: FnMut(&str) -> bool,
    {
        let mut sets = HashMap::new();

        for entry in fs::read_dir(path)? {
            let entry = entry?;
            let path = entry.path();

            if path.is_file() {
                if let Some(name) = path.file_stem().and_then(|n| n.to_str()) {
                    if predicate(name) {
                        let name = name.to_owned();

                        let extension = path.extension();
                        let extension_is = |ext: &str| extension.filter(|e| *e == ext).is_some();

                        if extension_is("in") {
                            sets.entry(name).or_insert((None, None)).0 = Some(path);
                        } else if extension_is("ans") {
                            sets.entry(name).or_insert((None, None)).1 = Some(path);
                        }
                    }
                }
            }
        }

        let mut test_cases: Vec<_> = sets
            .into_iter()
            .filter_map(|(name, pair)| match pair {
                (Some(input), Some(answer)) => Some(TestCase {
                    name,
                    input,
                    answer,
                }),
                _ => None,
            })
            .collect();

        test_cases.sort_by(|a, b| a.name.cmp(&b.name));

        Ok(test_cases)
    }
}

impl Template {
    pub fn dir() -> Result<PathBuf> {
        let dir = Config::home_directory()?.join("templates");
        Ok(dir)
    }

    pub fn find(name: String) -> Result<Template> {
        let template_dir = Template::dir()?;

        let candidates = util::file_name_matches(&name, &template_dir)?;

        if candidates.is_empty() {
            Err(Error::NoMatchingTemplate { name })
        } else if candidates.len() > 1 {
            Err(Error::MultipleTemplateCandidates { name })
        } else {
            let template = candidates.into_iter().next().unwrap();

            if !template.is_dir() {
                Err(Error::TemplateNotDirectory { path: template })
            } else {
                Ok(Template {
                    name,
                    path: template,
                })
            }
        }
    }

    pub fn init_dir(&self, target: impl AsRef<Path>) -> Result<()> {
        let mut template_items = Vec::new();
        for entry in fs::read_dir(&self.path)? {
            template_items.push(entry?.path());
        }

        let options = fs_extra::dir::CopyOptions {
            overwrite: false,
            skip_exist: true,
            buffer_size: 64000,
            copy_inside: false,
            depth: 0,
        };

        fs_extra::copy_items(&template_items, target, &options)?;

        Ok(())
    }
}
