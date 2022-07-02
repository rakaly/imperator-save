use crate::{
    flavor::ImperatorFlavor, ImperatorError, ImperatorErrorKind, ImperatorMelter, SaveHeader, Encoding,
};
use jomini::{
    binary::{BinaryDeserializerBuilder, FailedResolveStrategy, TokenResolver},
    text::ObjectReader,
    BinaryDeserializer, BinaryTape, TextDeserializer, TextTape, Utf8Encoding,
};
use serde::Deserialize;
use std::io::{Cursor, Read};
use zip::{read::ZipFile, result::ZipError};

enum FileKind<'a> {
    Text(&'a [u8]),
    Binary(&'a [u8]),
    Zip {
        archive: zip::ZipArchive<Cursor<&'a [u8]>>,
        metadata: &'a [u8],
        gamestate: VerifiedIndex,
        is_text: bool,
    },
}

/// Entrypoint for parsing Imperator saves
///
/// Only consumes enough data to determine encoding of the file
pub struct ImperatorFile<'a> {
    header: SaveHeader,
    kind: FileKind<'a>,
}

impl<'a> ImperatorFile<'a> {
    /// Creates a Imperator file from a slice of data
    pub fn from_slice(data: &[u8]) -> Result<ImperatorFile, ImperatorError> {
        let header = SaveHeader::from_slice(data)?;
        let data = &data[header.header_len()..];

        let reader = Cursor::new(data);
        match zip::ZipArchive::new(reader) {
            Ok(zip) => {
                let metadata = &data[..zip.offset() as usize];
                let files = ImperatorZipFiles::new(zip);
                let gamestate_idx = files
                    .gamestate_index()
                    .ok_or(ImperatorErrorKind::ZipMissingEntry)?;

                let is_text = !header.kind().is_binary();
                Ok(ImperatorFile {
                    header,
                    kind: FileKind::Zip {
                        archive: files.into_zip(),
                        gamestate: gamestate_idx,
                        metadata,
                        is_text,
                    },
                })
            }
            Err(ZipError::InvalidArchive(_)) => {
                if header.kind().is_binary() {
                    Ok(ImperatorFile {
                        header,
                        kind: FileKind::Binary(data),
                    })
                } else {
                    Ok(ImperatorFile {
                        header,
                        kind: FileKind::Text(data),
                    })
                }
            }
            Err(e) => Err(ImperatorErrorKind::ZipArchive(e).into()),
        }
    }

    /// Returns the detected decoding of the file
    pub fn encoding(&self) -> Encoding {
        match &self.kind {
            FileKind::Text(_) => Encoding::Text,
            FileKind::Binary(_) => Encoding::Binary,
            FileKind::Zip { is_text, .. } if *is_text => Encoding::TextZip,
            FileKind::Zip { .. } => Encoding::BinaryZip,
        }
    }

    /// Returns the size of the file
    ///
    /// The size includes the inflated size of the zip
    pub fn size(&self) -> usize {
        match &self.kind {
            FileKind::Text(x) | FileKind::Binary(x) => x.len(),
            FileKind::Zip { gamestate, .. } => gamestate.size,
        }
    }

    pub fn parse_metadata(&self) -> Result<ImperatorParsedFile<'a>, ImperatorError> {
        match &self.kind {
            FileKind::Text(x) => {
                // The metadata section should be way smaller than the total
                // length so if the total data isn't significantly bigger (2x or
                // more), assume that the header doesn't accurately represent
                // the metadata length. Like maybe someone accidentally
                // converted the line endings from unix to dos.
                let len = self.header.metadata_len() as usize;
                let data = if len * 2 > x.len() {
                    x
                } else {
                    &x[..len.min(x.len())]
                };

                let text = ImperatorText::from_raw(data)?;
                Ok(ImperatorParsedFile {
                    kind: ImperatorParsedFileKind::Text(text),
                })
            }
            FileKind::Binary(x) => {
                let metadata = x.get(..self.header.metadata_len() as usize).unwrap_or(x);
                let binary = ImperatorBinary::from_raw(metadata, self.header.clone())?;
                Ok(ImperatorParsedFile {
                    kind: ImperatorParsedFileKind::Binary(binary),
                })
            }
            FileKind::Zip {
                metadata, is_text, ..
            } if *is_text => {
                let text = ImperatorText::from_raw(metadata)?;
                Ok(ImperatorParsedFile {
                    kind: ImperatorParsedFileKind::Text(text),
                })
            }
            FileKind::Zip { metadata, .. } => {
                let binary = ImperatorBinary::from_raw(metadata, self.header.clone())?;
                Ok(ImperatorParsedFile {
                    kind: ImperatorParsedFileKind::Binary(binary),
                })
            }
        }
    }

    /// Parses the entire file
    ///
    /// If the file is a zip, the zip contents will be inflated into the zip
    /// sink before being parsed
    pub fn parse(
        &self,
        zip_sink: &'a mut Vec<u8>,
    ) -> Result<ImperatorParsedFile<'a>, ImperatorError> {
        match &self.kind {
            FileKind::Text(x) => {
                let text = ImperatorText::from_raw(x)?;
                Ok(ImperatorParsedFile {
                    kind: ImperatorParsedFileKind::Text(text),
                })
            }
            FileKind::Binary(x) => {
                let binary = ImperatorBinary::from_raw(x, self.header.clone())?;
                Ok(ImperatorParsedFile {
                    kind: ImperatorParsedFileKind::Binary(binary),
                })
            }
            FileKind::Zip {
                archive,
                gamestate,
                is_text,
                ..
            } => {
                let mut zip = ImperatorZipFiles::new(archive.clone());
                zip_sink.reserve(gamestate.size);
                zip.retrieve_file(*gamestate).read_to_end(zip_sink)?;

                if *is_text {
                    let text = ImperatorText::from_raw(zip_sink)?;
                    Ok(ImperatorParsedFile {
                        kind: ImperatorParsedFileKind::Text(text),
                    })
                } else {
                    let binary = ImperatorBinary::from_raw(zip_sink, self.header.clone())?;
                    Ok(ImperatorParsedFile {
                        kind: ImperatorParsedFileKind::Binary(binary),
                    })
                }
            }
        }
    }
}

