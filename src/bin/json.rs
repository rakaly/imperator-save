use imperator_save::{
    file::{ImperatorFsFileKind, ImperatorParsedText},
    BasicTokenResolver, ImperatorFile,
};
use std::{env, error::Error, io::Read};

fn json_to_stdout(file: &ImperatorParsedText) {
    let stdout = std::io::stdout();
    let _ = file.reader().json().to_writer(stdout.lock());
}

fn main() -> Result<(), Box<dyn Error>> {
    let args: Vec<String> = env::args().collect();
    let file = std::fs::File::open(&args[1])?;
    let mut file = ImperatorFile::from_file(file)?;

    let file_data = std::fs::read("assets/imperator.txt").unwrap_or_default();
    let resolver = BasicTokenResolver::from_text_lines(file_data.as_slice())?;

    let melt_options = imperator_save::MeltOptions::new();
    match file.kind_mut() {
        ImperatorFsFileKind::Text(x) => {
            let mut buf = Vec::new();
            x.read_to_end(&mut buf)?;
            let text = ImperatorParsedText::from_raw(&buf)?;
            json_to_stdout(&text);
        }
        ImperatorFsFileKind::Binary(x) => {
            let mut buf = Vec::new();
            x.melt(melt_options, resolver, &mut buf)?;
            let text = ImperatorParsedText::from_slice(&buf)?;
            json_to_stdout(&text);
        }
        ImperatorFsFileKind::Zip(x) => {
            let mut data = Vec::new();
            x.melt(melt_options, resolver, &mut data)?;
            let text = ImperatorParsedText::from_slice(&data)?;
            json_to_stdout(&text);
        }
    }

    Ok(())
}
