use crate::models::HeaderOwned;
use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct Save {
    #[serde(flatten)]
    pub header: HeaderOwned,

    #[serde(flatten)]
    pub gamestate: GameState,
}

#[derive(Debug, Deserialize)]
pub struct GameState {
    pub speed: i32,
}
