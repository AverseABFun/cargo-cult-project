To properly use this source, add this to your `.cargo/config[.toml]` somewhere:

```toml
[source.crates-io]
registry = 'sparse+<https://index.crates.io/>'
replace-with = 'local-registry'

[source.local-registry]
local-registry = '' # add the path to the crates folder(the folder containing this file)
```

(psst- if you know how to install crate files to the registry manually(smth like `cargo registry add /path/to/crate/file`), make an issue or PR on [rust-pkg-gen](https://github.com/AverseABFun/rust-pkg-gen)!)
