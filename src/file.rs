use crate::{
    flavor::ImperatorFlavor,
    melt,
    models::{GameState, Save},
    Encoding, ImperatorError, ImperatorErrorKind, MeltOptions, MeltedDocument, SaveHeader,
};
use jomini::{binary::TokenResolver, text::ObjectReader, TextDeserializer, TextTape, Utf8Encoding};
use rawzip::{FileReader, ReaderAt, ZipArchiveEntryWayfinder, ZipVerifier};
use serde::de::DeserializeOwned;
use std::{
    collections::HashMap,
    fs::File,
    io::{Cursor, Read, Seek, Write},
    ops::Range,
};

/// Entrypoint for parsing Imperator saves
///
/// Only consumes enough data to determine encoding of the file
pub struct ImperatorFile {}

impl ImperatorFile {
    /// Creates a Imperator file from a slice of data
    pub fn from_slice(data: &[u8]) -> Result<ImperatorSliceFile, ImperatorError> {
        let header = SaveHeader::from_slice(data)?;
        let data = &data[header.header_len()..];

        let archive = rawzip::ZipArchive::with_max_search_space(64 * 1024)
            .locate_in_slice(data)
            .map_err(ImperatorErrorKind::Zip);

        match archive {
            Ok(archive) => {
                let archive = archive.into_owned();
                let mut buf = vec![0u8; rawzip::RECOMMENDED_BUFFER_SIZE];
                let zip = ImperatorZip::try_from_archive(archive, &mut buf, header.clone())?;
                Ok(ImperatorSliceFile {
                    header,
                    kind: ImperatorSliceFileKind::Zip(Box::new(zip)),
                })
            }
            _ if header.kind().is_binary() => Ok(ImperatorSliceFile {
                header: header.clone(),
                kind: ImperatorSliceFileKind::Binary(ImperatorBinary {
                    reader: data,
                    header,
                }),
            }),
            _ => Ok(ImperatorSliceFile {
                header,
                kind: ImperatorSliceFileKind::Text(ImperatorText(data)),
            }),
        }
    }

    pub fn from_file(mut file: File) -> Result<ImperatorFsFile<FileReader>, ImperatorError> {
        let mut buf = [0u8; SaveHeader::SIZE];
        file.read_exact(&mut buf)?;
        let header = SaveHeader::from_slice(&buf)?;
        let mut buf = vec![0u8; rawzip::RECOMMENDED_BUFFER_SIZE];

        let archive =
            rawzip::ZipArchive::with_max_search_space(64 * 1024).locate_in_file(file, &mut buf);

        match archive {
            Ok(archive) => {
                let zip = ImperatorZip::try_from_archive(archive, &mut buf, header.clone())?;
                Ok(ImperatorFsFile {
                    header,
                    kind: ImperatorFsFileKind::Zip(Box::new(zip)),
                })
            }
            Err(e) => {
                let mut file = e.into_inner();
                file.seek(std::io::SeekFrom::Start(SaveHeader::SIZE as u64))?;
                if header.kind().is_binary() {
                    Ok(ImperatorFsFile {
                        header: header.clone(),
                        kind: ImperatorFsFileKind::Binary(ImperatorBinary {
                            header,
                            reader: file,
                        }),
                    })
                } else {
                    Ok(ImperatorFsFile {
                        header,
                        kind: ImperatorFsFileKind::Text(file),
                    })
                }
            }
        }
    }
}

