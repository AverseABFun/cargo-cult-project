use serde::Deserialize;
use std::{collections::HashMap, fs, path::Path};

pub mod copied;
pub mod resources;
pub mod targets;
pub mod tests;

#[derive(Deserialize, Debug, Clone)]
#[serde(rename_all = "kebab-case")]
pub struct Toolchain {
    pub edition: String,
    pub channel: String,
    pub profile: String,
    pub components: Vec<String>,
    pub crate_id: String,
    pub platforms: Vec<String>,
    pub targets: Vec<String>,
    pub format_map: HashMap<String, String>,
}

#[derive(Debug, Clone)]
pub enum Suffix {
    IfAvailable,
    Only,
}

struct StringVisitor;

impl<'de> serde::de::Visitor<'de> for StringVisitor {
    type Value = String;
    fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        formatter.write_str("any UTF-8 encoded string")
    }
    fn visit_string<E>(self, v: String) -> Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        Ok(v)
    }
    fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        Ok(v.to_string())
    }
}

#[derive(Debug, Clone)]
pub struct Format {
    pub format: String,
    pub suffix: Suffix,
}

impl<'de> Deserialize<'de> for Format {
    fn deserialize<D>(deserializer: D) -> Result<Format, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let base_string = deserializer.deserialize_str(StringVisitor)?;
        let split = base_string.split_once("-").unwrap_or((&base_string, ""));
        let suffix = split.1.to_string();
        let real_suffix = match suffix.as_str() {
            "-only" => Suffix::Only,
            _ => Suffix::IfAvailable,
        };
        Ok(Format {
            format: split.0.to_string(),
            suffix: real_suffix,
        })
    }
}

#[derive(Deserialize, Debug, Clone)]
#[serde(untagged)]
pub enum Crate {
    Version(String),
    Detailed {
        version: Option<String>,
        features: Option<Vec<String>>,
        path: Option<String>,
        git: Option<String>,
    },
}

impl Crate {
    pub fn serialize(self) -> String {
        if let Crate::Version(str) = self {
            return str;
        } else {
            let Crate::Detailed {
                version,
                features,
                path,
                git,
            } = self
            else {
                unreachable!();
            };
            let mut out = String::new();
            if let Some(version) = version {
                out += &format!("version = {},", version);
            }
            if let Some(mut features) = features {
                for ele in &mut features {
                    *ele = format!("\"{}\"", ele);
                }
                out += &format!("features = [{}],", features.join(", "));
            }
            if let Some(path) = path {
                out += &format!("path = {},", path);
            }
            if let Some(git) = git {
                out += &format!("git = {},", git);
            }
            out = format!("{{ {} }}", out);
            return out;
        }
    }
}

pub type Crates = HashMap<String, HashMap<String, Crate>>;

#[derive(Deserialize, Debug, Clone)]
pub struct RustConfigInner {
    pub toolchains: Vec<Toolchain>,
    pub crates: Crates,
    pub formats: HashMap<String, Vec<Format>>,
}

pub type RustConfig = HashMap<String, RustConfigInner>;

pub fn parse_file(path: &Path) -> RustConfig {
    toml::from_str(&fs::read_to_string(path).unwrap()).unwrap()
}
