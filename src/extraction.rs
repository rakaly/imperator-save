/// Describes the format of the save before decoding
#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum Encoding {
    /// Save is encoded with the debug plaintext format:
    ///
    ///  - a save id line
    ///  - uncompressed text gamestate
    Text,

    /// Non-native plaintext imperator format
    ///
    ///  - a save id line
    ///  - zip with compressed plaintext gamestate
    TextZip,

    /// A standard ironman or normal save
    ///
    ///  - a save id line
    ///  - zip with compressed binary gamestate
    BinaryZip,

    /// Non-native binary imperator format
    ///
    ///  - a save id line
    ///  - uncompressed binary gamestate
    Binary,
}