#[derive(Debug, Clone)]
pub enum ImperatorSliceFileKind<'a> {
    Text(ImperatorText<'a>),
    Binary(ImperatorBinary<&'a [u8]>),
    Zip(Box<ImperatorZip<&'a [u8]>>),
}

#[derive(Debug, Clone)]
pub struct ImperatorSliceFile<'a> {
    header: SaveHeader,
    kind: ImperatorSliceFileKind<'a>,
}

impl<'a> ImperatorSliceFile<'a> {
    pub fn kind(&self) -> &ImperatorSliceFileKind {
        &self.kind
    }

    pub fn kind_mut(&'a mut self) -> &'a mut ImperatorSliceFileKind<'a> {
        &mut self.kind
    }

    pub fn encoding(&self) -> Encoding {
        match &self.kind {
            ImperatorSliceFileKind::Text(_) => Encoding::Text,
            ImperatorSliceFileKind::Binary(_) => Encoding::Binary,
            ImperatorSliceFileKind::Zip(_) if self.header.kind().is_text() => Encoding::TextZip,
            ImperatorSliceFileKind::Zip(_) => Encoding::BinaryZip,
        }
    }

    pub fn parse_save<R>(&self, resolver: R) -> Result<Save, ImperatorError>
    where
        R: TokenResolver,
    {
        match &self.kind {
            ImperatorSliceFileKind::Text(data) => data.deserializer().deserialize(),
            ImperatorSliceFileKind::Binary(data) => {
                data.clone().deserializer(resolver).deserialize()
            }
            ImperatorSliceFileKind::Zip(archive) => {
                let game: GameState = archive.deserialize_gamestate(&resolver)?;
                let mut entry = archive.meta()?;
                let meta = entry.deserializer(&resolver).deserialize()?;
                Ok(Save {
                    meta,
                    gamestate: game,
                })
            }
        }
    }

    pub fn melt<Resolver, Writer>(
        &self,
        options: MeltOptions,
        resolver: Resolver,
        mut output: Writer,
    ) -> Result<MeltedDocument, ImperatorError>
    where
        Resolver: TokenResolver,
        Writer: Write,
    {
        match &self.kind {
            ImperatorSliceFileKind::Text(data) => {
                let mut new_header = self.header.clone();
                new_header.set_kind(crate::SaveHeaderKind::Text);
                new_header.write(&mut output)?;
                output.write_all(data.get_ref())?;
                Ok(MeltedDocument::new())
            }
            ImperatorSliceFileKind::Binary(data) => data.clone().melt(options, resolver, output),
            ImperatorSliceFileKind::Zip(zip) => zip.melt(options, resolver, output),
        }
    }
}

pub enum ImperatorFsFileKind<R> {
    Text(File),
    Binary(ImperatorBinary<File>),
    Zip(Box<ImperatorZip<R>>),
}

pub struct ImperatorFsFile<R> {
    header: SaveHeader,
    kind: ImperatorFsFileKind<R>,
}

impl<R> ImperatorFsFile<R> {
    pub fn kind(&self) -> &ImperatorFsFileKind<R> {
        &self.kind
    }

    pub fn kind_mut(&mut self) -> &mut ImperatorFsFileKind<R> {
        &mut self.kind
    }

    pub fn encoding(&self) -> Encoding {
        match &self.kind {
            ImperatorFsFileKind::Text(_) => Encoding::Text,
            ImperatorFsFileKind::Binary(_) => Encoding::Binary,
            ImperatorFsFileKind::Zip(_) if self.header.kind().is_text() => Encoding::TextZip,
            ImperatorFsFileKind::Zip(_) => Encoding::BinaryZip,
        }
    }
}

impl<R> ImperatorFsFile<R>
where
    R: ReaderAt,
{
    pub fn parse_save<RES>(&mut self, resolver: RES) -> Result<Save, ImperatorError>
    where
        RES: TokenResolver,
    {
        match &mut self.kind {
            ImperatorFsFileKind::Text(file) => {
                let reader = jomini::text::TokenReader::new(file);
                let mut deserializer = TextDeserializer::from_utf8_reader(reader);
                Ok(deserializer.deserialize()?)
            }
            ImperatorFsFileKind::Binary(file) => {
                let result = file.deserializer(resolver).deserialize()?;
                Ok(result)
            }
            ImperatorFsFileKind::Zip(archive) => {
                let game: GameState = archive.deserialize_gamestate(&resolver)?;
                let mut entry = archive.meta()?;
                let meta = entry.deserializer(&resolver).deserialize()?;
                Ok(Save {
                    meta,
                    gamestate: game,
                })
            }
        }
    }

    pub fn melt<Resolver, Writer>(
        &mut self,
        options: MeltOptions,
        resolver: Resolver,
        mut output: Writer,
    ) -> Result<MeltedDocument, ImperatorError>
    where
        Resolver: TokenResolver,
        Writer: Write,
    {
        match &mut self.kind {
            ImperatorFsFileKind::Text(file) => {
                let mut new_header = self.header.clone();
                new_header.set_kind(crate::SaveHeaderKind::Text);
                new_header.write(&mut output)?;
                std::io::copy(file, &mut output)?;
                Ok(MeltedDocument::new())
            }
            ImperatorFsFileKind::Binary(data) => data.melt(options, resolver, output),
            ImperatorFsFileKind::Zip(zip) => zip.melt(options, resolver, output),
        }
    }
}

#[derive(Debug, Clone)]
pub struct ImperatorZip<R> {
    pub(crate) archive: rawzip::ZipArchive<R>,
    pub(crate) metadata: ImperatorMetaKind,
    pub(crate) gamestate: ZipArchiveEntryWayfinder,
    pub(crate) header: SaveHeader,
}

impl<R> ImperatorZip<R>
where
    R: ReaderAt,
{
    pub fn try_from_archive(
        archive: rawzip::ZipArchive<R>,
        buf: &mut [u8],
        header: SaveHeader,
    ) -> Result<Self, ImperatorError> {
        let offset = archive.base_offset();
        let mut entries = archive.entries(buf);
        let mut gamestate = None;
        let mut metadata = None;

        while let Some(entry) = entries.next_entry().map_err(ImperatorErrorKind::Zip)? {
            match entry.file_raw_path() {
                b"gamestate" => gamestate = Some(entry.wayfinder()),
                b"meta" => metadata = Some(entry.wayfinder()),
                _ => {}
            };
        }

        match (gamestate, metadata) {
            (Some(gamestate), Some(metadata)) => Ok(ImperatorZip {
                archive,
                gamestate,
                metadata: ImperatorMetaKind::Zip(metadata),
                header,
            }),
            (Some(gamestate), None) => Ok(ImperatorZip {
                archive,
                gamestate,
                metadata: ImperatorMetaKind::Inlined(SaveHeader::SIZE..offset as usize),
                header,
            }),
            _ => Err(ImperatorErrorKind::ZipMissingEntry.into()),
        }
    }

    pub fn deserialize_gamestate<T, RES>(&self, resolver: RES) -> Result<T, ImperatorError>
    where
        T: DeserializeOwned,
        RES: TokenResolver,
    {
        let zip_entry = self
            .archive
            .get_entry(self.gamestate)
            .map_err(ImperatorErrorKind::Zip)?;
        let reader = CompressedFileReader::from_compressed(zip_entry.reader())?;
        let reader = zip_entry.verifying_reader(reader);
        let encoding = if self.header.kind().is_binary() {
            Encoding::Binary
        } else {
            Encoding::Text
        };
        let data: T = ImperatorModeller::from_reader(reader, &resolver, encoding).deserialize()?;
        Ok(data)
    }

    pub fn meta(&self) -> Result<ImperatorEntry<'_, rawzip::ZipReader<'_, R>, R>, ImperatorError> {
        let kind = match &self.metadata {
            ImperatorMetaKind::Inlined(x) => {
                let mut entry = vec![0u8; x.len()];
                self.archive
                    .get_ref()
                    .read_exact_at(&mut entry, x.start as u64)?;
                ImperatorEntryKind::Inlined(Cursor::new(entry))
            }
            ImperatorMetaKind::Zip(wayfinder) => {
                let zip_entry = self
                    .archive
                    .get_entry(*wayfinder)
                    .map_err(ImperatorErrorKind::Zip)?;
                let reader = CompressedFileReader::from_compressed(zip_entry.reader())?;
                let reader = zip_entry.verifying_reader(reader);
                ImperatorEntryKind::Zip(reader)
            }
        };

        Ok(ImperatorEntry {
            inner: kind,
            header: self.header.clone(),
        })
    }

    pub fn melt<Resolver, Writer>(
        &self,
        options: MeltOptions,
        resolver: Resolver,
        mut output: Writer,
    ) -> Result<MeltedDocument, ImperatorError>
    where
        Resolver: TokenResolver,
        Writer: Write,
    {
        let zip_entry = self
            .archive
            .get_entry(self.gamestate)
            .map_err(ImperatorErrorKind::Zip)?;
        let reader = CompressedFileReader::from_compressed(zip_entry.reader())?;
        let mut reader = zip_entry.verifying_reader(reader);

        if self.header.kind().is_text() {
            let mut new_header = self.header.clone();
            new_header.set_kind(crate::SaveHeaderKind::Text);
            new_header.write(&mut output)?;
            std::io::copy(&mut reader, &mut output)?;
            Ok(MeltedDocument::new())
        } else {
            melt::melt(
                &mut reader,
                &mut output,
                resolver,
                options,
                self.header.clone(),
            )
        }
    }
}

/// Describes the format of the metadata section of the save
#[derive(Debug, Clone)]
pub enum ImperatorMetaKind {
    Inlined(Range<usize>),
    Zip(ZipArchiveEntryWayfinder),
}

#[derive(Debug)]
pub struct ImperatorEntry<'archive, R, ReadAt> {
    inner: ImperatorEntryKind<'archive, R, ReadAt>,
    header: SaveHeader,
}

