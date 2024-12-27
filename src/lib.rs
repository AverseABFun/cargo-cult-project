#![warn(missing_docs)]

//! Main file for the crate. Contains most of the public API,
//! outside of mainly the code copied from [rustup-mirror](https://github.com/jiegec/rustup-mirror)
//! and the resources included in the output.

use anyhow::{anyhow, Error};
use serde::Deserialize;
use std::{collections::HashMap, fs, path::Path};

pub mod copied;
pub mod resources;
pub mod targets;
#[cfg(test)]
mod tests;

#[derive(Deserialize, Debug, Clone)]
#[serde(rename_all = "kebab-case")]
/// Contains all relevant information for a toolchain
///
/// `rust-config.toml` contains a list of these under `toolchains`
/// in each config.
pub struct Toolchain {
    /// The edition of rust to use. Should be one of 2015, 2018, 2021, or 2024,
    /// but isn't directly validated. Validation could be changed in the future.
    ///
    /// Deprecated as it's pretty much unnecessary
    #[deprecated(since = "1.2.0")]
    pub edition: Option<String>,
    /// The channel of rust to use. Should be in [`targets::RELEASE_CHANNELS`].
    pub channel: String,
    /// The components of rust to install.
    pub components: Vec<String>,
    /// The ID used to index into the [`Crates`] instance associated with the
    /// rust config(technically [`RustConfigInner`], but whatever).
    pub crate_id: String,
    /// The list of targets to provide the rust components for.
    pub platforms: Vec<String>,
    /// The list of targets to allow the [`platforms`](Toolchain::platforms) to build for.
    pub targets: Vec<String>,
    /// A map of [`platforms`](Toolchain::platforms) to format IDs. Format IDs are used to
    /// index into the [rust config's format list](RustConfigInner::formats).
    pub format_map: HashMap<String, String>,
}

#[derive(Debug, Clone, PartialEq)]
/// The suffix used for [Formats](Format).
pub enum Suffix {
    /// Use the format if it's available,
    /// but continue if it's not.
    IfAvailable,
    /// Use the provided format or stop building the package.
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

#[derive(Debug, Clone, PartialEq)]
/// A format that includes one of the allowed [formats](FORMATS)
/// and a [`Suffix`].
pub struct Format {
    /// The actual format. One of [`FORMATS`].
    pub format: String,
    /// The suffix. See [the type documentation](Suffix) for information about the valid values.
    pub suffix: Suffix,
}

/// Slice of the formats that can be used in a [`Format`].
pub const FORMATS: [&str; 4] = ["msi", "pkg", "gz", "xz"];

impl Format {
    /// this function is a basic wrapper around and thus
    /// has the same semantic meaning as [`Format::from_string`]
    pub fn from_str(base_string: &str) -> Result<Format, Error> {
        Format::from_string(base_string.to_string())
    }
    /// this function is a basic wrapper around and thus
    /// has the same semantic meaning as [`Format::from_string_no_err`]
    pub fn from_str_no_err(base_string: &str) -> Format {
        Format::from_string_no_err(base_string.to_string())
    }
    /// this function takes in a `String` and produces a [`Format`] if it's
    /// valid. if not, it returns an [`anyhow::Error`].
    ///
    /// any valid format has to match the regex:
    ///
    /// `(?:(?:msi)|(?:pkg)|(?:gz)|(?:xz))(?:(?:-if-available)|(?:-only))?`
    ///
    /// so `msi` or `gz-only` are valid but `dfsjj-only` isn't
    ///
    /// (the mentioned regex is not internally used)
    ///
    /// [`FORMATS`] is a slice of valid formats(msi, pkg, gz, and xz)
    ///
    /// you can use [`Format::from_str`] if you want to convert a `&str`
    /// to a Format, or [`Format::from_string_no_err`] if your application
    /// requires that there not be an error case.
    ///
    /// the reason you wouldn't just call [`Result::unwrap`] to remove the
    /// error case is that `from_string_no_err` also allows cases that
    /// this method would not. (for example, `from_string_no_err`
    /// allows the aforementioned case of `dfsjj-only` or even
    /// `dfsjj-ndfdsdf`)
    pub fn from_string(base_string: String) -> Result<Format, Error> {
        let split = base_string.split_once("-").unwrap_or((&base_string, ""));
        let suffix = split.1.to_string();
        let real_suffix = match suffix.as_str() {
            "only" => Suffix::Only,
            "" | "if-available" => Suffix::IfAvailable,
            _ => return Err(anyhow!("invalid suffix {}", suffix)),
        };
        if !FORMATS.contains(&split.0) {
            return Err(anyhow!("invalid format {}", split.0));
        }
        Ok(Format {
            format: split.0.to_string(),
            suffix: real_suffix,
        })
    }
    /// see [`Format::from_string`] for the usage, this is virtually
    /// identical however doesn't return a `Result`.
    ///
    /// note that this method will report a value for any string(so anything
    /// that matches `.*`) but will divide on the first occurance of `-`.
    /// see the following code:
    ///
    /// ```
    /// # use rust_pkg_gen::{Format,Suffix};
    /// let test_string = "test-ah-yes".to_string();
    /// assert_eq!(Format::from_string_no_err(test_string),
    ///     Format {
    ///         format: "test".to_string(),
    ///         suffix: Suffix::IfAvailable
    ///     }
    /// );
    /// ```
    ///
    /// keep this in mind when using this method. thus the regex `([\w&&[^-]]+)((?:-.*)?)`
    /// matches the output where capture group one is the format and capture group two
    /// is the suffix. however, if the suffix is not "only", then it will output
    /// [`Suffix::IfAvailable`].
    ///
    /// THIS REGEX MAY OR MAY NOT BE KEPT UP-TO-DATE AS IT IS NOT USED INTERNALLY
    ///
    /// the reason that this method was implemented was to solve the problem
    /// of constructing an error for Format::deserialize.
    ///
    /// I don't like that we had to implement it, but it's kind of necessary.
    /// If you have any ideas, please please please open an issue or PR!
    pub fn from_string_no_err(base_string: String) -> Format {
        let split = base_string.split_once("-").unwrap_or((&base_string, ""));
        let suffix = split.1.to_string();
        let real_suffix = match suffix.as_str() {
            "only" => Suffix::Only,
            _ => Suffix::IfAvailable,
        };
        Format {
            format: split.0.to_string(),
            suffix: real_suffix,
        }
    }
}

impl<'de> Deserialize<'de> for Format {
    /// Deserialize this value from the given Serde deserializer.
    fn deserialize<D>(deserializer: D) -> Result<Format, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        Ok(Format::from_string_no_err(
            deserializer.deserialize_str(StringVisitor)?,
        ))
    }
}

