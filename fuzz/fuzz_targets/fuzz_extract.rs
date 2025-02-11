#![no_main]
use imperator_save::{file::ImperatorParsedText, BasicTokenResolver};
use libfuzzer_sys::fuzz_target;
use std::sync::LazyLock;

static TOKENS: LazyLock<BasicTokenResolver> = LazyLock::new(|| {
    let file_data = std::fs::read("assets/imperator.txt").unwrap();
    BasicTokenResolver::from_text_lines(file_data.as_slice()).unwrap()
});

fn run(data: &[u8]) -> Result<(), Box<dyn std::error::Error>> {
    let file = imperator_save::ImperatorFile::from_slice(&data)?;

    let mut sink = std::io::sink();
    let _ = file.melt(imperator_save::MeltOptions::new(), &*TOKENS, &mut sink);
    let _ = file.parse_save(&*TOKENS);
    let _ = file.encoding();

    match file.kind() {
        imperator_save::file::ImperatorSliceFileKind::Text(x) => {
            ImperatorParsedText::from_raw(x.get_ref())?
                .reader()
                .json()
                .to_writer(std::io::sink())?;
        }
        _ => {}
    }

    Ok(())
}

fuzz_target!(|data: &[u8]| {
    let _ = run(data);
});
