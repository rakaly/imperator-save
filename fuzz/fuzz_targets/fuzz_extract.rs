#![no_main]
use imperator_save::{BasicTokenResolver, models::{MetadataOwned, Save}};
use libfuzzer_sys::fuzz_target;
use std::io::Cursor;

fn run(data: &[u8]) -> Result<(), Box<dyn std::error::Error>> {
    let file_data = std::fs::read("assets/imperator.txt").unwrap();
    let resolver = BasicTokenResolver::from_text_lines(file_data.as_slice())?;

    let file = imperator_save::ImperatorFile::from_slice(&data)?;

    let meta = file.meta().parse()?;
    let _meta: Result<MetadataOwned, _> = meta.deserializer(&resolver).deserialize();

    let mut zip_sink = Vec::new();
    let parsed_file = file.parse(&mut zip_sink)?;
    let mut out = Cursor::new(Vec::new());
    file.melter().melt(&mut out, &resolver)?;
    let _game = Save::from_deserializer(&parsed_file.deserializer(&resolver));

    Ok(())
}

fuzz_target!(|data: &[u8]| {
    let _ = run(data);
});
