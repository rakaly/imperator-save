use imperator_save::{models::Save, BasicTokenResolver, ImperatorFile, PdsDate};
use std::env;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = env::args().collect();
    let file = std::fs::File::open(&args[1])?;
    let mut file = ImperatorFile::from_file(file)?;
    let file_data = std::fs::read("assets/imperator.txt").unwrap_or_default();
    let resolver = BasicTokenResolver::from_text_lines(file_data.as_slice())?;
    let game = Save::from_file(&mut file, &resolver)?;
    print!("{}", game.meta.date.game_fmt());
    Ok(())
}
