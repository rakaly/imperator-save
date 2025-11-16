/*!
# Imperator Save

Imperator Save is a library to ergonomically work with Imperator Rome saves (debug + standard).

```rust,ignore
use std::collections::HashMap;
use imperator_save::{ImperatorFile, Encoding, models::Save};

let data = std::fs::read("assets/saves/observer1.5.rome")?;
let file = ImperatorFile::from_slice(&data[..])?;
assert_eq!(file.encoding(), Encoding::BinaryZip);

let resolver = HashMap::<u16, &str>::new();
let mut zip_sink = Vec::new();
let parsed_file = file.parse(&mut zip_sink)?;
let save = Save::from_deserializer(&parsed_file.deserializer(), &resolver)?;
assert_eq!(save.meta.version, String::from("1.5.3"));
# Ok::<(), Box<dyn std::error::Error>>(())
```

## Ironman

Ironman saves are supported through a provided `TokenResolver`. Per PDS counsel, the data to construct such a `TokenResolver` is not distributed here.

*/

mod date;
mod errors;
mod file;
mod flavor;
mod melt;
pub mod models;

pub use date::*;
pub use errors::*;
pub use file::*;
pub use jomini::binary::{BasicTokenResolver, FailedResolveStrategy};
pub use melt::*;
