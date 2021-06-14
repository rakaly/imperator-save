use imperator_save::{ImperatorExtractor, PdsDate};
use std::env;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = env::args().collect();
    let data = std::fs::read(&args[1])?;
    let (save, _encoding) = ImperatorExtractor::extract_header(&data[..])?;
    print!("{}", save.date.game_fmt());
    Ok(())
}
