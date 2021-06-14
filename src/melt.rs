use crate::{
    detect_encoding, flavor::ImperatorFlavor, tokens::TokenLookup, BodyEncoding, Extraction,
    FailedResolveStrategy, ImperatorDate, ImperatorError, ImperatorErrorKind, PdsDate,
};
use jomini::{BinaryTape, BinaryToken, TextWriterBuilder, TokenResolver};
use std::{
    collections::HashSet,
    io::{Cursor, Read},
};

/// Convert a binary gamestate to plaintext
///
/// Accepted inputs:
///
/// - a save file
/// - already extracted binary data
#[derive(Debug)]
pub struct Melter {
    on_failed_resolve: FailedResolveStrategy,
    extraction: Extraction,
    rewrite: bool,
}

impl Default for Melter {
    fn default() -> Self {
        Melter {
            extraction: Extraction::InMemory,
            on_failed_resolve: FailedResolveStrategy::Ignore,
            rewrite: true,
        }
    }
}

impl Melter {
    /// Create a customized version to melt binary data
    pub fn new() -> Self {
        Melter::default()
    }

    /// Set the memory allocation extraction behavior for when a zip is encountered
    pub fn with_extraction(mut self, extraction: Extraction) -> Self {
        self.extraction = extraction;
        self
    }

    /// Set the behavior for when an unresolved binary token is encountered
    pub fn with_on_failed_resolve(mut self, strategy: FailedResolveStrategy) -> Self {
        self.on_failed_resolve = strategy;
        self
    }

    /// Set if the melter should rewrite properties to better match the plaintext format
    ///
    /// Setting to false will preserve binary fields and values even if they
    /// don't make any sense in the plaintext output.
    pub fn with_rewrite(mut self, rewrite: bool) -> Self {
        self.rewrite = rewrite;
        self
    }

    fn convert(
        &self,
        input: &[u8],
        writer: &mut Vec<u8>,
        unknown_tokens: &mut HashSet<u16>,
    ) -> Result<(), ImperatorError> {
        let tape = BinaryTape::parser_flavor(ImperatorFlavor).parse_slice(input)?;
        let mut wtr = TextWriterBuilder::new()
            .indent_char(b'\t')
            .indent_factor(1)
            .from_writer(writer);
        let mut token_idx = 0;
        let mut known_number = false;
        let tokens = tape.tokens();

        while let Some(token) = tokens.get(token_idx) {
            match token {
                BinaryToken::Object(_) => {
                    wtr.write_object_start()?;
                }
                BinaryToken::HiddenObject(_) => {
                    wtr.write_hidden_object_start()?;
                }
                BinaryToken::Array(_) => {
                    wtr.write_array_start()?;
                }
                BinaryToken::End(_x) => {
                    wtr.write_end()?;
                }
                BinaryToken::Bool(x) => wtr.write_bool(*x)?,
                BinaryToken::U32(x) => wtr.write_u32(*x)?,
                BinaryToken::U64(x) => wtr.write_u64(*x)?,
                BinaryToken::I32(x) => {
                    if known_number {
                        wtr.write_i32(*x)?;
                        known_number = false;
                    } else if let Some(date) = ImperatorDate::from_binary_heuristic(*x) {
                        wtr.write_date(date.game_fmt())?;
                    } else {
                        wtr.write_i32(*x)?;
                    }
                }
                BinaryToken::Quoted(x) => {
                    wtr.write_quoted(x.view_data())?;
                }
                BinaryToken::Unquoted(x) => {
                    wtr.write_unquoted(x.view_data())?;
                }
                BinaryToken::F32(x) => wtr.write_f32(*x)?,
                BinaryToken::F64(x) => wtr.write_f64(*x)?,
                BinaryToken::Token(x) => match TokenLookup.resolve(*x) {
                    Some(id) if (self.rewrite && id == "is_ironman") && wtr.expecting_key() => {
                        let skip = tokens
                            .get(token_idx + 1)
                            .map(|next_token| match next_token {
                                BinaryToken::Object(end) => end + 1,
                                BinaryToken::Array(end) => end + 1,
                                _ => token_idx + 2,
                            })
                            .unwrap_or(token_idx + 1);

                        token_idx = skip;
                        continue;
                    }
                    Some(id) => {
                        known_number = id == "seed";
                        wtr.write_unquoted(id.as_bytes())?;
                    }
                    None => {
                        unknown_tokens.insert(*x);
                        match self.on_failed_resolve {
                            FailedResolveStrategy::Error => {
                                return Err(
                                    ImperatorErrorKind::UnknownToken { token_id: *x }.into()
                                );
                            }
                            FailedResolveStrategy::Ignore if wtr.expecting_key() => {
                                let skip = tokens
                                    .get(token_idx + 1)
                                    .map(|next_token| match next_token {
                                        BinaryToken::Object(end) => end + 1,
                                        BinaryToken::Array(end) => end + 1,
                                        _ => token_idx + 2,
                                    })
                                    .unwrap_or(token_idx + 1);

                                token_idx = skip;
                                continue;
                            }
                            _ => {
                                write!(wtr, "__unknown_0x{:x}", x)?;
                            }
                        }
                    }
                },
                BinaryToken::Rgb(color) => {
                    wtr.write_header(b"rgb")?;
                    wtr.write_array_start()?;
                    wtr.write_u32(color.r)?;
                    wtr.write_u32(color.g)?;
                    wtr.write_u32(color.b)?;
                    wtr.write_end()?;
                }
            }

            token_idx += 1;
        }

        Ok(())
    }

