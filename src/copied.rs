// Copied from crate rustup-mirror(https://github.com/jiegec/rustup-mirror)
// Some modifications made due to deprecated/changed APIs
// (and differing purposes/necessities)

use anyhow::{anyhow, Error};
use filebuffer::FileBuffer;
use sha2::{Digest, Sha256};
use std::collections::{HashMap, HashSet};
use std::fs::{copy, create_dir_all, File};
use std::io::{Read, Write};
use std::path::{Component, Path, PathBuf};
use toml::Value;
use url::Url;

pub const DEFAULT_UPSTREAM_URL: &str = "https://static.rust-lang.org/";

fn file_sha256(file_path: &Path) -> Option<String> {
    let file = Path::new(file_path);
    if file.exists() {
        let buffer = FileBuffer::open(&file).unwrap();
        Some(hex::encode(Sha256::new().chain_update(buffer).finalize()))
    } else {
        None
    }
}

fn download(upstream_url: &str, dir: &str, path: &str) -> Result<PathBuf, Error> {
    println!("Downloading file {}...", path);
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

pub fn download_all(
    channels: Vec<&str>,
    upstream_url: &str,
    orig_path: &str,
    targets: Vec<&str>,
    to_path: &str,
    components: Vec<&str>,
    for_targets: Vec<&str>,
    quiet: bool,
    format_map: HashMap<&str, Vec<crate::Format>>,
) {
    for channel in channels.clone() {
        if !crate::targets::RELEASE_CHANNELS.contains(&channel) {
            return;
        }
    }
    for target in targets.clone() {
        if !crate::targets::TARGETS.contains(&target) {
            return;
        }
    }
    for target in for_targets.clone() {
        if !crate::targets::TARGETS.contains(&target) {
            return;
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
        assert_eq!(
            file_sha256(file_path.as_path()).unwrap(),
            &sha256_data[..64]
        );

        let mut value = data.parse::<Value>().unwrap();
        assert_eq!(value["manifest-version"].as_str(), Some("2"));

        for ele in for_targets.clone() {
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
                    println!("File {} already downloaded, skipping", file_name);
                }

                if need_download || hash_file_missing {
                    File::create(hash_file)
                        .unwrap()
                        .write_all(hash_file_cont.unwrap().as_bytes())
                        .unwrap();
                    if !quiet {
                        println!("Writing checksum for file {}", file_name);
                    }
                }
            } else if ele.contains("darwin") {
                let artifacts = value["artifacts"]["installer-pkg"]["target"][ele]
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
                    println!("File {} already downloaded, skipping", file_name);
                }

                if need_download || hash_file_missing {
                    File::create(hash_file)
                        .unwrap()
                        .write_all(hash_file_cont.unwrap().as_bytes())
                        .unwrap();
                    if !quiet {
                        println!("Writing checksum for file {}", file_name);
                    }
                }
            }
        }

        let pkgs = value["pkg"].as_table_mut().unwrap();
        let keys: Vec<String> = pkgs.keys().cloned().collect();
        for pkg_name in keys {
            if !components.contains(&pkg_name.as_str()) {
                continue;
            }
            let pkg = pkgs.get_mut(&pkg_name).unwrap().as_table_mut().unwrap();
            let pkg_targets = pkg.get_mut("target").unwrap().as_table_mut().unwrap();
            for (target, pkg_target) in pkg_targets {
                let pkg_target = pkg_target.as_table_mut().unwrap();

                // if we don't want to download this target
                // set available to false and do not download
                // but we will keep this table in the toml, which is required for newer version of
                // rustup
                if !(targets.contains(&target.as_str()) || *target == "*") {
                    *pkg_target.get_mut("available").unwrap() = toml::Value::Boolean(false);
                    continue;
                }

                if pkg_target["available"].as_bool().unwrap() {
                    all_targets.insert(target.clone());

                    let prefixes = ["", "xz_"];
                    for prefix in prefixes.iter() {
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
                            println!("File {} already downloaded, skipping", file_name);
                        }

                        if need_download || hash_file_missing {
                            File::create(hash_file)
                                .unwrap()
                                .write_all(hash_file_cont.unwrap().as_bytes())
                                .unwrap();
                            if !quiet {
                                println!("Writing checksum for file {}", file_name);
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
            println!("Producing /{}", name);
        }
        file.write_all(output.as_bytes()).unwrap();

        let sha256_new_file = file_sha256(&path).unwrap();
        let sha256_new_file_path = Path::new(to_path).join(&sha256_name);
        let mut file = File::create(sha256_new_file_path.clone()).unwrap();
        if !quiet {
            println!("Producing /{}", sha256_name);
        }
        file.write_all(format!("{}  channel-rust-{}.toml", sha256_new_file, channel).as_bytes())
            .unwrap();

        let date = value["date"].as_str().unwrap();

        let alt_name = format!("dist/{}/channel-rust-{}.toml", date, channel);
        let alt_path = Path::new(to_path).join(&alt_name);
        create_dir_all(alt_path.parent().unwrap()).unwrap();
        copy(path, alt_path).unwrap();
        if !quiet {
            println!("Producing /{}", alt_name);
        }

        let alt_sha256_new_file_name =
            format!("dist/{}/channel-rust-{}.toml.sha256", date, channel);
        let alt_sha256_new_file_path = Path::new(to_path).join(&alt_sha256_new_file_name);
        copy(sha256_new_file_path, alt_sha256_new_file_path).unwrap();
        if !quiet {
            println!("Producing /{}", alt_sha256_new_file_name);
        }
    }
}
