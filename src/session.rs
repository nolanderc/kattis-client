use failure::Fail;
use regex::Regex;
use reqwest::{multipart, Client, StatusCode};
use serde_derive::*;
use std::convert::{TryFrom, TryInto};
use std::str::FromStr;

use select::document::Document;
use select::predicate::*;

use crate::config::Submission;
use crate::credentials::*;
use crate::error::*;

pub struct Session {
    client: Client,
    credentials: Credentials,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash, derive_more::Display)]
pub struct SubmissionId(u32);

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct SubmissionStatus {
    pub status: Status,
    pub cpu_time: String,
    pub date: String,
    pub test_cases: Vec<TestCase>,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct TestCase {
    pub status: Status,
    pub id: u32,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash, Deserialize, derive_more::Display)]
#[serde(from = "u8")]
pub enum Status {
    #[display(fmt = "New")]
    New,
    #[display(fmt = "Not Checked")]
    NotChecked,

    #[display(fmt = "Compiling")]
    Compiling,
    #[display(fmt = "Running")]
    Running,

    #[display(fmt = "Accepted")]
    Accepted,
    #[display(fmt = "Wrong Answer")]
    WrongAnswer,

    #[display(fmt = "Time Limit Exceeded")]
    TimeLimitExceeded,
    #[display(fmt = "Memory Limit Exceeded")]
    MemoryLimitExceeded,
    #[display(fmt = "Compile Error")]
    CompileError,
    #[display(fmt = "Run Time Error")]
    RunTimeError,

    #[display(fmt = "Other ({})", _0)]
    Other(u8),
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct SubmissionRow {
    component: String,
    status_id: Status,
    #[serde(rename = "testcases_number")]
    test_case_count: u32,
}

impl Session {
    pub fn new(hostname: &str) -> Result<Session> {
        let client = Client::builder().cookie_store(true).build()?;
        let credentials = Credentials::find(hostname)?;

        let session = Session {
            client,
            credentials,
        };

        Ok(session)
    }

    // We need the authentication cookies from Kattis in order to do anything
    fn login(&mut self) -> Result<()> {
        let creds = &self.credentials;

        let mut form = Vec::new();
        form.push(("user", creds.user.user.clone()));
        form.push(("script", "false".to_owned()));
        if let Some(password) = creds.user.password.clone() {
            form.push(("password", password));
        }
        if let Some(token) = creds.user.token.clone() {
            form.push(("token", token));
        }

        let response = self
            .client
            .post(&creds.kattis.loginurl)
            .form(&form)
            .send()?;

        let status = response.status();
        match status {
            StatusCode::OK => Ok(()),
            code => Err(Error::LoginFailed { code }),
        }
    }

    pub fn submit<'a>(&mut self, problem: &str, submission: Submission) -> Result<SubmissionId> {
        // FIXME: For some reason we have to log in again. Are the cookies somehow being deleted from
        // cookie store or invalidated?
        self.login()?;

        let mut form = multipart::Form::new()
            .text("submit", "true")
            .text("submit_ctr", "2")
            .text("language", format!("{}", submission.language))
            .text(
                "mainclass",
                submission.mainclass.clone().unwrap_or("".to_owned()),
            )
            .text("problem", problem.to_owned())
            .text("tag", "")
            .text("script", "true");

        for path in submission.files.iter() {
            let part = multipart::Part::file(path)?.mime_str("application/octet-stream")?;
            form = form.part("sub_file[]", part);
        }

        let submit_url = &self.credentials.kattis.submissionurl;
        let request = self.client.post(submit_url).multipart(form);
        let mut response = request.send()?;

        let status = response.status();

        match status {
            StatusCode::OK => {
                let text = response.text()?;
                let id = SubmissionId::extract_from_response(&text)?;
                Ok(id)
            }
            code => Err(Error::SubmitFailed { code }),
        }
    }

    pub fn submission_status(&mut self, id: SubmissionId) -> Result<SubmissionStatus> {
        // FIXME: For some reason we have to log in again. Are the cookies somehow being deleted from
        // cookie store or invalidated?
        self.login()?;

        let url = format!(
            "{base_url}/{id}?only_submission_row",
            base_url = self.credentials.kattis.submissionsurl,
            id = id,
        );

        let mut response = self.client.get(&url).send()?;

        let submission_row: SubmissionRow = response.json()?;
        let submission_status = submission_row.try_into()?;

        Ok(submission_status)
    }
}

