//! Copied from crate [rustup-mirror](https://crates.io/crates/rustup-mirror/0.8.1)
//! Lots of modifications made due to deprecated/changed APIs
//! (and differing purposes/necessities).
//!
//! Note that I(the maintainer of rust-pkg-gen) wrote the doc comments, NOT
//! the maintainer of [rustup-mirror](https://crates.io/crates/rustup-mirror/0.8.1).

use anyhow::{anyhow, Error};
use filebuffer::FileBuffer;
use log::*;
use sha2::{Digest, Sha256};
use std::collections::{HashMap, HashSet};
use std::fs::{copy, create_dir_all, File};
use std::io::{Read, Write};
use std::path::{Component, Path, PathBuf};
use toml::Value;
use url::Url;

use crate::Suffix;

/// The default upstream URL. Usually passed to [`download`] or [`download_all`]
/// when you don't have a custom upstream url to use.
pub const DEFAULT_UPSTREAM_URL: &str = "https://static.rust-lang.org/";

/// Produces the SHA256 hash of the provided file
fn file_sha256(file_path: &Path) -> Option<String> {
    let file = Path::new(file_path);
    if file.exists() {
        let buffer = FileBuffer::open(&file).unwrap();
        Some(hex::encode(Sha256::new().chain_update(buffer).finalize()))
    } else {
        None
    }
}

/// Download a path from the provided upstream URL.
fn download(upstream_url: &str, dir: &str, path: &str) -> Result<PathBuf, Error> {
    info!("Downloading file {}...", path);
    let manifest = format!("{}{}", upstream_url, path);
    let mut response = reqwest::blocking::get(&manifest)?;
    let mirror = Path::new(dir);
    let file_path = mirror.join(&path);
    create_dir_all(file_path.parent().unwrap())?;
    let mut dest = File::create(file_path)?;

    let length = match response.content_length() {
        None => return Err(anyhow!("Not found")),
        Some(l) => l,
    };

    let mut buffer = [0u8; 4096];
    let mut read = 0;

    while read < length {
        let len = response.read(&mut buffer)?;
        dest.write_all(&buffer[..len])?;
        read += len as u64;
    }

    Ok(mirror.join(path))
}

/// I'm honestly unsure what this one does. If you know, please submit an issue or PR!
pub fn normalize_path(path: &Path) -> PathBuf {
    let mut components = path.components().peekable();
    let mut ret = if let Some(c @ Component::Prefix(..)) = components.peek().cloned() {
        components.next();
        PathBuf::from(c.as_os_str())
    } else {
        PathBuf::new()
    };

    for component in components {
        match component {
            Component::Prefix(..) => unreachable!(),
            Component::RootDir => {
                ret.push(component.as_os_str());
            }
            Component::CurDir => {}
            Component::ParentDir => {
                ret.pop();
            }
            Component::Normal(c) => {
                ret.push(c);
            }
        }
    }
    ret
}

