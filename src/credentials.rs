use failure::Fail;
use serde_derive::*;
use std::path::PathBuf;

use crate::config::*;
use crate::error::*;
use crate::util;

#[derive(Debug, Clone)]
pub struct Credentials {
    pub user: User,
    pub kattis: Kattis,
}

#[derive(Debug, Clone, Serialize)]
pub struct User {
    pub user: String,
    pub password: Option<String>,
    pub token: Option<String>,
}

#[derive(Debug, Clone)]
pub struct Kattis {
    pub hostname: String,
    pub loginurl: String,
    pub submissionurl: String,
    pub submissionsurl: String,
}

#[derive(Debug, Clone, Fail)]
pub enum CredentailsParseError {
    #[fail(display = "Missing section terminator")]
    MissingSectionTerminator,
    #[fail(display = "Missing field: {}", field)]
    MissingField { field: &'static str },
}

impl Credentials {
    pub fn directory() -> Result<PathBuf> {
        let home = Config::home_directory()?;
        let credentials_dir = home.join("credentials");

        Ok(credentials_dir)
    }

    /// Finds credentials for the credentials file matching the name
    pub fn find(name: &str) -> Result<Credentials> {
        let credentials_dir = Self::directory()?;

        let candidates = util::file_name_matches(name, &credentials_dir)?;

        if candidates.len() == 0 {
            return Err(Error::NoMatchingCredentials {
                name: name.to_owned(),
            });
        } else if candidates.len() > 1 {
            return Err(Error::MultipleCredentialCandidates {
                name: name.to_owned(),
            });
        } else {
            let path = candidates.into_iter().next().unwrap();
            let content = util::read_file(path)?;
            Credentials::parse(&content)
        }
    }

    pub fn parse(text: &str) -> Result<Credentials> {
        let mut username = None;
        let mut token = None;
        let mut password = None;
        let mut hostname = None;
        let mut loginurl = None;
        let mut submissionurl = None;
        let mut submissionsurl = None;

        let mut section = None;

        for line in text.lines() {
            let line = line.trim();
            let mut chars = line.chars();

            match chars.next() {
                // Comment
                Some(';') | Some('#') => continue,

                // Begin section
                Some('[') => {
                    if let Some(end) = line.find(']') {
                        section = Some(&line[1..end]);
                    } else {
                        Err(CredentailsParseError::MissingSectionTerminator)?;
                    }
                }

                _ => {
                    if let Some(assign) = line.find(|ch| ch == ':' || ch == '=') {
                        let key = &line[0..assign];
                        let value = if assign < line.len() {
                            line[assign + 1..].trim()
                        } else {
                            ""
                        };

                        match section {
                            Some("user") => match key {
                                "username" => username = Some(value),
                                "token" => token = Some(value),
                                "password" => password = Some(value),
                                _ => {}
                            },
                            Some("kattis") => match key {
                                "hostname" => hostname = Some(value),
                                "loginurl" => loginurl = Some(value),
                                "submissionurl" => submissionurl = Some(value),
                                "submissionsurl" => submissionsurl = Some(value),
                                _ => {}
                            },

                            _ => {}
                        }
                    }
                }
            }
        }

        let ok_or_missing = |value: Option<&str>, field| {
            value
                .map(|v| v.to_owned())
                .ok_or(CredentailsParseError::MissingField { field })
        };

        let credentials = Credentials {
            user: User {
                user: ok_or_missing(username, "username")?,
                password: password.map(|v| v.to_owned()),
                token: token.map(|v| v.to_owned()),
            },
            kattis: Kattis {
                hostname: ok_or_missing(hostname, "hostname")?,
                loginurl: ok_or_missing(loginurl, "loginurl")?,
                submissionurl: ok_or_missing(submissionurl, "submissionurl")?,
                submissionsurl: ok_or_missing(submissionsurl, "submissionsurl")?,
            },
        };

        Ok(credentials)
    }
}