#[derive(Debug)]
pub enum ImperatorEntryKind<'archive, R, ReadAt> {
    Inlined(Cursor<Vec<u8>>),
    Zip(ZipVerifier<'archive, CompressedFileReader<R>, ReadAt>),
}

impl<R, ReadAt> Read for ImperatorEntry<'_, R, ReadAt>
where
    R: Read,
    ReadAt: ReaderAt,
{
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        match &mut self.inner {
            ImperatorEntryKind::Inlined(data) => data.read(buf),
            ImperatorEntryKind::Zip(reader) => reader.read(buf),
        }
    }
}

impl<'archive, R, ReadAt> ImperatorEntry<'archive, R, ReadAt>
where
    R: Read,
    ReadAt: ReaderAt,
{
    pub fn deserializer<'a, RES>(
        &'a mut self,
        resolver: RES,
    ) -> ImperatorModeller<&'a mut ImperatorEntry<'archive, R, ReadAt>, RES>
    where
        RES: TokenResolver,
    {
        let encoding = if self.header.kind().is_text() {
            Encoding::Text
        } else {
            Encoding::Binary
        };
        ImperatorModeller::from_reader(self, resolver, encoding)
    }

    pub fn melt<Resolver, Writer>(
        &mut self,
        options: MeltOptions,
        resolver: Resolver,
        mut output: Writer,
    ) -> Result<MeltedDocument, ImperatorError>
    where
        Resolver: TokenResolver,
        Writer: Write,
    {
        if self.header.kind().is_text() {
            let mut new_header = self.header.clone();
            new_header.set_kind(crate::SaveHeaderKind::Text);
            new_header.write(&mut output)?;
            std::io::copy(self, &mut output)?;
            Ok(MeltedDocument::new())
        } else {
            let header = self.header.clone();
            melt::melt(self, &mut output, resolver, options, header)
        }
    }
}

