
use std::fs;
use std::io::Read;
use std::path::Path;

use crate::error::*;

pub fn read_file(path: impl AsRef<Path>) -> Result<String> {
    let mut file = fs::File::open(path)?;
    let mut string = String::new();
    file.read_to_string(&mut string)?;
    Ok(string)
}


pub mod serde_string {
    use std::fmt::Display;
    use std::str::FromStr;
    use serde::{de, Serializer, Deserialize, Deserializer};

    pub fn serialize<T, S>(value: &T, serializer: S) -> Result<S::Ok, S::Error>
    where
        T: Display,
        S: Serializer
    {
        serializer.collect_str(value)
    }

    pub fn deserialize<'de, T, D>(deserializer: D) -> Result<T, D::Error>
    where
        T: FromStr,
        T::Err: Display,
        D: Deserializer<'de>
    {
        String::deserialize(deserializer)?.parse().map_err(de::Error::custom)
    }
}

