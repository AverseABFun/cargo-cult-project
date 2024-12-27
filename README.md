# rust-pkg-gen

`rust-pkg-gen` creates so-called rust packages with arbitrary toolchains/components and arbitrary crates. These packages then can be used on separate machines, and installed with provided install scripts.

Currently, there is a small public API, but this API doesn't contain much. One of the things it does include, however, is code for parsing `rust-config.toml` files(contains configuration for `rust-pkg-gen`).

An example `rust-config.toml` file is in this repo.

In debug builds, the default temporary directory is `./test`(relative to where `rust-pkg-gen` was called). In release builds, it creates a new folder in `std::env::temp_dir()`. This behavior can be changed by providing a path to `--temp-dir`. See `--help` for a list of flags that can be used.