/// A parsed Imperator text document
pub struct ImperatorParsedText<'a> {
    tape: TextTape<'a>,
}

impl<'a> ImperatorParsedText<'a> {
    pub fn from_slice(data: &'a [u8]) -> Result<Self, ImperatorError> {
        let header = SaveHeader::from_slice(data)?;
        Self::from_raw(&data[header.header_len()..])
    }

    pub fn from_raw(data: &'a [u8]) -> Result<Self, ImperatorError> {
        let tape = TextTape::from_slice(data).map_err(ImperatorErrorKind::Parse)?;
        Ok(ImperatorParsedText { tape })
    }

    pub fn reader(&self) -> ObjectReader<Utf8Encoding> {
        self.tape.utf8_reader()
    }
}

#[derive(Debug, Clone)]
pub struct ImperatorText<'a>(&'a [u8]);

impl ImperatorText<'_> {
    pub fn get_ref(&self) -> &[u8] {
        self.0
    }

    pub fn deserializer(&self) -> ImperatorModeller<&[u8], HashMap<u16, String>> {
        ImperatorModeller {
            reader: self.0,
            resolver: HashMap::new(),
            encoding: Encoding::Text,
        }
    }
}

#[derive(Debug, Clone)]
pub struct ImperatorBinary<R> {
    reader: R,
    header: SaveHeader,
}

