use imperator_save::{EnvTokens, FailedResolveStrategy, ImperatorFile};
use std::env;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = env::args().collect();
    let data = std::fs::read(&args[1])?;
    let file = ImperatorFile::from_slice(&data)?;
    let stdout = std::io::stdout();
    let handle = stdout.lock();
    file.melter()
        .on_failed_resolve(FailedResolveStrategy::Error)
        .melt(handle, &EnvTokens)
        .unwrap();
    Ok(())
}
