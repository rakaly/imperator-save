use imperator_save::{models::MetadataBorrowed, EnvTokens, ImperatorFile, PdsDate};
use std::env;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = env::args().collect();
    let data = std::fs::read(&args[1])?;
    let file = ImperatorFile::from_slice(&data)?;
    let mut zip_sink = Vec::new();
    let file = file.parse(&mut zip_sink)?;
    let meta: MetadataBorrowed = file.deserializer(&EnvTokens).deserialize()?;
    print!("{}", meta.date.game_fmt());
    Ok(())
}
