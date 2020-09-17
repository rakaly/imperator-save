use std::fmt;
use std::io::Error as IoError;
use zip::result::ZipError;

/// An Imperator Error
#[derive(Debug)]
pub struct ImperatorError(Box<ImperatorErrorKind>);

impl ImperatorError {
    pub(crate) fn new(kind: ImperatorErrorKind) -> ImperatorError {
        ImperatorError(Box::new(kind))
    }

    /// Return the specific type of error
    pub fn kind(&self) -> &ImperatorErrorKind {
        &self.0
    }
}

/// Specific type of error
#[derive(Debug)]
pub enum ImperatorErrorKind {
    ZipCentralDirectory(ZipError),
    ZipMissingEntry(&'static str, ZipError),
    ZipExtraction(&'static str, IoError),
    ZipSize(&'static str),
    IoErr(IoError),
    UnknownHeader,
    UnknownToken {
        token_id: u16,
    },
    Deserialize {
        part: Option<String>,
        err: jomini::Error,
    },
}

impl fmt::Display for ImperatorError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.kind() {
            ImperatorErrorKind::ZipCentralDirectory(_) => {
                write!(f, "unable to read zip central directory")
            }
            ImperatorErrorKind::ZipMissingEntry(s, _) => write!(f, "unable to locate {} in zip", s),
            ImperatorErrorKind::ZipExtraction(s, _) => write!(f, "unable to extract {} in zip", s),
            ImperatorErrorKind::ZipSize(s) => write!(f, "{} in zip is too large", s),
            ImperatorErrorKind::IoErr(_) => write!(f, "io error"),
            ImperatorErrorKind::UnknownHeader => write!(f, "unknown header encountered in zip"),
            ImperatorErrorKind::UnknownToken { token_id } => {
                write!(f, "unknown binary token encountered (id: {})", token_id)
            }
            ImperatorErrorKind::Deserialize { ref part, ref err } => match part {
                Some(p) => write!(f, "error deserializing: {}: {}", p, err),
                None => err.fmt(f),
            },
        }
    }
}

impl std::error::Error for ImperatorError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self.kind() {
            ImperatorErrorKind::ZipCentralDirectory(e) => Some(e),
            ImperatorErrorKind::ZipMissingEntry(_, e) => Some(e),
            ImperatorErrorKind::ZipExtraction(_, e) => Some(e),
            ImperatorErrorKind::IoErr(e) => Some(e),
            ImperatorErrorKind::Deserialize { ref err, .. } => Some(err),
            _ => None,
        }
    }
}

impl From<jomini::Error> for ImperatorError {
    fn from(err: jomini::Error) -> Self {
        ImperatorError::new(ImperatorErrorKind::Deserialize { part: None, err })
    }
}

impl From<IoError> for ImperatorError {
    fn from(err: IoError) -> Self {
        ImperatorError::new(ImperatorErrorKind::IoErr(err))
    }
}

impl From<ImperatorErrorKind> for ImperatorError {
    fn from(err: ImperatorErrorKind) -> Self {
        ImperatorError::new(err)
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
