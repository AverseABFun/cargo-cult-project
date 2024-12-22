#![warn(missing_docs)]

//! The binary entrypoint to rust-pkg-gen. Currently contains most of the code,
//! but that will be changed eventually as a non-breaking change.

use clap::Parser;
use log::*;
use rand::{Rng, SeedableRng};
use rust_pkg_gen::resources::TemplateAssets;
use std::{
    fs::{self, write},
    path::PathBuf,
    process::Stdio,
};

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
    #[arg(
        default_value = "rust-config.toml",
        help = "The path to the configuration file"
    )]
    path: PathBuf,
    #[arg(
        short = 'q',
        long = "quiet",
        default_value_t = false,
        help = "Doesn't display any unnecessary text(still shows confirmation prompts; to remove, use -y --overwrite as well or --silent."
    )]
    quiet: bool,
    #[arg(
        long = "silent",
        default_value_t = false,
        help = "Equivalent to -y -q --overwrite"
    )]
    silent: bool,
    #[arg(
        long = "save-temp",
        default_value_t = false,
        help = "Saves all temporary files"
    )]
    save_temp: bool,
}

fn generate_crates(
    cfg: &rust_pkg_gen::RustConfigInner,
    toolchain: &rust_pkg_gen::Toolchain,
) -> String {
    let mut out: String = String::new();
    for (crte, version) in cfg.crates.get(&toolchain.crate_id).unwrap() {
        out = format!("{}{} = {}\n", out, crte, version.clone().serialize()).to_string();
    }
    out
}

fn main() {
    let mut args = Cli::parse();

    if args.silent {
        args.quiet = true;
        args.yes = true;
        args.overwrite = true;
    } else {
        env_logger::init();
    }

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
                .with_prompt("Temporary directory already exists, overwrite?")
                .default(false)
                .interact()
                .unwrap();
            if !confirmation {
                if !args.quiet {
                    warn!("Aborting.");
                }
                return;
            } else {
                if !args.quiet {
                    info!("Overwriting.");
                }
                std::fs::remove_dir_all(args.temp_dir.clone().unwrap()).unwrap();
            }
        }
        if args.overwrite {
            std::fs::remove_dir_all(args.temp_dir.clone().unwrap()).unwrap();
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

            let mut build_c = std::process::Command::new("bash");
            let mut build = build_c.arg(
                dir.join(PathBuf::from("crates"))
                    .join(PathBuf::from("build.sh")),
            );
            build = build.env(
                "CARGO_BIN_FILE_CARGO_LOCAL_REGISTRY",
                env!("CARGO_BIN_FILE_CARGO_LOCAL_REGISTRY"),
            );
            if args.quiet {
                build = build
                    .stdout(Stdio::null())
                    .stderr(Stdio::null())
                    .stdin(Stdio::null());
            }
            if args.save_temp {
                build = build.arg("save")
            }
            build.spawn().unwrap().wait().unwrap();
            write(
                dir.join(PathBuf::from("crates"))
                    .join(PathBuf::from("README.md")),
                rust_pkg_gen::resources::CRATES_README.replace(
                    "{?TOOLCHAIN.CRATES_DIR}",
                    fs::canonicalize(dir.join(PathBuf::from("crates")))
                        .unwrap()
                        .to_str()
                        .unwrap(),
                ),
            )
            .unwrap();

            rust_pkg_gen::copied::download_all(
                vec![&toolchain.channel],
                rust_pkg_gen::copied::DEFAULT_UPSTREAM_URL,
                dir.join("tmp").to_str().unwrap(),
                toolchain.targets.iter().map(|s| &**s).collect(),
                dir.join("toolchain").to_str().unwrap(),
                toolchain.components.iter().map(|s| &**s).collect(),
                toolchain.platforms.iter().map(|s| &**s).collect(),
                args.quiet,
                toolchain
                    .format_map
                    .iter()
                    .map(|(k, v)| (k.as_str(), cfg.formats[v].clone()))
                    .collect(),
            );

            if !args.save_temp {
                fs::remove_dir_all(dir.join("tmp").to_str().unwrap()).unwrap();
            }
        }
    }
}