/// Contains the parsed Imperator file
pub enum ImperatorParsedFileKind<'a> {
    /// The Imperator file as text
    Text(ImperatorText<'a>),

    /// The Imperator file as binary
    Binary(ImperatorBinary<'a>),
}

/// An Imperator file that has been parsed
pub struct ImperatorParsedFile<'a> {
    kind: ImperatorParsedFileKind<'a>,
}

impl<'a> ImperatorParsedFile<'a> {
    /// Returns the file as text
    pub fn as_text(&self) -> Option<&ImperatorText> {
        match &self.kind {
            ImperatorParsedFileKind::Text(x) => Some(x),
            _ => None,
        }
    }

    /// Returns the file as binary
    pub fn as_binary(&self) -> Option<&ImperatorBinary> {
        match &self.kind {
            ImperatorParsedFileKind::Binary(x) => Some(x),
            _ => None,
        }
    }

    /// Returns the kind of file (binary or text)
    pub fn kind(&self) -> &ImperatorParsedFileKind {
        &self.kind
    }

    /// Prepares the file for deserialization into a custom structure
    pub fn deserializer(&self) -> ImperatorDeserializer {
        match &self.kind {
            ImperatorParsedFileKind::Text(x) => ImperatorDeserializer {
                kind: ImperatorDeserializerKind::Text(x),
            },
            ImperatorParsedFileKind::Binary(x) => ImperatorDeserializer {
                kind: ImperatorDeserializerKind::Binary(x.deserializer()),
            },
        }
    }
}

#[derive(Debug, Clone, Copy)]
struct VerifiedIndex {
    index: usize,
    size: usize,
}

#[derive(Debug, Clone)]
struct ImperatorZipFiles<'a> {
    archive: zip::ZipArchive<Cursor<&'a [u8]>>,
    gamestate_index: Option<VerifiedIndex>,
}

impl<'a> ImperatorZipFiles<'a> {
    pub fn new(mut archive: zip::ZipArchive<Cursor<&'a [u8]>>) -> Self {
        let mut gamestate_index = None;

        for index in 0..archive.len() {
            if let Ok(file) = archive.by_index(index) {
                let size = file.size() as usize;
                if file.name() == "gamestate" {
                    gamestate_index = Some(VerifiedIndex { index, size })
                }
            }
        }

        Self {
            archive,
            gamestate_index,
        }
    }

    pub fn retrieve_file(&mut self, index: VerifiedIndex) -> ImperatorZipFile {
        let file = self.archive.by_index(index.index).unwrap();
        ImperatorZipFile { file }
    }

    pub fn gamestate_index(&self) -> Option<VerifiedIndex> {
        self.gamestate_index
    }

    pub fn into_zip(self) -> zip::ZipArchive<Cursor<&'a [u8]>> {
        self.archive
    }
}

struct ImperatorZipFile<'a> {
    file: ZipFile<'a>,
}

impl<'a> ImperatorZipFile<'a> {
    fn internal_read_to_end(&mut self, buf: &mut Vec<u8>) -> std::io::Result<usize> {
        buf.reserve(self.size());
        self.file.read_to_end(buf)
    }

