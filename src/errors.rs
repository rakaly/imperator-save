use jomini::binary;
use std::{fmt, io};

/// A Imperator Error
#[derive(thiserror::Error, Debug)]
#[error(transparent)]
pub struct ImperatorError(#[from] Box<ImperatorErrorKind>);

impl ImperatorError {
    pub(crate) fn new(kind: ImperatorErrorKind) -> ImperatorError {
        ImperatorError(Box::new(kind))
    }

    /// Return the specific type of error
    pub fn kind(&self) -> &ImperatorErrorKind {
        &self.0
    }
}

impl From<ImperatorErrorKind> for ImperatorError {
    fn from(err: ImperatorErrorKind) -> Self {
        ImperatorError::new(err)
    }
}

/// Specific type of error
#[derive(thiserror::Error, Debug)]
pub enum ImperatorErrorKind {
    #[error("unable to parse as zip: {0}")]
    Zip(#[from] rawzip::Error),

    #[error("missing gamestate entry in zip")]
    ZipMissingEntry,

    #[error("unable to deserialize due to: {msg}. This shouldn't occur as this is a deserializer wrapper")]
    DeserializeImpl { msg: String },

    #[error("unable to parse due to: {0}")]
    Parse(#[source] jomini::Error),

    #[error("unable to deserialize due to: {0}")]
    Deserialize(#[source] jomini::Error),

    #[error("error while writing output: {0}")]
    Writer(#[source] jomini::Error),

    #[error("unknown binary token encountered: {token_id:#x}")]
    UnknownToken { token_id: u16 },

    #[error("invalid header")]
    InvalidHeader,

    #[error("io error: {0}")]
    Io(#[from] io::Error),
}

impl serde::de::Error for ImperatorError {
    fn custom<T: fmt::Display>(msg: T) -> Self {
        ImperatorError::new(ImperatorErrorKind::DeserializeImpl {
            msg: msg.to_string(),
        })
    }
}

impl From<jomini::Error> for ImperatorError {
    fn from(value: jomini::Error) -> Self {
        let kind = match value.into_kind() {
            jomini::ErrorKind::Deserialize(x) => match x.kind() {
                &jomini::DeserializeErrorKind::UnknownToken { token_id } => {
                    ImperatorErrorKind::UnknownToken { token_id }
                }
                _ => ImperatorErrorKind::Deserialize(x.into()),
            },
            _ => ImperatorErrorKind::DeserializeImpl {
                msg: String::from("unexpected error"),
            },
        };

        ImperatorError::new(kind)
    }
}

impl From<io::Error> for ImperatorError {
    fn from(value: io::Error) -> Self {
        ImperatorError::from(ImperatorErrorKind::from(value))
    }
}

impl From<binary::ReaderError> for ImperatorError {
    fn from(value: binary::ReaderError) -> Self {
        Self::from(jomini::Error::from(value))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn size_of_error_test() {
        assert_eq!(std::mem::size_of::<ImperatorError>(), 8);
    }
}
