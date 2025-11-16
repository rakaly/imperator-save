use imperator_save::{BasicTokenResolver, DeserializeImperator, ImperatorFile, PdsDate, models::Save};
use std::env;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = env::args().collect();
    let file = std::fs::File::open(&args[1])?;
    let file = ImperatorFile::from_file(file)?;
    let file_data = std::fs::read("assets/imperator.txt").unwrap_or_default();
    let resolver = BasicTokenResolver::from_text_lines(file_data.as_slice())?;
    let game: Save = (&file).deserialize(resolver)?;
    print!("{}", game.meta.date.game_fmt());
    Ok(())
}