#[derive(Deserialize, Debug, Clone)]
#[serde(untagged)]
/// A crate(used in [a rust config's crates value](RustConfigInner::crates)).
///
/// Can be any valid value that can be put in a Cargo.toml's dependency section.
///
/// Doesn't include the crate's name, which is assumed to be placed elsewhere.
pub enum Crate {
    /// A basic version. What is generally seen in most Cargo.toml's.
    Version(String),
    /// Detailed information about the dependency. Can include a version,
    /// features, a path, and/or a git repository.
    Detailed {
        /// The version. Generally a semver.
        version: Option<String>,
        /// The required features.
        features: Option<Vec<String>>,
        /// The path to the crate.
        path: Option<String>,
        /// The git repository of the crate.
        git: Option<String>,
    },
}

impl Crate {
    /// Serializes the [`Crate`] to the standard format used in a Cargo.toml.
    pub fn serialize(self) -> String {
        if let Crate::Version(str) = self {
            return format!("\"{}\"", str);
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

/// Many crates. The key for the outer HashMap is
/// a crate ID, and the key for the inner HashMap
/// is a crate.
pub type Crates = HashMap<String, HashMap<String, Crate>>;

#[derive(Deserialize, Debug, Clone)]
/// The actual Rust config. Referred to simply by "Rust config"
/// throughout this documentation. The entrypoint to deserializing
/// a `rust-config.toml` file's individual configs.
pub struct RustConfigInner {
    /// A list of toolchains.
    pub toolchains: Vec<Toolchain>,
    /// A list of crates.
    pub crates: Crates,
    /// A list of formats.
    pub formats: HashMap<String, Vec<Format>>,
}

/// A Rust config file. The entrypoint to deserializing a
/// `rust-config.toml` file.
pub type RustConfig = HashMap<String, RustConfigInner>;

/// Parse a `rust-config.toml` file. Simply reads a path and parses it as toml.
///
/// Currently panics upon an error; May change in the future.
pub fn parse_file(path: &Path) -> RustConfig {
    toml::from_str(&fs::read_to_string(path).unwrap()).unwrap()
}
