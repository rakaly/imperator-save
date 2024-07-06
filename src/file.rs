use crate::{
    flavor::ImperatorFlavor, Encoding, ImperatorError, ImperatorErrorKind, ImperatorMelter,
    SaveHeader,
};
use jomini::{
    binary::{FailedResolveStrategy, TokenResolver},
    text::ObjectReader,
    BinaryDeserializer, BinaryTape, TextDeserializer, TextTape, Utf8Encoding,
};
use serde::Deserialize;
use std::io::Cursor;
use zip::result::ZipError;

#[derive(Clone, Debug)]
pub(crate) struct ImperatorZip<'a> {
    pub(crate) archive: ImperatorZipFiles<'a>,
    pub(crate) metadata: &'a [u8],
    pub(crate) gamestate: VerifiedIndex,
    pub(crate) is_text: bool,
}

enum FileKind<'a> {
    Text(&'a [u8]),
    Binary(&'a [u8]),
    Zip(ImperatorZip<'a>),
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
            Ok(mut zip) => {
                let metadata = &data[..zip.offset() as usize];
                let files = ImperatorZipFiles::new(&mut zip, data);
                let gamestate_idx = files
                    .gamestate_index()
                    .ok_or(ImperatorErrorKind::ZipMissingEntry)?;

                let is_text = !header.kind().is_binary();
                Ok(ImperatorFile {
                    header,
                    kind: FileKind::Zip(ImperatorZip {
                        archive: files,
                        gamestate: gamestate_idx,
                        metadata,
                        is_text,
                    }),
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

    /// Return first line header
    pub fn header(&self) -> &SaveHeader {
        &self.header
    }

    /// Returns the detected decoding of the file
    pub fn encoding(&self) -> Encoding {
        match &self.kind {
            FileKind::Text(_) => Encoding::Text,
            FileKind::Binary(_) => Encoding::Binary,
            FileKind::Zip(ImperatorZip { is_text: true, .. }) => Encoding::TextZip,
            FileKind::Zip(ImperatorZip { is_text: false, .. }) => Encoding::BinaryZip,
        }
    }

    /// Returns the size of the file
    ///
    /// The size includes the inflated size of the zip
    pub fn size(&self) -> usize {
        match &self.kind {
            FileKind::Text(x) | FileKind::Binary(x) => x.len(),
            FileKind::Zip(ImperatorZip { gamestate, .. }) => gamestate.size,
        }
    }

    pub fn meta(&self) -> ImperatorMeta<'a> {
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

                ImperatorMeta {
                    kind: ImperatorMetaKind::Text(data),
                    header: self.header.clone(),
                }
            }
            FileKind::Binary(x) => {
                let metadata = x.get(..self.header.metadata_len() as usize).unwrap_or(x);
                ImperatorMeta {
                    kind: ImperatorMetaKind::Binary(metadata),
                    header: self.header.clone(),
                }
            }
            FileKind::Zip(ImperatorZip {
                metadata,
                is_text: true,
                ..
            }) => ImperatorMeta {
                kind: ImperatorMetaKind::Text(metadata),
                header: self.header.clone(),
            },
            FileKind::Zip(ImperatorZip { metadata, .. }) => ImperatorMeta {
                kind: ImperatorMetaKind::Binary(metadata),
                header: self.header.clone(),
            },
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
            FileKind::Zip(ImperatorZip {
                archive,
                gamestate,
                is_text,
                ..
            }) => {
                let zip = archive.retrieve_file(*gamestate);
                zip.read_to_end(zip_sink)?;

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

    pub fn melter(&self) -> ImperatorMelter<'a> {
        match &self.kind {
            FileKind::Text(x) => ImperatorMelter::new_text(x, self.header.clone()),
            FileKind::Binary(x) => ImperatorMelter::new_binary(x, self.header.clone()),
            FileKind::Zip(x) => ImperatorMelter::new_zip((*x).clone(), self.header.clone()),
        }
    }
}

/// Holds the metadata section of the save
#[derive(Debug)]
pub struct ImperatorMeta<'a> {
    kind: ImperatorMetaKind<'a>,
    header: SaveHeader,
}

/// Describes the format of the metadata section of the save
#[derive(Debug)]
pub enum ImperatorMetaKind<'a> {
    Text(&'a [u8]),
    Binary(&'a [u8]),
}

impl<'a> ImperatorMeta<'a> {
    pub fn header(&self) -> &SaveHeader {
        &self.header
    }

    pub fn kind(&self) -> &ImperatorMetaKind {
        &self.kind
    }

    pub fn parse(&self) -> Result<ImperatorParsedFile<'a>, ImperatorError> {
        match self.kind {
            ImperatorMetaKind::Text(x) => {
                ImperatorText::from_raw(x).map(|kind| ImperatorParsedFile {
                    kind: ImperatorParsedFileKind::Text(kind),
                })
            }

            ImperatorMetaKind::Binary(x) => {
                ImperatorBinary::from_raw(x, self.header.clone()).map(|kind| ImperatorParsedFile {
                    kind: ImperatorParsedFileKind::Binary(kind),
                })
            }
        }
    }

    pub fn melter(&self) -> ImperatorMelter<'a> {
        match self.kind {
            ImperatorMetaKind::Text(x) => ImperatorMelter::new_text(x, self.header.clone()),
            ImperatorMetaKind::Binary(x) => ImperatorMelter::new_binary(x, self.header.clone()),
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
    pub fn deserializer<'b, RES>(&'b self, resolver: &'b RES) -> ImperatorDeserializer<RES>
    where
        RES: TokenResolver,
    {
        match &self.kind {
            ImperatorParsedFileKind::Text(x) => ImperatorDeserializer {
                kind: ImperatorDeserializerKind::Text(x),
            },
            ImperatorParsedFileKind::Binary(x) => ImperatorDeserializer {
                kind: ImperatorDeserializerKind::Binary(x.deserializer(resolver)),
            },
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct VerifiedIndex {
    data_start: usize,
    data_end: usize,
    size: usize,
}

#[derive(Debug, Clone)]
pub(crate) struct ImperatorZipFiles<'a> {
    archive: &'a [u8],
    gamestate_index: Option<VerifiedIndex>,
}

impl<'a> ImperatorZipFiles<'a> {
    pub fn new(archive: &mut zip::ZipArchive<Cursor<&'a [u8]>>, data: &'a [u8]) -> Self {
        let mut gamestate_index = None;

        for index in 0..archive.len() {
            if let Ok(file) = archive.by_index_raw(index) {
                let size = file.size() as usize;
                let data_start = file.data_start() as usize;
                let data_end = data_start + file.compressed_size() as usize;

                if file.name() == "gamestate" {
                    gamestate_index = Some(VerifiedIndex {
                        data_start,
                        data_end,
                        size,
                    })
                }
            }
        }

        Self {
            archive: data,
            gamestate_index,
        }
    }

    pub fn retrieve_file(&self, index: VerifiedIndex) -> ImperatorZipFile {
        let raw = &self.archive[index.data_start..index.data_end];
        ImperatorZipFile {
            raw,
            size: index.size,
        }
    }

    pub fn gamestate_index(&self) -> Option<VerifiedIndex> {
        self.gamestate_index
    }
}

pub(crate) struct ImperatorZipFile<'a> {
    raw: &'a [u8],
    size: usize,
}

impl<'a> ImperatorZipFile<'a> {
    pub fn read_to_end(&self, buf: &mut Vec<u8>) -> Result<(), ImperatorError> {
        let start_len = buf.len();
        buf.resize(start_len + self.size(), 0);
        let body = &mut buf[start_len..];
        crate::deflate::inflate_exact(self.raw, body).map_err(ImperatorErrorKind::from)?;
        Ok(())
    }

    pub fn reader(&self) -> crate::deflate::DeflateReader<'a> {
        crate::deflate::DeflateReader::new(self.raw, crate::deflate::CompressionMethod::Deflate)
    }

    pub fn size(&self) -> usize {
        self.size
    }
}

/// A parsed Imperator text document
pub struct ImperatorText<'a> {
    tape: TextTape<'a>,
}

impl<'a> ImperatorText<'a> {
    pub fn from_slice(data: &'a [u8]) -> Result<Self, ImperatorError> {
        let header = SaveHeader::from_slice(data)?;
        Self::from_raw(&data[header.header_len()..])
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
        let deser = TextDeserializer::from_utf8_tape(&self.tape);
        let result = deser
            .deserialize()
            .map_err(ImperatorErrorKind::Deserialize)?;
        Ok(result)
    }
}

/// A parsed Imperator binary document
pub struct ImperatorBinary<'data> {
    tape: BinaryTape<'data>,
    #[allow(dead_code)]
    header: SaveHeader,
}

impl<'data> ImperatorBinary<'data> {
    pub fn from_slice(data: &'data [u8]) -> Result<Self, ImperatorError> {
        let header = SaveHeader::from_slice(data)?;
        Self::from_raw(&data[header.header_len()..], header)
    }

    pub(crate) fn from_raw(data: &'data [u8], header: SaveHeader) -> Result<Self, ImperatorError> {
        let tape = BinaryTape::from_slice(data).map_err(ImperatorErrorKind::Parse)?;
        Ok(ImperatorBinary { tape, header })
    }

    pub fn deserializer<'b, RES>(
        &'b self,
        resolver: &'b RES,
    ) -> ImperatorBinaryDeserializer<'data, 'b, RES>
    where
        RES: TokenResolver,
    {
        ImperatorBinaryDeserializer {
            deser: BinaryDeserializer::builder_flavor(ImperatorFlavor)
                .from_tape(&self.tape, resolver),
        }
    }
}

enum ImperatorDeserializerKind<'data, 'tape, RES> {
    Text(&'tape ImperatorText<'data>),
    Binary(ImperatorBinaryDeserializer<'data, 'tape, RES>),
}

/// A deserializer for custom structures
pub struct ImperatorDeserializer<'data, 'tape, RES> {
    kind: ImperatorDeserializerKind<'data, 'tape, RES>,
}

impl<'data, 'tape, RES> ImperatorDeserializer<'data, 'tape, RES>
where
    RES: TokenResolver,
{
    pub fn on_failed_resolve(&mut self, strategy: FailedResolveStrategy) -> &mut Self {
        if let ImperatorDeserializerKind::Binary(x) = &mut self.kind {
            x.on_failed_resolve(strategy);
        }
        self
    }

    pub fn deserialize<T>(&self) -> Result<T, ImperatorError>
    where
        T: Deserialize<'data>,
    {
        match &self.kind {
            ImperatorDeserializerKind::Text(x) => x.deserialize(),
            ImperatorDeserializerKind::Binary(x) => x.deserialize(),
        }
    }
}

/// Deserializes binary data into custom structures
pub struct ImperatorBinaryDeserializer<'data, 'tape, RES> {
    deser: BinaryDeserializer<'tape, 'data, 'tape, RES, ImperatorFlavor>,
}

impl<'data, 'tape, RES> ImperatorBinaryDeserializer<'data, 'tape, RES>
where
    RES: TokenResolver,
{
    pub fn on_failed_resolve(&mut self, strategy: FailedResolveStrategy) -> &mut Self {
        self.deser.on_failed_resolve(strategy);
        self
    }

    pub fn deserialize<T>(&self) -> Result<T, ImperatorError>
    where
        T: Deserialize<'data>,
    {
        let result = self.deser.deserialize().map_err(|e| match e.kind() {
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
