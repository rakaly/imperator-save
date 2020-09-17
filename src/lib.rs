/*!
# Imperator Save

Imperator Save is a library to ergonomically work with Imperator Rome saves (debug + standard).

```rust
use imperator_save::{ImperatorExtractor, Encoding};
use std::io::Cursor;

let data = std::fs::read("assets/saves/observer1.5.rome")?;
let reader = Cursor::new(&data[..]);
let (save, encoding) = ImperatorExtractor::extract_save(reader)?;
assert_eq!(encoding, Encoding::Standard);
assert_eq!(save.header.version, String::from("1.5.3"));
# Ok::<(), Box<dyn std::error::Error>>(())
```

`ImperatorExtractor` will deserialize standard Imperator saves as well as those saved saved with `-debug_mode` (plaintext).

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
mod errors;
mod extraction;
mod flavor;
mod melt;
pub mod models;
mod tokens;

pub use date::*;
pub use errors::*;
pub use extraction::*;
pub use jomini::FailedResolveStrategy;
pub use melt::*;
