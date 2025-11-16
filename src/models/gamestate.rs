use crate::{
    ImperatorBinaryDeserialization, ImperatorDate, ImperatorError, ImperatorErrorKind,
    ImperatorFile, JominiFileKind, ReaderAt, SaveContentKind, SaveDataKind, SaveMetadataKind,
};
use jomini::binary::TokenResolver;
use serde::Deserialize;

#[derive(Debug)]
pub struct Save {
    pub meta: Metadata,
    pub gamestate: GameState,
}

impl Save {
    pub fn from_file(
        file: &mut ImperatorFile<impl ReaderAt>,
        resolver: &impl TokenResolver,
    ) -> Result<Self, ImperatorError> {
        match file.kind_mut() {
            JominiFileKind::Uncompressed(SaveDataKind::Text(x)) => Ok(x
                .deserializer()
                .deserialize()
                .map_err(ImperatorErrorKind::Deserialize)?),
            JominiFileKind::Uncompressed(SaveDataKind::Binary(x)) => Ok(x
                .deserializer(resolver)
                .deserialize()
                .map_err(ImperatorErrorKind::Deserialize)?),
            JominiFileKind::Zip(x) => {
                let gamestate: GameState =
                    match x.gamestate().map_err(ImperatorErrorKind::Envelope)? {
                        SaveContentKind::Text(mut x) => x
                            .deserializer()
                            .deserialize()
                            .map_err(ImperatorErrorKind::Deserialize)?,
                        SaveContentKind::Binary(mut x) => x
                            .deserializer(resolver)
                            .deserialize()
                            .map_err(ImperatorErrorKind::Deserialize)?,
                    };

                let meta = match x.meta().map_err(ImperatorErrorKind::Envelope)? {
                    SaveMetadataKind::Text(mut x) => x
                        .deserializer()
                        .deserialize()
                        .map_err(ImperatorErrorKind::Deserialize)?,
                    SaveMetadataKind::Binary(mut x) => x
                        .deserializer(resolver)
                        .deserialize()
                        .map_err(ImperatorErrorKind::Deserialize)?,
                };

                Ok(Self { meta, gamestate })
            }
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct Metadata {
    pub version: String,
    pub date: ImperatorDate,
    #[serde(default)]
    pub ironman: bool,
    pub meta_player_name: Option<String>,
    pub enabled_dlcs: Vec<String>,
    pub play_time: i32,
    #[serde(default)]
    pub iron: bool,
}

#[derive(Debug, Deserialize)]
pub struct GameState {
    pub speed: i32,
}

impl<'de> Deserialize<'de> for Save {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        #[derive(Debug, Deserialize)]
        struct ImperatorFlatten {
            pub version: String,
            pub date: ImperatorDate,
            #[serde(default)]
            pub ironman: bool,
            pub meta_player_name: Option<String>,
            pub enabled_dlcs: Vec<String>,
            pub play_time: i32,
            #[serde(default)]
            pub iron: bool,
            pub speed: i32,
        }

        let result = ImperatorFlatten::deserialize(deserializer)?;
        Ok(Save {
            meta: Metadata {
                version: result.version,
                date: result.date,
                ironman: result.ironman,
                meta_player_name: result.meta_player_name,
                enabled_dlcs: result.enabled_dlcs,
                play_time: result.play_time,
                iron: result.iron,
            },
            gamestate: GameState {
                speed: result.speed,
            },
        })
    }
}
