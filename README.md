# rust-pkg-gen

`rust-pkg-gen` creates so-called rust packages with arbitrary toolchains/components and arbitrary crates. These packages then can be used on separate machines, and installed with provided install scripts.

To use the cli, you can generally simply run `rust-pkg-gen`/`cargo run`. There are a couple of command line options, mostly for debugging, that you can see in `src/main.rs` and below:

| Argument | Purpose |
| -------- | ------- |
| --temp-dir | Changes the temporary directory. Use in conjunction with --save-temp to save more of the temporary files. Some temporary files are always kept. Default value in debug builds is ./test, in release builds a automatically created directory in the system temporary directory. |
| -y or --yes | Currently unused, says yes to all non-overwriting prompts. |
| --overwrite | Doesn't ask the user if they want to overwrite the temporary directory(mainly used in testing). |
| --path | The path to a toml file of the format of rust-config.toml. |
| --quiet | Displays minimal text; confirmation prompts still appear. |
| --silent | Equivelent to --quiet --yes --overwrite. |
| --save-temp | Saves temporary files that by default aren't saved. |
| --no-download-toolchain | Doesn't download a toolchain. Mainly used in testing when working on crates. Massively improves speed. |
| --no-build-crates | Copies the template crates directory, however doesn't run build.sh. Mainly used in testing. |

Currently, there is a small public API, but this API doesn't contain much. One of the things it does include, however, is code for parsing `rust-config.toml` files(contains configuration for `rust-pkg-gen`).

An example `rust-config.toml` file is in this repo.

In debug builds, the default temporary directory is `./test`(relative to where `rust-pkg-gen` was called). In release builds, it creates a new folder in `std::env::temp_dir()`. This behavior can be changed by providing a path to `--temp-dir`. See `--help` for a list of flags that can be used.