    /// Given one of the accepted inputs, this will return the save id line (if present in the input)
    /// with the gamestate data decoded from binary to plain text.
    pub fn melt(&self, data: &[u8]) -> Result<(Vec<u8>, HashSet<u16>), ImperatorError> {
        let mut result = Vec::with_capacity(data.len());
        let mut unknown_tokens = HashSet::new();

        // if there is a save id line in the data, we should preserve it
        let has_save_id = data.get(0..3).map_or(false, |x| x == b"SAV");
        let data = if has_save_id {
            let split_ind = data.iter().position(|&x| x == b'\n').unwrap_or(0);
            let at = std::cmp::max(split_ind, 0);
            let (header, rest) = data.split_at(at + 1);
            result.extend_from_slice(header);
            rest
        } else {
            data
        };

        let mut reader = Cursor::new(data);
        match detect_encoding(&mut reader)? {
            BodyEncoding::Plain => self.convert(data, &mut result, &mut unknown_tokens)?,
            BodyEncoding::Zip(mut zip) => {
                let size = zip
                    .by_name("gamestate")
                    .map_err(|e| ImperatorErrorKind::ZipMissingEntry("gamestate", e))
                    .map(|x| x.size())?;
                result.reserve(size as usize);

                let mut zip_file = zip
                    .by_name("gamestate")
                    .map_err(|e| ImperatorErrorKind::ZipMissingEntry("gamestate", e))?;

                match self.extraction {
                    Extraction::InMemory => {
                        let mut inflated_data: Vec<u8> = Vec::with_capacity(size as usize);
                        zip_file
                            .read_to_end(&mut inflated_data)
                            .map_err(|e| ImperatorErrorKind::ZipExtraction("gamestate", e))?;
                        self.convert(&inflated_data, &mut result, &mut unknown_tokens)?
                    }

                    #[cfg(feature = "mmap")]
                    Extraction::MmapTemporaries => {
                        let mut mmap = memmap::MmapMut::map_anon(zip_file.size() as usize)?;
                        std::io::copy(&mut zip_file, &mut mmap.as_mut())
                            .map_err(|e| ImperatorErrorKind::ZipExtraction("gamestate", e))?;
                        self.convert(&mmap[..], &mut result, &unknown_tokens)?
                    }
                }
            }
        }

        Ok((result, unknown_tokens))
    }
}
