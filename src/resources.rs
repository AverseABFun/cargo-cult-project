use rust_embed::Embed;

#[derive(Embed)]
#[folder = "src/template/"]
#[prefix = "template/"]
pub struct TemplateAssets;

const RUSTUP_INIT_ASSET: &str = include_str!("rustup-init.sh");