impl SubmissionId {
    pub(self) fn extract_from_response(response: &str) -> Result<SubmissionId> {
        let re = Regex::new(r#"Submission received\. Submission ID: \d+\."#).unwrap();

        if !re.is_match(response) {
            Err(Error::SubmissionIdExtractFailed {
                response: response.to_owned(),
            })?;
        }

        let is_digit = |ch: char| ch.is_digit(10);
        let is_not_digit = |ch: char| !ch.is_digit(10);

        if let Some(id_start) = response.find(is_digit) {
            let trimmed = &response[id_start..];
            let id_end = trimmed.find(is_not_digit).unwrap_or(trimmed.len());
            let id = trimmed[..id_end]
                .parse()
                .expect("Could not parse submission id");
            Ok(SubmissionId(id))
        } else {
            Err(Error::SubmissionIdExtractFailed {
                response: response.to_owned(),
            })
        }
    }
}

impl SubmissionStatus {
    pub fn is_terminated(&self) -> bool {
        use Status::*;
        match self.status {
            Accepted | WrongAnswer | RunTimeError | CompileError | MemoryLimitExceeded | TimeLimitExceeded
            | Other(_) => true,
            Running | Compiling | New | NotChecked => false,
        }
    }
}

impl From<u8> for Status {
    fn from(byte: u8) -> Status {
        match byte {
            16 => Status::Accepted,

            byte => Status::Other(byte),
        }
    }
}

impl FromStr for Status {
    type Err = ParseSubmissionRowError;

    fn from_str(text: &str) -> std::result::Result<Status, Self::Err> {
        use Status::*;
        match text.to_lowercase().as_str() {
            "new" => Ok(New),
            "not checked" => Ok(NotChecked),
            "compiling" => Ok(Compiling),
            "running" => Ok(Running),
            "accepted" => Ok(Accepted),
            "wrong answer" => Ok(WrongAnswer),
            "time limit exceeded" => Ok(TimeLimitExceeded),
            "memory limit exceeded" => Ok(MemoryLimitExceeded),
            "compile error" => Ok(CompileError),
            "run time error" => Ok(RunTimeError),

            _ => Err(ParseSubmissionRowError::UnknownStatus {
                status: text.to_owned(),
            }),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Fail)]
pub enum ParseSubmissionRowError {
    #[fail(display = "Submission contained no status.")]
    StatusMissing,
    #[fail(display = "Submission contained no CPU time.")]
    CpuTimeMissing,
    #[fail(display = "Submission contained no date.")]
    DateMissing,
    #[fail(display = "Submission contained no test cases.")]
    TestCasesMissing,
    #[fail(display = "Test case contained invalid title")]
    InvalidTestCaseTitle,

    #[fail(display = "Unkown status: {:?}", _0)]
    UnknownStatus { status: String },
}

impl TryFrom<SubmissionRow> for SubmissionStatus {
    type Error = ParseSubmissionRowError;

    fn try_from(row: SubmissionRow) -> std::result::Result<SubmissionStatus, Self::Error> {
        // We need to add all these excess tags so that it can be parsed as valid HTML.
        let html = format!("<html><body><table>{}</table></body></html>", row.component);

        let root = Document::from(html.as_str());

        let status = root
            .find(Name("td").and(Attr("data-type", "status")))
            .next()
            .ok_or(ParseSubmissionRowError::StatusMissing)?
            .text()
            .trim()
            .parse()?;

        let cpu_time = root
            .find(Name("td").and(Attr("data-type", "cpu")))
            .next()
            .ok_or(ParseSubmissionRowError::CpuTimeMissing)?
            .text()
            .trim()
            .to_owned();

        let date = root
            .find(Name("td").and(Attr("data-type", "time")))
            .next()
            .ok_or(ParseSubmissionRowError::DateMissing)?
            .text()
            .trim()
            .to_owned();

        let test_cases = root
            .find(Name("div").and(Class("testcases")))
            .next()
            .ok_or(ParseSubmissionRowError::TestCasesMissing)
            .and_then(|testcases| {
                testcases
                    .children()
                    .filter_map(|test_case| test_case.attr("title"))
                    .try_fold(Vec::new(), |mut acc, title| {
                        let test_case = TestCase::from_title(title.trim())?;
                        acc.push(test_case);
                        Ok(acc)
                    })
            })?;

        let submission_status = SubmissionStatus {
            status,
            cpu_time,
            date,
            test_cases,
        };

        Ok(submission_status)
    }
}

impl TestCase {
    pub fn from_title(title: &str) -> std::result::Result<TestCase, ParseSubmissionRowError> {
        let re = Regex::new(r#"Test case \d+/\d+: .+"#).unwrap();
        if !re.is_match(title) {
            Err(ParseSubmissionRowError::InvalidTestCaseTitle)?;
        }

        let id_start = "Test case ".len();
        let id_end = title.find('/').unwrap();
        let id = title[id_start..id_end].parse().unwrap();

        let status_start = title.find(':').unwrap() + ": ".len();
        let status = title[status_start..].trim().parse()?;

        Ok(TestCase {
            id,
            status,
        })
    }
}
