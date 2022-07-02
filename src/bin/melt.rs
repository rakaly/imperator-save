use imperator_save::{EnvTokens, FailedResolveStrategy, ImperatorFile};
use std::env;
use std::io::Write;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = env::args().collect();
    let data = std::fs::read(&args[1])?;
    let file = ImperatorFile::from_slice(&data)?;
    let mut zip_sink = Vec::new();
    let file = file.parse(&mut zip_sink)?;
    let binary = file.as_binary().unwrap();
    let melted = binary
        .melter()
        .on_failed_resolve(FailedResolveStrategy::Error)
        .melt(&EnvTokens)?;

    let stdout = std::io::stdout();
    let mut handle = stdout.lock();
    let _ = handle.write_all(melted.data());
    Ok(())
}