/// This is a beefy function. It takes an absurd number of arguments
/// and based on them downloads a certain subset of the rust components
/// that are relevant.
///
/// I changed this one from the original crate a *lot*. This is based
/// on part of the main function in the original crate with many more
/// validations and miscellaneous changes.
pub fn download_all(
    channels: Vec<&str>,
    upstream_url: &str,
    orig_path: &str,
    mut targets: Vec<&str>,
    to_path: &str,
    mut components: Vec<&str>,
    platforms: Vec<&str>,
    quiet: bool,
    format_map: HashMap<&str, Vec<crate::Format>>,
) -> Option<Error> {
    for channel in channels.clone() {
        if !crate::targets::RELEASE_CHANNELS.contains(&channel) {
            return Some(anyhow!("invalid channel"));
        }
    }
    for target in targets.clone() {
        if !crate::targets::TARGETS.contains(&target) {
            return Some(anyhow!("invalid rust target"));
        }
    }
    for target in platforms.clone() {
        if !crate::targets::TARGETS.contains(&target) {
            return Some(anyhow!("invalid compilation target"));
        }
        let idx = targets.binary_search(&target);
        if idx.is_ok() {
            targets.swap_remove(idx.unwrap());
        }
    }
    for (target, formats) in format_map.clone() {
        if !platforms.contains(&target) {
            return Some(anyhow!(
                "target {target} that is not being built for in target map"
            ));
        }
        if formats.len() < 1 {
            return Some(anyhow!("format list is empty"));
        }
        if formats[0].format == "msi" && !target.contains("windows") {
            if formats[0].suffix == Suffix::Only {
                return Some(anyhow!(
                    "target {target} is not windows but formats require msi"
                ));
            }
            if !quiet {
                warn!("target {target} is not windows but formats want msi; continuing");
            }
        }
        if formats[0].format == "pkg" && !target.contains("apple") {
            if formats[0].suffix == Suffix::Only {
                return Some(anyhow!(
                    "target {target} is not apple but formats require pkg"
                ));
            }
            if !quiet {
                warn!("target {target} is not apple but formats want pkg; continuing");
            }
        }
        for format in formats {
            if !vec![
                String::from("msi"),
                String::from("pkg"),
                String::from("gz"),
                String::from("xz"),
            ]
            .contains(&format.format)
            {
                return Some(anyhow!("invalid format {}", format.format));
            }
        }
    }

    let mut all_targets = HashSet::new();

    // All referenced files
    let mut referenced = HashSet::new();

    // Fetch rust components
    for channel in channels.iter() {
        let name = format!("dist/channel-rust-{}.toml", channel);
        let file_path = download(upstream_url, orig_path, &name).unwrap();
        let sha256_name = format!("dist/channel-rust-{}.toml.sha256", channel);
        let sha256_file_path = download(upstream_url, orig_path, &sha256_name).unwrap();

        let mut file = File::open(file_path.clone()).unwrap();
        let mut data = String::new();
        file.read_to_string(&mut data).unwrap();

        let mut sha256_file = File::open(sha256_file_path.clone()).unwrap();
        let mut sha256_data = String::new();
        sha256_file.read_to_string(&mut sha256_data).unwrap();
        let sha256 = file_sha256(file_path.as_path()).unwrap();
        if sha256 != &sha256_data[..64] {
            return Some(anyhow!(
                "expected SHA256 of {name} to be {} but was {}",
                &sha256_data[..64],
                sha256
            ));
        }

        let mut value = data.parse::<Value>().unwrap();
        if value["manifest-version"].as_str() != Some("2") {
            return Some(anyhow!("manifest version of channel {channel} not 2"));
        }

        for ele in platforms.clone() {
            if ele.contains("windows") {
                let artifacts = value["artifacts"]["installer-msi"]["target"][ele][0]
                    .as_table_mut()
                    .unwrap();

                let url = Url::parse(artifacts["url"].as_str().unwrap()).unwrap();
                let mirror = Path::new(to_path);
                let file_name = url.path().replace("%20", " ");
                let file = mirror.join(&file_name[1..]);

                let hash_file = mirror.join(format!("{}.sha256", &file_name[1..]));
                let hash_file_cont = File::open(hash_file.clone()).ok().and_then(|mut f| {
                    let mut cont = String::new();
                    f.read_to_string(&mut cont).ok().map(|_| cont)
                });

                let hash_file_missing = hash_file_cont.is_none();
                let mut hash_file_cont = hash_file_cont.or_else(|| file_sha256(file.as_path()));

                let chksum_upstream = artifacts["hash-sha256"].as_str().unwrap();

                let need_download = match hash_file_cont {
                    Some(ref chksum) => chksum_upstream != chksum,
                    None => true,
                };

                if need_download {
                    download(upstream_url, to_path, &file_name[1..]).unwrap();
                    hash_file_cont = file_sha256(file.as_path());
                    assert_eq!(Some(chksum_upstream), hash_file_cont.as_deref());
                } else if !quiet {
                    info!("File {} already downloaded, skipping", file_name);
                }

                if need_download || hash_file_missing {
                    File::create(hash_file)
                        .unwrap()
                        .write_all(hash_file_cont.unwrap().as_bytes())
                        .unwrap();
                    if !quiet {
                        info!("Writing checksum for file {}", file_name);
                    }
                }
                let idx = components.binary_search(&"rustc");
                if idx.is_ok() {
                    components.swap_remove(idx.unwrap());
                }
                let idx = components.binary_search(&"cargo");
                if idx.is_ok() {
                    components.swap_remove(idx.unwrap());
                }
                let idx = components.binary_search(&"rustdoc");
                if idx.is_ok() {
                    components.swap_remove(idx.unwrap());
                }
                let idx = components.binary_search(&"rust-std");
                if idx.is_ok() {
                    components.swap_remove(idx.unwrap());
                }
            } else if ele.contains("apple") {
                let artifacts = value["artifacts"]["installer-pkg"]["target"][ele][0]
                    .as_table_mut()
                    .unwrap();

                let url = Url::parse(artifacts["url"].as_str().unwrap()).unwrap();
                let mirror = Path::new(to_path);
                let file_name = url.path().replace("%20", " ");
                let file = mirror.join(&file_name[1..]);

                let hash_file = mirror.join(format!("{}.sha256", &file_name[1..]));
                let hash_file_cont = File::open(hash_file.clone()).ok().and_then(|mut f| {
                    let mut cont = String::new();
                    f.read_to_string(&mut cont).ok().map(|_| cont)
                });

                let hash_file_missing = hash_file_cont.is_none();
                let mut hash_file_cont = hash_file_cont.or_else(|| file_sha256(file.as_path()));

                let chksum_upstream = artifacts["hash-sha256"].as_str().unwrap();

                let need_download = match hash_file_cont {
                    Some(ref chksum) => chksum_upstream != chksum,
                    None => true,
                };

                if need_download {
                    download(upstream_url, to_path, &file_name[1..]).unwrap();
                    hash_file_cont = file_sha256(file.as_path());
                    assert_eq!(Some(chksum_upstream), hash_file_cont.as_deref());
                } else if !quiet {
                    info!("File {} already downloaded, skipping", file_name);
                }

                if need_download || hash_file_missing {
                    File::create(hash_file)
                        .unwrap()
                        .write_all(hash_file_cont.unwrap().as_bytes())
                        .unwrap();
                    if !quiet {
                        info!("Writing checksum for file {}", file_name);
                    }
                }
                let idx = components.binary_search(&"rustc");
                if idx.is_ok() {
                    components.swap_remove(idx.unwrap());
                }
                let idx = components.binary_search(&"cargo");
                if idx.is_ok() {
                    components.swap_remove(idx.unwrap());
                }
                let idx = components.binary_search(&"rustdoc");
                if idx.is_ok() {
                    components.swap_remove(idx.unwrap());
                }
                let idx = components.binary_search(&"rust-std");
                if idx.is_ok() {
                    components.swap_remove(idx.unwrap());
                }
            }
        }
        if !components.contains(&"rust-std") {
            components.push("rust-std")
        }

        let pkgs = value["pkg"].as_table_mut().unwrap();
        let keys: Vec<String> = pkgs.keys().cloned().collect();
        for component in keys {
            if !components.contains(&component.as_str()) {
                continue;
            }
            let pkg = pkgs.get_mut(&component).unwrap().as_table_mut().unwrap();
            let pkg_targets = pkg.get_mut("target").unwrap().as_table_mut().unwrap();
            for (target, pkg_target) in pkg_targets {
                let pkg_target = pkg_target.as_table_mut().unwrap();

                // if we don't want to download this target
                // set available to false and do not download
                // but we will keep this table in the toml, which is required for newer version of
                // rustup
                if !(platforms.contains(&target.as_str()) || *target == "*")
                    && !targets.contains(&target.as_str())
                {
                    *pkg_target.get_mut("available").unwrap() = toml::Value::Boolean(false);
                    continue;
                }

                if pkg_target["available"].as_bool().unwrap() {
                    all_targets.insert(target.clone());

                    let prefixes = ["", "xz_"];
                    for prefix in prefixes.iter() {
                        if !targets.contains(&target.as_str()) {
                            if *prefix == "xz_"
                                && !format_map.clone()[target.as_str()]
                                    .clone()
                                    .into_iter()
                                    .any(|v| v.format == "xz")
                            {
                                continue;
                            }
                            if *prefix == ""
                                && !format_map.clone()[target.as_str()]
                                    .clone()
                                    .into_iter()
                                    .any(|v| v.format == "gz")
                            {
                                continue;
                            }
                        }
                        let url =
                            Url::parse(pkg_target[&format!("{}url", prefix)].as_str().unwrap())
                                .unwrap();
                        let mirror = Path::new(to_path);
                        let file_name = url.path().replace("%20", " ");
                        let file = mirror.join(&file_name[1..]);

                        referenced.insert(normalize_path(&file));

                        let hash_file = mirror.join(format!("{}.sha256", &file_name[1..]));
                        let hash_file_cont =
                            File::open(hash_file.clone()).ok().and_then(|mut f| {
                                let mut cont = String::new();
                                f.read_to_string(&mut cont).ok().map(|_| cont)
                            });

                        let hash_file_missing = hash_file_cont.is_none();
                        let mut hash_file_cont =
                            hash_file_cont.or_else(|| file_sha256(file.as_path()));

                        let chksum_upstream =
                            pkg_target[&format!("{}hash", prefix)].as_str().unwrap();

                        let need_download = match hash_file_cont {
                            Some(ref chksum) => chksum_upstream != chksum,
                            None => true,
                        };

                        if need_download {
                            download(upstream_url, to_path, &file_name[1..]).unwrap();
                            hash_file_cont = file_sha256(file.as_path());
                            assert_eq!(Some(chksum_upstream), hash_file_cont.as_deref());
                        } else if !quiet {
                            info!("File {} already downloaded, skipping", file_name);
                        }

                        if need_download || hash_file_missing {
                            File::create(hash_file)
                                .unwrap()
                                .write_all(hash_file_cont.unwrap().as_bytes())
                                .unwrap();
                            if !quiet {
                                info!("Writing checksum for file {}", file_name);
                            }
                        }

                        pkg_target.insert(
                            format!("{}url", prefix),
                            Value::String(format!("{}{}", upstream_url, file_name)),
                        );
                    }
                }
            }
        }

        let output = toml::to_string(&value).unwrap();
        let path = Path::new(to_path).join(&name);
        create_dir_all(path.parent().unwrap()).unwrap();
        let mut file = File::create(path.clone()).unwrap();
        if !quiet {
            info!("Producing /{}", name);
        }
        file.write_all(output.as_bytes()).unwrap();

        let sha256_new_file = file_sha256(&path).unwrap();
        let sha256_new_file_path = Path::new(to_path).join(&sha256_name);
        let mut file = File::create(sha256_new_file_path.clone()).unwrap();
        if !quiet {
            info!("Producing /{}", sha256_name);
        }
        file.write_all(format!("{}  channel-rust-{}.toml", sha256_new_file, channel).as_bytes())
            .unwrap();

        let date = value["date"].as_str().unwrap();

        let alt_name = format!("dist/{}/channel-rust-{}.toml", date, channel);
        let alt_path = Path::new(to_path).join(&alt_name);
        create_dir_all(alt_path.parent().unwrap()).unwrap();
        copy(path, alt_path).unwrap();
        if !quiet {
            info!("Producing /{}", alt_name);
        }

        let alt_sha256_new_file_name =
            format!("dist/{}/channel-rust-{}.toml.sha256", date, channel);
        let alt_sha256_new_file_path = Path::new(to_path).join(&alt_sha256_new_file_name);
        copy(sha256_new_file_path, alt_sha256_new_file_path).unwrap();
        if !quiet {
            info!("Producing /{}", alt_sha256_new_file_name);
        }
    }
    None
}
