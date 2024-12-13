use rust_embed::Embed;

#[derive(Embed)]
#[folder = "src/template/"]
#[prefix = "template/"]
pub struct TemplateAssets;
