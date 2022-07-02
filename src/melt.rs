use crate::{
    flavor::ImperatorFlavor, ImperatorDate, ImperatorError, ImperatorErrorKind, SaveHeader,
    SaveHeaderKind,
};
use jomini::{
    binary::{BinaryFlavor, FailedResolveStrategy, TokenResolver},
    common::PdsDate,
    BinaryTape, BinaryToken, TextWriterBuilder,
};
use std::collections::HashSet;

#[derive(thiserror::Error, Debug)]
pub(crate) enum MelterError {
    #[error("{0}")]
    Write(#[from] jomini::Error),

    #[error("")]
    UnknownToken { token_id: u16 },
}

/// Output from melting a binary save to plaintext
pub struct MeltedDocument {
    data: Vec<u8>,
    unknown_tokens: HashSet<u16>,
}

impl MeltedDocument {
    /// The converted plaintext data
    pub fn data(&self) -> &[u8] {
        self.data.as_slice()
    }

    /// The list of unknown tokens that the provided resolver accumulated
    pub fn unknown_tokens(&self) -> &HashSet<u16> {
        &self.unknown_tokens
    }
}

/// Convert a binary save to plaintext
pub struct ImperatorMelter<'a, 'b> {
    tape: &'b BinaryTape<'a>,
    header: &'b SaveHeader,
    verbatim: bool,
    on_failed_resolve: FailedResolveStrategy,
}

impl<'a, 'b> ImperatorMelter<'a, 'b> {
    pub(crate) fn new(tape: &'b BinaryTape<'a>, header: &'b SaveHeader) -> Self {
        ImperatorMelter {
            tape,
            header,
            verbatim: false,
            on_failed_resolve: FailedResolveStrategy::Ignore,
        }
    }

    pub fn verbatim(&mut self, verbatim: bool) -> &mut Self {
        self.verbatim = verbatim;
        self
    }

    pub fn on_failed_resolve(&mut self, strategy: FailedResolveStrategy) -> &mut Self {
        self.on_failed_resolve = strategy;
        self
    }

    pub(crate) fn skip_value_idx(&self, token_idx: usize) -> usize {
        self.tape
            .tokens()
            .get(token_idx + 1)
            .map(|next_token| match next_token {
                BinaryToken::Object(end) | BinaryToken::Array(end) => end + 1,
                _ => token_idx + 2,
            })
            .unwrap_or(token_idx + 1)
    }

    pub fn melt<R>(&self, resolver: &R) -> Result<MeltedDocument, ImperatorError>
    where
        R: TokenResolver,
    {
        let out = melt(self, resolver).map_err(|e| match e {
            MelterError::Write(x) => ImperatorErrorKind::Writer(x),
            MelterError::UnknownToken { token_id } => ImperatorErrorKind::UnknownToken { token_id },
        })?;
        Ok(out)
    }
}

fn update_header(data: &mut Vec<u8>, mut header: SaveHeader) {
    header.set_kind(SaveHeaderKind::Text);
    header.set_metadata_len((data.len() + 1 - header.header_len()) as u64);
    let _ = header.write(&mut data[..header.header_len()]);
}

pub(crate) fn melt<R>(melter: &ImperatorMelter, resolver: &R) -> Result<MeltedDocument, MelterError>
where
    R: TokenResolver,
{
    let flavor = ImperatorFlavor;
    let mut out = Vec::with_capacity(melter.tape.tokens().len() * 10);
    let _ = melter.header.write(&mut out);

    let mut unknown_tokens = HashSet::new();
    let mut has_written_new_header = false;

    let mut wtr = TextWriterBuilder::new()
        .indent_char(b'\t')
        .indent_factor(1)
        .from_writer(out);
    let mut token_idx = 0;
    let mut known_number = false;
    let tokens = melter.tape.tokens();

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
                if wtr.expecting_key() {
                    wtr.write_unquoted(x.as_bytes())?;
                } else {
                    wtr.write_quoted(x.as_bytes())?;
                }
            }
            BinaryToken::Unquoted(x) => {
                wtr.write_unquoted(x.as_bytes())?;
            }
            BinaryToken::F32(x) => wtr.write_f32(flavor.visit_f32(*x))?,
            BinaryToken::F64(x) => wtr.write_f64(flavor.visit_f64(*x))?,
            BinaryToken::Token(x) => match resolver.resolve(*x) {
                Some(id) => {
                    if !melter.verbatim && id == "is_ironman" && wtr.expecting_key() {
                        token_idx = melter.skip_value_idx(token_idx);
                        continue;
                    }

                    if id == "speed" {
                        update_header(wtr.inner(), melter.header.clone());
                        has_written_new_header = true;
                    }

                    known_number = id == "seed";
                    wtr.write_unquoted(id.as_bytes())?;
                }
                None => match melter.on_failed_resolve {
                    FailedResolveStrategy::Error => {
                        return Err(MelterError::UnknownToken { token_id: *x });
                    }
                    FailedResolveStrategy::Ignore if wtr.expecting_key() => {
                        token_idx = melter.skip_value_idx(token_idx);
                        continue;
                    }
                    _ => {
                        unknown_tokens.insert(*x);
                        write!(wtr, "__unknown_0x{:x}", x)?;
                    }
                },
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

    let mut inner = wtr.into_inner();

    if !has_written_new_header {
        update_header(&mut inner, melter.header.clone());
    }

    inner.push(b'\n');

    Ok(MeltedDocument {
        data: inner,
        unknown_tokens,
    })
}
