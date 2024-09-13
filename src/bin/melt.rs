use imperator_save::{BasicTokenResolver, FailedResolveStrategy, ImperatorFile};
use std::env;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = env::args().collect();
    let data = std::fs::read(&args[1])?;
    let file = ImperatorFile::from_slice(&data)?;
    let file_data = std::fs::read("assets/imperator.txt").unwrap_or_default();
    let resolver = BasicTokenResolver::from_text_lines(file_data.as_slice())?;
    let stdout = std::io::stdout();
    let handle = stdout.lock();
    file.melter()
        .on_failed_resolve(FailedResolveStrategy::Error)
        .melt(handle, &resolver)
        .unwrap();
    Ok(())
}
