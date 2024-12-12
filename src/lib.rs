use serde::Deserialize;
use std::{collections::HashMap, fs, path::Path};

pub mod copied;
pub mod resources;
pub mod targets;
mod tests;

#[derive(Deserialize, Debug, Clone)]
pub struct Toolchain {
    pub edition: String,
    pub channel: String,
    pub profile: String,
    pub components: Vec<String>,
    pub crate_id: String,
    pub platforms: Vec<String>,
    pub targets: Vec<String>,
}

#[derive(Deserialize, Debug, Clone)]
pub struct Meta {
    pub offline: bool,
}

pub type Crate = String;

pub type Crates = HashMap<String, HashMap<String, String>>;

#[derive(Deserialize, Debug, Clone)]
pub struct RustConfigInner {
    pub toolchains: Vec<Toolchain>,
    pub meta: Meta,
    pub crates: Crates,
}

pub type RustConfig = HashMap<String, RustConfigInner>;

pub fn parse_file(path: &Path) -> RustConfig {
    toml::from_str(&fs::read_to_string(path).unwrap()).unwrap()
}
