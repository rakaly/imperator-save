/*!
# Imperator Save

Imperator Save is a library to ergonomically work with Imperator Rome saves (debug + standard).

```rust,ignore
use imperator_save::{ImperatorFile, Encoding, EnvTokens, models::Save};

let data = std::fs::read("assets/saves/observer1.5.rome")?;
let file = ImperatorFile::from_slice(&data[..])?;
assert_eq!(file.encoding(), Encoding::BinaryZip);

let mut zip_sink = Vec::new();
let parsed_file = file.parse(&mut zip_sink)?;
let save = Save::from_deserializer(&parsed_file.deserializer(), &EnvTokens)?;
assert_eq!(save.meta.version, String::from("1.5.3"));
# Ok::<(), Box<dyn std::error::Error>>(())
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
*/

mod date;
mod deflate;
mod errors;
mod extraction;
pub mod file;
mod flavor;
mod header;
mod melt;
pub mod models;
mod tokens;

pub use date::*;
pub use errors::*;
pub use extraction::*;
#[doc(inline)]
pub use file::ImperatorFile;
pub use header::*;
pub use jomini::binary::FailedResolveStrategy;
pub use melt::*;
pub use tokens::EnvTokens;
