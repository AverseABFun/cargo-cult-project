//! Resources used in the produced output.

use rust_embed::Embed;

#[derive(Embed)]
#[folder = "src/template/"]
#[prefix = "template/"]
/// The template assets outputted to {temp_dir}/{target}/crates.
///
/// If viewing after running, it will be the output of [`cargo-local-registry`](https://crates.io/crates/cargo-local-registry/0.2.7)
/// (built alongside this crate in an incredibly janky way)
pub struct TemplateAssets;

#[derive(Embed)]
#[folder = "src/install/"]
#[prefix = "install/"]
/// The install assets outputted to {temp_dir}/{target}
/// Contains (as of writing this) install.sh, install.ps1, and install.fish
pub struct InstallAssets;

/// The README file automatically placed into {temp_dir}/{target}/crates
/// after [`cargo-local-registry`](https://crates.io/crates/cargo-local-registry/0.2.7) runs.
pub const CRATES_README: &str = include_str!("crates_README.md");
