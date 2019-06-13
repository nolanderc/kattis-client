#[macro_use]
mod macros;

mod args;
mod config;
mod credentials;
mod error;
mod language;
mod util;

use reqwest::{multipart, Client, StatusCode};
use std::collections::HashMap;
use std::fs;
use std::io::{Cursor, Read, Write};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::str::from_utf8;
use structopt::StructOpt;
use zip::ZipArchive;

use crate::args::*;
use crate::config::*;
use crate::credentials::*;
use crate::error::*;

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

#[derive(Debug, Clone, PartialEq, Eq, Hash, derive_more::Display)]
struct SubmissionId(u32);

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
            let hostname = args.hostname.as_ref().unwrap_or(&config.default_hostname);

            assert_problem_exists(hostname, &command.problem)?;

            let samples = Sample::download(hostname, &command.problem)?;

            for sample in samples {
                sample.save_in(&command.directory)?;
            }
        }

        SubCommand::New(command) => {
            let hostname = args.hostname.as_ref().unwrap_or(&config.default_hostname);

            let template = Template::find(command.template, &config)?;

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
                Err(Error::DownloadSample { code: StatusCode::NOT_FOUND }) => {
                    warn!("No samples found for problem.")
                }
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

        SubCommand::Test(TestSolution { directory }) => {
            let solution_config = SolutionConfig::load(&directory)?;

            build_solution(&directory, &solution_config.build)?;

            let sample_dir = if solution_config.samples.is_relative() {
                directory.join(&solution_config.samples)
            } else {
                solution_config.samples
            };

            if !sample_dir.is_dir() {
                return Err(Error::SampleDirectoryNotFound { path: sample_dir });
            }

            let samples = TestCase::load(&sample_dir)?;

            test_solution(&directory, &solution_config.run, &samples)?;
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

            let templates = Template::list(templates_dir)?;

            if templates.len() > 0 {
                let max_len = templates
                    .iter()
                    .map(|template| template.name.chars().count())
                    .max()
                    .unwrap();

                for template in templates {
                    let chars = template.name.chars().count();
                    print!("{}", template.name);
                    for _ in chars..max_len + 2 {
                        print!(" ")
                    }

                    let path_name = template.path.into_os_string().into_string();
                    if let Ok(path) = path_name {
                        println!("{}", path);
                    } else {
                        println!();
                    }
                }
            }
        }

        SubCommand::Submit(submit) => {
            let solution_config = SolutionConfig::load(&submit.directory)?;

            let hostname = args.hostname.unwrap_or(solution_config.hostname);

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

            // TODO: ask for confirmation

            let mut client = Client::builder()
                .cookie_store(true)
                .build()?;

            let credentials = Credentials::load(&hostname)?;
            login(&mut client, &credentials)?;

            let submit_url = &credentials.kattis.submissionurl;
            let submission_id = submit_files(&mut client, submit_url, &problem, &submission)?;
            println!("Submission ID: {}", submission_id);

            // TODO: track progress or, if configured, ask to open in browser
        }
    }

    Ok(())
}

// We need the authentication cookies from Kattis in order to submit
fn login(client: &mut Client, creds: &Credentials) -> Result<()> {
    let mut form = HashMap::new();
    form.insert("user", creds.user.user.clone());
    form.insert("script", "false".to_owned());
    if let Some(password) = creds.user.password.clone() {
        form.insert("password", password);
    }
    if let Some(token) = creds.user.token.clone() {
        form.insert("token", token);
    }

    let response = client
        .post(&creds.kattis.loginurl)
        .form(&form)
        .send()?;

    let status = response.status();
    match status {
        StatusCode::OK => Ok(()),
        code => Err(Error::LoginFailed { code }),
    }
}

fn submit_files<'a>(
    client: &mut Client,
    url: &str,
    problem: &str,
    submission: &Submission
) -> Result<SubmissionId> {
    let mut form = multipart::Form::new()
        .text("submit", "true")
        .text("submit_ctr", "2")
        .text("language", format!("{}", submission.language))
        .text("mainclass", submission.mainclass.clone().unwrap_or("".to_owned()))
        .text("problem", problem.to_owned())
        .text("tag", "")
        .text("script", "true");

    for path in submission.files.iter() {
        let part = multipart::Part::file(path)?
            .mime_str("application/octet-stream")?;
        form = form.part("sub_file[]", part);
    }
    
    let request = client.post(url)
        .multipart(form);

    let mut response = request.send()?;

    let status = response.status();

    match status {
        StatusCode::OK => {
            let text = response.text()?;
            let id = SubmissionId::extract_from_response(&text)?;
            Ok(id)
        },
        code => Err(Error::SubmitFailed { code }),
    }
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

    let mut term = term::stdout().unwrap();
    for case in cases {
        term.attr(term::Attr::Bold).unwrap();
        println!("Running test case: {}", case.name);
        term.reset().unwrap();

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
        let output = Command::new("sh")
            .arg("-c")
            .arg(final_run_command)
            .current_dir(&current_dir)
            .stdin(fs::File::open(&case.input)?)
            .output()?;

        if !output.status.success() {
            let error = Error::RunCommandFailed {
                command: final_run_command.clone(),
            };
            error!("{}", error);
        } else {
            let output = from_utf8(&output.stdout).map_err(Error::InvalidUtf8Answer)?;
            let expected = util::read_file(&case.answer)?;

            if output == expected {
                term.fg(term::color::GREEN).unwrap();
                println!("Correct");
                term.reset().unwrap();
            } else {
                term.fg(term::color::RED).unwrap();
                println!("Wrong Answer");
                term.reset().unwrap();

                let input = util::read_file(&case.input)?;

                println!("Input:\n{}", input);
                println!("Found:\n{}", output);
                println!("Expected:\n{}", expected);
            }
        }
    }

    Ok(())
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
    pub fn load(path: impl AsRef<Path>) -> Result<Vec<TestCase>> {
        let mut sets = HashMap::new();

        for entry in fs::read_dir(path)? {
            let entry = entry?;
            let path = entry.path();

            if path.is_file() {
                if let Some(name) = path.file_stem().and_then(|n| n.to_str()) {
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

    pub fn find(name: Option<String>, config: &Config) -> Result<Template> {
        let name = name
            .or_else(|| config.default_template.clone())
            .ok_or(Error::TemplateNotSpecified)?;

        let template_dir = Template::dir()?.join(&name);

        if !template_dir.is_dir() {
            Err(Error::TemplateNotFound { path: template_dir })
        } else {
            Ok(Template {
                name,
                path: template_dir,
            })
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

    fn list(directory: impl AsRef<Path>) -> Result<Vec<Template>> {
        let mut templates = Vec::new();

        for entry in fs::read_dir(&directory)? {
            let entry = entry?;
            let path = entry.path();
            if path.is_dir() {
                let file_name = entry.file_name().into_string();

                if let Ok(name) = file_name {
                    templates.push(Template { name, path });
                }
            }
        }

        Ok(templates)
    }
}

impl SubmissionId {
    pub fn extract_from_response(response: &str) -> Result<SubmissionId> {
        let is_digit = |ch: char| ch.is_digit(10);
        let is_not_digit = |ch: char| !ch.is_digit(10);

        if let Some(id_start) = response.find(is_digit) {
            let trimmed = &response[id_start..];
            let id_end = trimmed.find(is_not_digit).unwrap_or(trimmed.len());
            let id = trimmed[..id_end].parse()
                .expect("Could not parse submission id");
            Ok(SubmissionId(id))
        } else {
            Err(Error::SubmissionIdExtractFailed { response: response.to_owned() })
        }
    }
}
