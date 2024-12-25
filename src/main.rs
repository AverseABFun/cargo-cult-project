#![warn(missing_docs)]

//! The binary entrypoint to rust-pkg-gen. Currently contains most of the code,
//! but that will be changed eventually as a non-breaking change.

use clap::Parser;
use log::*;
use rand::{Rng, SeedableRng};
use rust_pkg_gen::resources::{InstallAssets, TemplateAssets};
use std::{
    fs::{self, write},
    path::{Path, PathBuf},
    process::{self, Stdio},
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
    #[arg(
        long = "no-download-toolchain",
        default_value_t = false,
        help = "Disable downloading the toolchain for major speed ups"
    )]
    no_download_toolchain: bool,
}

fn move_files_in_directory(src_dir: &str, dest_dir: &str) -> std::io::Result<()> {
    if !Path::new(dest_dir).exists() {
        fs::create_dir_all(dest_dir)?;
    }

    let entries = fs::read_dir(src_dir)?;

    for entry in entries {
        let entry = entry?;
        let entry_path = entry.path();

        if entry_path.is_dir() {
            continue;
        }

        let dest_path = Path::new(dest_dir).join(entry_path.file_name().unwrap());

        fs::rename(entry_path, dest_path)?;
    }

    Ok(())
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

            if !args.no_download_toolchain {
                if let Some(err) = rust_pkg_gen::copied::download_all(
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
                ) {
                    error!("{}", err);
                    process::exit(1);
                };
            }

            if !args.save_temp && !args.no_download_toolchain {
                fs::remove_dir_all(dir.join("tmp").to_str().unwrap()).unwrap();
            }

            if !args.no_download_toolchain {
                let dist_data_path = dir
                    .join("toolchain")
                    .join(format!("dist/channel-rust-{}.toml", toolchain.channel));
                let data = fs::read_to_string(dist_data_path.to_str().unwrap()).unwrap();

                let dist_data = data.parse::<toml::Value>().unwrap();
                let dist_dir_name = dist_data["date"].as_str().unwrap();

                move_files_in_directory(
                    dir.join("toolchain")
                        .join("dist")
                        .join(dist_dir_name)
                        .to_str()
                        .unwrap(),
                    dir.join("toolchain").to_str().unwrap(),
                )
                .unwrap();

                if !args.save_temp {
                    fs::remove_dir_all(dir.join("toolchain").join("dist").to_str().unwrap())
                        .unwrap();
                    fs::remove_file(
                        dir.join("toolchain")
                            .join(format!("channel-rust-{}.toml", toolchain.channel)),
                    )
                    .unwrap();
                    fs::remove_file(
                        dir.join("toolchain")
                            .join(format!("channel-rust-{}.toml.sha256", toolchain.channel)),
                    )
                    .unwrap();
                }
            }

            for ele in InstallAssets::iter() {
                let file = InstallAssets::get(&ele).unwrap();

                let ele = std::borrow::Cow::Borrowed(ele.split_once("/").unwrap().1);

                let path = dir.join(PathBuf::from(ele.as_ref()));
                let prefix = path.parent().unwrap();
                std::fs::create_dir_all(prefix).unwrap();

                let str_data = std::str::from_utf8(file.data.as_ref());

                if str_data.is_ok() {
                    std::fs::write(
                        path,
                        str_data
                            .unwrap()
                            .replace("&?TOOLCHAIN.CHANNEL", &toolchain.channel)
                            .replace("&?TOOLCHAIN.COMPONENTS", &toolchain.components.join(" "))
                            .replace(
                                "&?TOOLCHAIN.PKG",
                                if toolchain
                                    .format_map
                                    .iter()
                                    .map(|(_, v)| {
                                        cfg.formats[v]
                                            .iter()
                                            .map(|v| v.format.clone())
                                            .collect::<Vec<String>>()
                                            .contains(&"pkg".to_string())
                                    })
                                    .collect::<Vec<bool>>()
                                    .iter()
                                    .any(|v| *v)
                                {
                                    "$true"
                                } else {
                                    "$false"
                                },
                            )
                            .replace(
                                "&?TOOLCHAIN.MSI",
                                if toolchain
                                    .format_map
                                    .iter()
                                    .map(|(_, v)| {
                                        cfg.formats[v]
                                            .iter()
                                            .map(|v| v.format.clone())
                                            .collect::<Vec<String>>()
                                            .contains(&"msi".to_string())
                                    })
                                    .collect::<Vec<bool>>()
                                    .iter()
                                    .any(|v| *v)
                                {
                                    "$true"
                                } else {
                                    "$false"
                                },
                            ),
                    )
                    .unwrap();
                } else {
                    std::fs::write(path, file.data).unwrap();
                }
            }
        }
    }
}
