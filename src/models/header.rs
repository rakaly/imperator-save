use crate::ImperatorDate;
use serde::Deserialize;
use std::borrow::Cow;

#[derive(Debug, Deserialize)]
pub struct HeaderOwned {
    pub save_game_version: i32,
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
pub struct HeaderBorrowed<'a> {
    pub save_game_version: i32,
    #[serde(borrow)]
    pub version: Cow<'a, str>,
    pub date: ImperatorDate,
    #[serde(default)]
    pub ironman: bool,
    pub meta_player_name: Option<Cow<'a, str>>,
    pub enabled_dlcs: Vec<Cow<'a, str>>,
    pub play_time: i32,
    #[serde(default)]
    pub iron: bool,
}
