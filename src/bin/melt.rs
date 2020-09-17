use std::env;
use std::io::Write;

fn main() {
    let args: Vec<String> = env::args().collect();
    let file_data = std::fs::read(&args[1]).unwrap();
    let melted = imperator_save::Melter::new()
        .with_on_failed_resolve(imperator_save::FailedResolveStrategy::Stringify)
        .melt(&file_data[..])
        .unwrap();

    let stdout = std::io::stdout();
    let mut handle = stdout.lock();
    handle.write_all(&melted[..]).unwrap();
}
