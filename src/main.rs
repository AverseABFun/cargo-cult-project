use std::{env, path::PathBuf};
use rand::{Rng, SeedableRng};
use rust_pkg_gen::parse_file;
use rust_pkg_gen::resources::TemplateAssets;
use clap::Parser;

fn gen_char() -> u8 {
    rand::rngs::StdRng::from_entropy().gen_range(65..90)
}

#[derive(Parser, Debug)]
struct Cli {
    #[arg(long="temp-dir", default_value="InVaLiD!")]
    temp_dir: PathBuf,
    #[arg(default_value="rust-config.toml")]
    path: PathBuf,
}

fn main() {
    let args = Cli::parse();

    let path = &PathBuf::from(args.path);
    println!("{:#?}", parse_file(path));

    let chars: &[u8; 6] = &[gen_char(), gen_char(), gen_char(), gen_char(), gen_char(), gen_char()];
    let dir = if args.temp_dir != PathBuf::from("InVaLiD!") {
        args.temp_dir
    } else {
        std::env::temp_dir().join(PathBuf::from(String::from_utf8_lossy(chars).as_ref()))
    };
    let _ = std::fs::create_dir(dir.clone());
    for mut ele in TemplateAssets::iter() {
        let file = TemplateAssets::get(&ele).unwrap();
        ele = ele.split_once("/").unwrap().1.as_ref();
        println!("{}", ele.as_ref());
        for ele in PathBuf::from(ele.as_ref()).ancestors() {
            let _ = std::fs::create_dir(PathBuf::from().join(ele));
        }
        std::fs::write(dir.join(PathBuf::from(ele.as_ref())), file.data).unwrap();
    }
}
