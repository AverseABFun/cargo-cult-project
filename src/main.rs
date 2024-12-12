use clap::Parser;
use rand::{Rng, SeedableRng};
use rust_pkg_gen::resources::TemplateAssets;
use std::{fs, path::PathBuf};

fn gen_char() -> u8 {
    rand::rngs::StdRng::from_entropy().gen_range(65..90)
}

#[derive(Parser, Debug)]
#[command(version)]
struct Cli {
    #[cfg(not(debug_assertions))]
    #[arg(long = "temp-dir")]
    temp_dir: Option<PathBuf>,

    #[cfg(debug_assertions)]
    #[arg(long = "temp-dir", default_value = "test")]
    temp_dir: Option<PathBuf>,

    #[arg(
        short = 'y',
        long = "yes",
        default_value_t = false,
        help = "Assume yes for all questions(excluding overwriting files)"
    )]
    yes: bool,
    #[arg(
        long = "overwrite",
        default_value_t = false,
        help = "The equivalent to --yes for overwriting files"
    )]
    overwrite: bool,
    #[arg(default_value = "rust-config.toml", help = "")]
    path: PathBuf,
}

fn generate_crates(
    cfg: &rust_pkg_gen::RustConfigInner,
    toolchain: &rust_pkg_gen::Toolchain,
) -> String {
    let mut out: String = String::new();
    for (crte, version) in cfg.crates.get(&toolchain.crate_id).unwrap() {
        out = format!("{}{} = \"{}\"\n", out, crte, version).to_string();
    }
    out
}

fn main() {
    let args = Cli::parse();

    let path = &PathBuf::from(args.path);

    let chars: &[u8; 6] = &[
        gen_char(),
        gen_char(),
        gen_char(),
        gen_char(),
        gen_char(),
        gen_char(),
    ];
    let dir = if args.temp_dir.is_some() {
        if args.temp_dir.clone().unwrap().exists() && !args.overwrite {
            let confirmation = dialoguer::Confirm::new()
                .with_prompt("Provided temporary directory already exists, overwrite?")
                .default(false)
                .interact()
                .unwrap();
            if !confirmation {
                println!("Aborting.");
                std::process::exit(1)
            } else {
                println!("Overwriting.");
                std::fs::remove_dir_all(args.temp_dir.clone().unwrap()).unwrap();
            }
        }
        args.temp_dir.unwrap()
    } else {
        std::env::temp_dir().join(PathBuf::from(String::from_utf8_lossy(chars).as_ref()))
    };

    let data = rust_pkg_gen::parse_file(path);

    for (item, cfg) in data {
        let dir = dir.join(item);
        for toolchain in &cfg.toolchains {
            for ele in TemplateAssets::iter() {
                let file = TemplateAssets::get(&ele).unwrap();

                let ele = std::borrow::Cow::Borrowed(ele.split_once("/").unwrap().1);

                let path = dir
                    .join(PathBuf::from("crates"))
                    .join(PathBuf::from(ele.as_ref()));
                let prefix = path.parent().unwrap();
                std::fs::create_dir_all(prefix).unwrap();

                std::fs::write(
                    path,
                    std::str::from_utf8(file.data.as_ref())
                        .unwrap()
                        .replace("{?TOOLCHAIN.EDITION}", &toolchain.edition)
                        .replace("{?TOOLCHAIN.CHANNEL}", &toolchain.channel)
                        .replace("{?TOOLCHAIN.PROFILE}", &toolchain.profile)
                        .replace(
                            "{?TOOLCHAIN.TARGETS}",
                            &("\"".to_owned() + &toolchain.targets.join("\",\"") + "\""),
                        )
                        .replace(
                            "{?TOOLCHAIN.COMPONENTS}",
                            &("\"".to_owned() + &toolchain.components.join("\",\"") + "\""),
                        )
                        .replace("{?CRATES}", &generate_crates(&(cfg.clone()), &toolchain)),
                )
                .unwrap();
            }

            std::process::Command::new("bash")
                .arg(
                    dir.join(PathBuf::from("crates"))
                        .join(PathBuf::from("build.sh")),
                )
                .spawn()
                .unwrap()
                .wait()
                .unwrap();

            rust_pkg_gen::copied::download_all(
                vec![&toolchain.channel],
                rust_pkg_gen::copied::DEFAULT_UPSTREAM_URL,
                dir.join("tmp").to_str().unwrap(),
                toolchain.targets.iter().map(|s| &**s).collect(),
                dir.join("toolchain").to_str().unwrap(),
                toolchain.components.iter().map(|s| &**s).collect(),
                toolchain.platforms.iter().map(|s| &**s).collect()
            );

            fs::remove_dir_all(dir.join("tmp").to_str().unwrap()).unwrap();
        }
    }
}