impl<R> ImperatorBinary<R>
where
    R: Read,
{
    pub fn get_ref(&self) -> &R {
        &self.reader
    }

    pub fn deserializer<RES>(&mut self, resolver: RES) -> ImperatorModeller<&'_ mut R, RES> {
        ImperatorModeller {
            reader: &mut self.reader,
            resolver,
            encoding: Encoding::Binary,
        }
    }

    pub fn melt<Resolver, Writer>(
        &mut self,
        options: MeltOptions,
        resolver: Resolver,
        mut output: Writer,
    ) -> Result<MeltedDocument, ImperatorError>
    where
        Resolver: TokenResolver,
        Writer: Write,
    {
        melt::melt(
            &mut self.reader,
            &mut output,
            resolver,
            options,
            self.header.clone(),
        )
    }
}

#[derive(Debug)]
pub struct ImperatorModeller<Reader, Resolver> {
    reader: Reader,
    resolver: Resolver,
    encoding: Encoding,
}

impl<Reader: Read, Resolver: TokenResolver> ImperatorModeller<Reader, Resolver> {
    pub fn from_reader(reader: Reader, resolver: Resolver, encoding: Encoding) -> Self {
        ImperatorModeller {
            reader,
            resolver,
            encoding,
        }
    }

    pub fn encoding(&self) -> Encoding {
        self.encoding
    }

    pub fn deserialize<T>(&mut self) -> Result<T, ImperatorError>
    where
        T: DeserializeOwned,
    {
        T::deserialize(self)
    }

    pub fn into_inner(self) -> Reader {
        self.reader
    }
}

impl<'de, 'a: 'de, Reader: Read, Resolver: TokenResolver> serde::de::Deserializer<'de>
    for &'a mut ImperatorModeller<Reader, Resolver>
{
    type Error = ImperatorError;

    fn deserialize_any<V>(self, _visitor: V) -> Result<V::Value, Self::Error>
    where
        V: serde::de::Visitor<'de>,
    {
        Err(ImperatorError::new(ImperatorErrorKind::DeserializeImpl {
            msg: String::from("only struct supported"),
        }))
    }

    fn deserialize_struct<V>(
        self,
        name: &'static str,
        fields: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value, Self::Error>
    where
        V: serde::de::Visitor<'de>,
    {
        if matches!(self.encoding, Encoding::Binary) {
            use jomini::binary::BinaryFlavor;
            let mut deser = ImperatorFlavor
                .deserializer()
                .from_reader(&mut self.reader, &self.resolver);
            Ok(deser.deserialize_struct(name, fields, visitor)?)
        } else {
            let reader = jomini::text::TokenReader::new(&mut self.reader);
            let mut deser = TextDeserializer::from_utf8_reader(reader);
            Ok(deser.deserialize_struct(name, fields, visitor)?)
        }
    }

    serde::forward_to_deserialize_any! {
        bool i8 i16 i32 i64 i128 u8 u16 u32 u64 u128 f32 f64 char str string
        bytes byte_buf option unit unit_struct newtype_struct seq tuple
        tuple_struct map enum identifier ignored_any
    }
}

#[derive(Debug)]
pub struct CompressedFileReader<R> {
    reader: flate2::read::DeflateDecoder<R>,
}

impl<R: Read> CompressedFileReader<R> {
    pub fn from_compressed(reader: R) -> Result<Self, ImperatorError>
    where
        R: Read,
    {
        let inflater = flate2::read::DeflateDecoder::new(reader);
        Ok(CompressedFileReader { reader: inflater })
    }
}

impl<R> std::io::Read for CompressedFileReader<R>
where
    R: Read,
{
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        self.reader.read(buf)
    }
}
