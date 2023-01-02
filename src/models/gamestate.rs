use crate::{file::ImperatorDeserializer, models::MetadataOwned, ImperatorError};
use jomini::binary::TokenResolver;
use serde::Deserialize;

#[derive(Debug)]
pub struct Save {
    pub meta: MetadataOwned,
    pub gamestate: GameState,
}

impl Save {
    pub fn from_deserializer<RES>(
        deser: &ImperatorDeserializer<RES>,
    ) -> Result<Self, ImperatorError>
    where
        RES: TokenResolver,
    {
        let meta = deser.deserialize()?;
        let gamestate = deser.deserialize()?;
        Ok(Save { meta, gamestate })
    }
}

#[derive(Debug, Deserialize)]
pub struct GameState {
    pub speed: i32,
}
