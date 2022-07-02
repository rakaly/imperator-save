![ci](https://github.com/rakaly/imperator-save/workflows/ci/badge.svg) [![](https://docs.rs/imperator-save/badge.svg)](https://docs.rs/imperator-save) [![Version](https://img.shields.io/crates/v/imperator-save.svg?style=flat-square)](https://crates.io/crates/imperator-save)

# Imperator Save

Imperator Save is a library to ergonomically work with Imperator Rome saves (debug + standard).

```rust
use imperator_save::{ImperatorFile, Encoding, EnvTokens, models::Save};

let data = std::fs::read("assets/saves/observer1.5.rome")?;
let file = ImperatorFile::from_slice(&data[..])?;
assert_eq!(file.encoding(), Encoding::BinaryZip);

let mut zip_sink = Vec::new();
let parsed_file = file.parse(&mut zip_sink)?;
let save = Save::from_deserializer(&parsed_file.deserializer(), &EnvTokens)?;
assert_eq!(save.meta.version, String::from("1.5.3"));
assert_eq!(save.meta.version, String::from("1.5.3"));
```

## Ironman

By default, standard saves will not be decoded properly.

To enable support, one must supply an environment variable
(`IMPERATOR_TOKENS`) that points to a newline delimited
text file of token descriptions. For instance:

```ignore
0xffff my_test_token
0xeeee my_test_token2
```

In order to comply with legal restrictions, I cannot share the list of
tokens. I am also restricted from divulging how the list of tokens can be derived.
