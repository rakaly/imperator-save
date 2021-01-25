#![cfg(ironman)]
use imperator_save::{Encoding, FailedResolveStrategy, ImperatorExtractor};
use std::io::{Cursor, Read};

mod utils;

#[test]
fn test_debug_save() {
    let data = utils::request("debug-save.zip");
    let reader = Cursor::new(&data[..]);
    let mut zip = zip::ZipArchive::new(reader).unwrap();
    let mut zip_file = zip.by_index(0).unwrap();
    let mut buffer = Vec::with_capacity(0);
    zip_file.read_to_end(&mut buffer).unwrap();

    let (save, encoding) = ImperatorExtractor::builder()
        .with_on_failed_resolve(FailedResolveStrategy::Error)
        .extract_header_owned(&buffer[..])
        .unwrap();
    assert_eq!(encoding, Encoding::Debug);
    assert_eq!(save.version, String::from("1.4.2"));

    let (save, encoding) = ImperatorExtractor::builder()
        .with_on_failed_resolve(FailedResolveStrategy::Error)
        .extract_header_borrowed(&buffer[..])
        .unwrap();
    assert_eq!(encoding, Encoding::Debug);
    assert_eq!(save.version, String::from("1.4.2"));

    let reader = Cursor::new(&buffer[..]);
    let (save, encoding) = ImperatorExtractor::builder()
        .with_on_failed_resolve(FailedResolveStrategy::Error)
        .extract_save(reader)
        .unwrap();
    assert_eq!(encoding, Encoding::Debug);
    assert_eq!(save.header.version, String::from("1.4.2"));
}

#[test]
fn test_observer_save() {
    let data = utils::request("observer1.5.rome");

    let (save, encoding) = ImperatorExtractor::builder()
        .with_on_failed_resolve(FailedResolveStrategy::Error)
        .extract_header_owned(&data[..])
        .unwrap();
    assert_eq!(encoding, Encoding::Standard);
    assert_eq!(save.version, String::from("1.5.3"));

    let (save, encoding) = ImperatorExtractor::builder()
        .with_on_failed_resolve(FailedResolveStrategy::Error)
        .extract_header_borrowed(&data[..])
        .unwrap();
    assert_eq!(encoding, Encoding::Standard);
    assert_eq!(save.version, String::from("1.5.3"));

    let reader = Cursor::new(&data[..]);
    let (save, encoding) = ImperatorExtractor::builder()
        .with_on_failed_resolve(FailedResolveStrategy::Error)
        .extract_save(reader)
        .unwrap();
    assert_eq!(encoding, Encoding::Standard);
    assert_eq!(save.header.version, String::from("1.5.3"));
}

#[test]
fn test_non_ascii_save() -> Result<(), Box<dyn std::error::Error>> {
    let data = utils::request("non-ascii.rome");
    let reader = Cursor::new(&data[..]);
    let (save, encoding) = ImperatorExtractor::builder()
        .with_on_failed_resolve(FailedResolveStrategy::Error)
        .extract_save(reader)?;
    assert_eq!(encoding, Encoding::Standard);
    assert_eq!(save.header.version, String::from("1.5.3"));
    Ok(())
}

#[test]
fn test_roundtrip_header_melt() {
    let data = include_bytes!("fixtures/header");
    let (out, _tokens) = imperator_save::Melter::new().melt(&data[..]).unwrap();
    let (header, encoding) = ImperatorExtractor::extract_header(&out).unwrap();
    assert_eq!(encoding, Encoding::Debug);
    assert_eq!(header.version, String::from("1.5.3"));
}

#[test]
fn test_header_melt() {
    let data = include_bytes!("fixtures/header");
    let melted = include_bytes!("fixtures/header.melted");
    let (out, _tokens) = imperator_save::Melter::new().melt(&data[..]).unwrap();
    assert_eq!(&melted[..], &out[..]);
}
