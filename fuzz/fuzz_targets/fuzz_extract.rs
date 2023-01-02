#![no_main]
use imperator_save::{
    models::{MetadataOwned, Save},
    EnvTokens,
};
use libfuzzer_sys::fuzz_target;

fn run(data: &[u8]) -> Result<(), Box<dyn std::error::Error>> {
    let file = imperator_save::ImperatorFile::from_slice(&data)?;

    let meta = file.parse_metadata()?;
    let _meta: Result<MetadataOwned, _> = meta.deserializer(&EnvTokens).deserialize();

    let mut zip_sink = Vec::new();
    let parsed_file = file.parse(&mut zip_sink)?;

    match parsed_file.kind() {
        imperator_save::file::ImperatorParsedFileKind::Text(x) => {
            x.reader().json().to_writer(std::io::sink())?;
        }
        imperator_save::file::ImperatorParsedFileKind::Binary(x) => {
            x.melter().melt(&EnvTokens)?;
        }
    }

    let _game = Save::from_deserializer(&parsed_file.deserializer(&EnvTokens));

    Ok(())
}

fuzz_target!(|data: &[u8]| {
    let _ = run(data);
});
