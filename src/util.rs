use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};
use regex::Regex;

use crate::error::*;

pub fn read_file(path: impl AsRef<Path>) -> Result<String> {
    let mut file = fs::File::open(path)?;
    let mut string = String::new();
    file.read_to_string(&mut string)?;
    Ok(string)
}

pub fn file_name_matches(name: &str, directory: impl AsRef<Path>) -> Result<Vec<PathBuf>> {
    let re = Regex::new(name)?;

    let mut candidates = Vec::new();
    for entry in fs::read_dir(directory)? {
        let entry = entry?;
        if let Some(name) = entry.file_name().to_str() {
            if re.is_match(name) {
                candidates.push(entry.path());
            }
        }
    }

    Ok(candidates)
}

pub mod serde_string {
    use serde::{de, Deserialize, Deserializer, Serializer};
    use std::fmt::Display;
    use std::str::FromStr;

    pub fn serialize<T, S>(value: &T, serializer: S) -> Result<S::Ok, S::Error>
    where
        T: Display,
        S: Serializer,
    {
        serializer.collect_str(value)
    }

    pub fn deserialize<'de, T, D>(deserializer: D) -> Result<T, D::Error>
    where
        T: FromStr,
        T::Err: Display,
        D: Deserializer<'de>,
    {
        String::deserialize(deserializer)?
            .parse()
            .map_err(de::Error::custom)
    }
}
