use imperator_save::{models::MetadataBorrowed, BasicTokenResolver, ImperatorFile, PdsDate};
use std::env;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = env::args().collect();
    let data = std::fs::read(&args[1])?;
    let file = ImperatorFile::from_slice(&data)?;
    let mut zip_sink = Vec::new();
    let file = file.parse(&mut zip_sink)?;
    let file_data = std::fs::read("assets/imperator.txt").unwrap_or_default();
    let resolver = BasicTokenResolver::from_text_lines(file_data.as_slice())?;
    let meta: MetadataBorrowed = file.deserializer(&resolver).deserialize()?;
    print!("{}", meta.date.game_fmt());
    Ok(())
}
