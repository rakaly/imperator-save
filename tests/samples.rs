use imperator_save::{
    file::ImperatorFile,
    models::{MetadataBorrowed, MetadataOwned, Save},
    BasicTokenResolver, Encoding, FailedResolveStrategy,
};
use jomini::binary::TokenResolver;
use std::{
    io::{Cursor, Read},
    sync::LazyLock,
};

mod utils;

static TOKENS: LazyLock<BasicTokenResolver> = LazyLock::new(|| {
    let file_data = std::fs::read("assets/imperator.txt").unwrap_or_default();
    BasicTokenResolver::from_text_lines(file_data.as_slice()).unwrap()
});

macro_rules! skip_if_no_tokens {
    () => {
        if TOKENS.is_empty() {
            return;
        }
    };
}

#[test]
fn test_debug_save() {
    skip_if_no_tokens!();
    let data = utils::request("debug-save.zip");
    let reader = Cursor::new(&data[..]);
    let mut zip = zip::ZipArchive::new(reader).unwrap();
    let mut zip_file = zip.by_index(0).unwrap();
    let mut buffer = Vec::with_capacity(0);
    zip_file.read_to_end(&mut buffer).unwrap();

    let file = ImperatorFile::from_slice(&buffer[..]).unwrap();
    let meta = file.meta();
    let parsed_metadata = meta.parse().unwrap();
    let save: MetadataOwned = parsed_metadata
        .deserializer(&*TOKENS)
        .deserialize()
        .unwrap();

    assert_eq!(file.encoding(), Encoding::Text);
    assert_eq!(save.version, String::from("1.4.2"));

    let save: MetadataBorrowed = parsed_metadata
        .deserializer(&*TOKENS)
        .deserialize()
        .unwrap();
    assert_eq!(file.encoding(), Encoding::Text);
    assert_eq!(save.version, String::from("1.4.2"));

    let mut zip_sink = Vec::new();
    let parsed_file = file.parse(&mut zip_sink).unwrap();
    let save = Save::from_deserializer(&parsed_file.deserializer(&*TOKENS)).unwrap();
    assert_eq!(file.encoding(), Encoding::Text);
    assert_eq!(save.meta.version, String::from("1.4.2"));
}

#[test]
fn test_observer_save() {
    skip_if_no_tokens!();
    let data = utils::request("observer1.5.rome");
    let file = ImperatorFile::from_slice(&data[..]).unwrap();
    let meta = file.meta();
    let parsed_metadata = meta.parse().unwrap();
    let save: MetadataOwned = parsed_metadata
        .deserializer(&*TOKENS)
        .deserialize()
        .unwrap();

    assert_eq!(file.encoding(), Encoding::BinaryZip);
    assert_eq!(save.version, String::from("1.5.3"));

    let save: MetadataBorrowed = parsed_metadata
        .deserializer(&*TOKENS)
        .deserialize()
        .unwrap();
    assert_eq!(file.encoding(), Encoding::BinaryZip);
    assert_eq!(save.version, String::from("1.5.3"));

    let mut zip_sink = Vec::new();
    let parsed_file = file.parse(&mut zip_sink).unwrap();
    let save = Save::from_deserializer(
        &parsed_file
            .deserializer(&*TOKENS)
            .on_failed_resolve(FailedResolveStrategy::Error),
    )
    .unwrap();
    assert_eq!(file.encoding(), Encoding::BinaryZip);
    assert_eq!(save.meta.version, String::from("1.5.3"));
}

#[test]
fn test_observer_melt() {
    skip_if_no_tokens!();
    let melt = utils::request("observer1.5_melted.rome.zip");
    let reader = Cursor::new(melt.as_slice());
    let mut zip = zip::ZipArchive::new(reader).unwrap();
    let mut file = zip.by_index(0).unwrap();
    let mut melted = Vec::new();
    file.read_to_end(&mut melted).unwrap();

    let data = utils::request("observer1.5.rome");
    let file = ImperatorFile::from_slice(&data[..]).unwrap();
    let mut out = Cursor::new(Vec::new());
    file.melter().melt(&mut out, &*TOKENS).unwrap();
    assert!(
        eq(&out.into_inner(), &melted),
        "patch 1.5 did not melt currently"
    );
}

fn eq(a: &[u8], b: &[u8]) -> bool {
    for (ai, bi) in a.iter().zip(b.iter()) {
        if ai != bi {
            return false;
        }
    }

    a.len() == b.len()
}

#[test]
fn test_non_ascii_save() {
    skip_if_no_tokens!();
    let data = utils::request("non-ascii.rome");
    let file = ImperatorFile::from_slice(&data[..]).unwrap();
    let mut zip_sink = Vec::new();
    let parsed_file = file.parse(&mut zip_sink).unwrap();
    let save = Save::from_deserializer(&parsed_file.deserializer(&*TOKENS)).unwrap();
    assert_eq!(file.encoding(), Encoding::BinaryZip);
    assert_eq!(save.meta.version, String::from("1.5.3"));
}

#[test]
fn test_roundtrip_header_melt() {
    skip_if_no_tokens!();
    let data = include_bytes!("fixtures/header");
    let file = ImperatorFile::from_slice(&data[..]).unwrap();

    let mut out = Cursor::new(Vec::new());
    file.melter().melt(&mut out, &*TOKENS).unwrap();

    let file = ImperatorFile::from_slice(&out.get_ref()).unwrap();
    let mut zip_sink = Vec::new();
    let parsed_file = file.parse(&mut zip_sink).unwrap();
    let meta: MetadataOwned = parsed_file.deserializer(&*TOKENS).deserialize().unwrap();

    assert_eq!(file.encoding(), Encoding::Text);
    assert_eq!(meta.version, String::from("1.5.3"));
}

#[test]
fn test_header_melt() {
    skip_if_no_tokens!();
    let data = include_bytes!("fixtures/header");
    let melted = include_bytes!("fixtures/header.melted");

    let file = ImperatorFile::from_slice(&data[..]).unwrap();
    let meta = file.meta();
    let mut out = Cursor::new(Vec::new());
    meta.melter().melt(&mut out, &*TOKENS).unwrap();
    assert_eq!(&melted[..], out.get_ref());
}
