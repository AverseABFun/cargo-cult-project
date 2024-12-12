use std::{collections::HashMap, fs, path::Path};
use serde::Deserialize;

pub mod resources;
mod tests;

#[derive(Deserialize, Debug)]
pub struct Toolchain {
    pub edition: String,
    pub channel: String,
    pub profile: String,
    pub components: Vec<String>
}

#[derive(Deserialize, Debug)]
pub struct Meta {
    pub offline: bool,
    pub platforms: Vec<String>,
    pub targets: Vec<String>
}

pub type Crate = String;

pub type Crates = HashMap<String, HashMap<String, String>>;

#[derive(Deserialize, Debug)]
pub struct RustConfigInner {
    pub toolchains: Vec<Toolchain>,
    pub meta: Meta,
    pub crates: Crates
}

pub type RustConfig = HashMap<String, RustConfigInner>;

pub fn parse_file(path: &Path) -> RustConfig {
    toml::from_str(&fs::read_to_string(path).unwrap()).unwrap()
}