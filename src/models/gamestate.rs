use crate::{file::ImperatorDeserializer, models::MetadataOwned, ImperatorError};
use jomini::binary::TokenResolver;
use serde::Deserialize;

#[derive(Debug)]
pub struct Save {
    pub meta: MetadataOwned,
    pub gamestate: GameState,
}

impl Save {
    pub fn from_deserializer<R>(
        deser: &ImperatorDeserializer,
        resolver: &R,
    ) -> Result<Self, ImperatorError>
    where
        R: TokenResolver,
    {
        let meta = deser.build(resolver)?;
        let gamestate = deser.build(resolver)?;
        Ok(Save { meta, gamestate })
    }
}

#[derive(Debug, Deserialize)]
pub struct GameState {
    pub speed: i32,
}
