To properly use this source, add this to your `.cargo/config[.toml]` somewhere:

```toml
[source.crates-io]
registry = 'sparse+<https://index.crates.io/>'
replace-with = 'local-registry'

[source.local-registry]
local-registry = '{?TOOLCHAIN.CRATES_DIR}' # double-check this path is correct, especially if you are creating the package on a different machine than the machine you are creating the package for
```