    pub fn read_to_end(&mut self, buf: &mut Vec<u8>) -> Result<usize, ImperatorError> {
        let res = self
            .internal_read_to_end(buf)
            .map_err(|e| ImperatorErrorKind::ZipInflation { source: e })?;

        Ok(res)
    }

    pub fn size(&self) -> usize {
        self.file.size() as usize
    }
}

/// A parsed Imperator text document
pub struct ImperatorText<'a> {
    tape: TextTape<'a>,
}

impl<'a> ImperatorText<'a> {
    pub fn from_slice(data: &'a [u8]) -> Result<Self, ImperatorError> {
        let header = SaveHeader::from_slice(data)?;
        Self::from_raw(&data[..header.header_len()])
    }

    pub(crate) fn from_raw(data: &'a [u8]) -> Result<Self, ImperatorError> {
        let tape = TextTape::from_slice(data).map_err(ImperatorErrorKind::Parse)?;
        Ok(ImperatorText { tape })
    }

    pub fn reader(&self) -> ObjectReader<Utf8Encoding> {
        self.tape.utf8_reader()
    }

    pub fn deserialize<T>(&self) -> Result<T, ImperatorError>
    where
        T: Deserialize<'a>,
    {
        let result = TextDeserializer::from_utf8_tape(&self.tape)
            .map_err(ImperatorErrorKind::Deserialize)?;
        Ok(result)
    }
}

/// A parsed Imperator binary document
pub struct ImperatorBinary<'a> {
    tape: BinaryTape<'a>,
    header: SaveHeader,
}

impl<'a> ImperatorBinary<'a> {
    pub fn from_slice(data: &'a [u8]) -> Result<Self, ImperatorError> {
        let header = SaveHeader::from_slice(data)?;
        Self::from_raw(&data[..header.header_len()], header)
    }

    pub(crate) fn from_raw(data: &'a [u8], header: SaveHeader) -> Result<Self, ImperatorError> {
        let tape = BinaryTape::from_slice(data).map_err(ImperatorErrorKind::Parse)?;
        Ok(ImperatorBinary { tape, header })
    }

    pub fn deserializer<'b>(&'b self) -> ImperatorBinaryDeserializer<'a, 'b> {
        ImperatorBinaryDeserializer {
            builder: BinaryDeserializer::builder_flavor(ImperatorFlavor),
            tape: &self.tape,
        }
    }

    pub fn melter<'b>(&'b self) -> ImperatorMelter<'a, 'b> {
        ImperatorMelter::new(&self.tape, &self.header)
    }
}

enum ImperatorDeserializerKind<'a, 'b> {
    Text(&'b ImperatorText<'a>),
    Binary(ImperatorBinaryDeserializer<'a, 'b>),
}

/// A deserializer for custom structures
pub struct ImperatorDeserializer<'a, 'b> {
    kind: ImperatorDeserializerKind<'a, 'b>,
}

impl<'a, 'b> ImperatorDeserializer<'a, 'b> {
    pub fn on_failed_resolve(&mut self, strategy: FailedResolveStrategy) -> &mut Self {
        if let ImperatorDeserializerKind::Binary(x) = &mut self.kind {
            x.on_failed_resolve(strategy);
        }
        self
    }

    pub fn build<T, R>(&self, resolver: &'a R) -> Result<T, ImperatorError>
    where
        R: TokenResolver,
        T: Deserialize<'a>,
    {
        match &self.kind {
            ImperatorDeserializerKind::Text(x) => x.deserialize(),
            ImperatorDeserializerKind::Binary(x) => x.build(resolver),
        }
    }
}

/// Deserializes binary data into custom structures
pub struct ImperatorBinaryDeserializer<'a, 'b> {
    builder: BinaryDeserializerBuilder<ImperatorFlavor>,
    tape: &'b BinaryTape<'a>,
}

impl<'a, 'b> ImperatorBinaryDeserializer<'a, 'b> {
    pub fn on_failed_resolve(&mut self, strategy: FailedResolveStrategy) -> &mut Self {
        self.builder.on_failed_resolve(strategy);
        self
    }

    pub fn build<T, R>(&self, resolver: &'a R) -> Result<T, ImperatorError>
    where
        R: TokenResolver,
        T: Deserialize<'a>,
    {
        let result = self
            .builder
            .from_tape(self.tape, resolver)
            .map_err(|e| match e.kind() {
                jomini::ErrorKind::Deserialize(e2) => match e2.kind() {
                    &jomini::DeserializeErrorKind::UnknownToken { token_id } => {
                        ImperatorErrorKind::UnknownToken { token_id }
                    }
                    _ => ImperatorErrorKind::Deserialize(e),
                },
                _ => ImperatorErrorKind::Deserialize(e),
            })?;
        Ok(result)
    }
}
