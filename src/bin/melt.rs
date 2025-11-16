use imperator_save::{
    BasicTokenResolver, FailedResolveStrategy, ImperatorFile, ImperatorMelt, MeltOptions,
};
use std::env;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = env::args().collect();
    let data = std::fs::read(&args[1])?;
    let mut file = ImperatorFile::from_slice(&data)?;
    let file_data = std::fs::read("assets/imperator.txt").unwrap_or_default();
    let resolver = BasicTokenResolver::from_text_lines(file_data.as_slice())?;
    let stdout = std::io::stdout();
    let handle = stdout.lock();
    let mut writer = std::io::BufWriter::new(handle);
    let options = MeltOptions::new().on_failed_resolve(FailedResolveStrategy::Error);
    file.melt(options, resolver, &mut writer)?;
    Ok(())
}
