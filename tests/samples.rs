use core::panic;
use imperator_save::{
    file::{ImperatorFile, ImperatorFsFileKind, ImperatorSliceFileKind},
    models::{GameState, Metadata},
    BasicTokenResolver, Encoding, MeltOptions,
};
use jomini::binary::TokenResolver;
use std::{io::Cursor, sync::LazyLock};

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
    let data = utils::inflate(utils::request_file("debug-save.zip"));

    let file = ImperatorFile::from_slice(&data).unwrap();
    assert_eq!(file.encoding(), Encoding::Text);
    let ImperatorSliceFileKind::Text(text) = file.kind() else {
        panic!("Expected a text file");
    };

    let save: Metadata = text.deserializer().deserialize().unwrap();
    assert_eq!(save.version, String::from("1.4.2"));

    let save: GameState = text.deserializer().deserialize().unwrap();
    assert_eq!(save.speed, 2);
}

#[test]
fn test_observer_save() {
    skip_if_no_tokens!();
    let file = utils::request_file("observer1.5.rome");
    let mut file = ImperatorFile::from_file(file).unwrap();
    assert_eq!(file.encoding(), Encoding::BinaryZip);

    let ImperatorFsFileKind::Zip(zip) = file.kind() else {
        panic!("Expected a zip file");
    };

    let save: Metadata = zip
        .meta()
        .unwrap()
        .deserializer(&*TOKENS)
        .deserialize()
        .unwrap();
    assert_eq!(save.version, String::from("1.5.3"));

    let save = file.parse_save(&*TOKENS).unwrap();
    assert_eq!(file.encoding(), Encoding::BinaryZip);
    assert_eq!(save.meta.version, String::from("1.5.3"));
}

#[test]
fn test_observer_melt() {
    skip_if_no_tokens!();
    let melt = utils::inflate(utils::request_file("observer1.5_melted.rome.zip"));
    let file = utils::request_file("observer1.5.rome");
    let mut file = ImperatorFile::from_file(file).unwrap();
    let mut out = Cursor::new(Vec::new());
    let options = MeltOptions::new();
    file.melt(options, &*TOKENS, &mut out).unwrap();
    assert_eq!(
        &melt[..],
        out.get_ref(),
        "observer 1.5 did not melt correctly"
    );
}

#[test]
fn test_patch_20() {
    skip_if_no_tokens!();
    let file = utils::request_file("Oponia.rome");
    let mut file = ImperatorFile::from_file(file).unwrap();
    assert_eq!(file.encoding(), Encoding::BinaryZip);

    let ImperatorFsFileKind::Zip(zip) = file.kind() else {
        panic!("Expected a zip file");
    };

    let save: Metadata = zip
        .meta()
        .unwrap()
        .deserializer(&*TOKENS)
        .deserialize()
        .unwrap();
    assert_eq!(save.version, String::from("2.0.5"));

    let save = file.parse_save(&*TOKENS).unwrap();
    assert_eq!(save.meta.version, String::from("2.0.5"));
}

#[test]
fn test_non_ascii_save() {
    skip_if_no_tokens!();
    let file = utils::request_file("non-ascii.rome");
    let mut file = ImperatorFile::from_file(file).unwrap();
    let save = file.parse_save(&*TOKENS).unwrap();
    assert_eq!(file.encoding(), Encoding::BinaryZip);
    assert_eq!(save.meta.version, String::from("1.5.3"));
}

#[test]
fn test_roundtrip_header_melt() {
    skip_if_no_tokens!();
    let data = include_bytes!("fixtures/header");
    let file = ImperatorFile::from_slice(&data[..]).unwrap();

    let mut out = Cursor::new(Vec::new());
    let options = MeltOptions::new();
    file.melt(options, &*TOKENS, &mut out).unwrap();

    let file = ImperatorFile::from_slice(&out.get_ref()).unwrap();
    let ImperatorSliceFileKind::Text(text) = file.kind() else {
        panic!("Expected a text file");
    };
    let meta: Metadata = text.deserializer().deserialize().unwrap();

    assert_eq!(file.encoding(), Encoding::Text);
    assert_eq!(meta.version, String::from("1.5.3"));
}

#[test]
fn test_header_melt() {
    skip_if_no_tokens!();
    let data = include_bytes!("fixtures/header");
    let melted = include_bytes!("fixtures/header.melted");

    let file = ImperatorFile::from_slice(&data[..]).unwrap();
    let mut out = Cursor::new(Vec::new());
    let options = MeltOptions::new();
    file.melt(options, &*TOKENS, &mut out).unwrap();
    assert_eq!(&melted[..], out.get_ref());
}
