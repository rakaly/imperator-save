use imperator_save::{file::ImperatorText, EnvTokens, ImperatorFile};
use std::{env, io::Cursor};

fn json_to_stdout(file: &ImperatorText) {
    let _ = file.reader().json().to_writer(std::io::stdout());
}

fn parsed_file_to_json(file: &ImperatorFile) -> Result<(), Box<dyn std::error::Error>> {
    let mut out = Cursor::new(Vec::new());
    file.melter().verbatim(true).melt(&mut out, &EnvTokens)?;
    json_to_stdout(&ImperatorText::from_slice(out.get_ref())?);
    Ok(())
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = env::args().collect();
    let data = std::fs::read(&args[1]).unwrap();

    let file = ImperatorFile::from_slice(&data)?;
    parsed_file_to_json(&file)?;

    Ok(())
}
