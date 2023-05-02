use std::error::Error;
use std::fmt::{self, Display};

use crate::parse::{Parse, ParseBuf, Parser};

used_in_docs!(Parse, Parser, ParseBuf);

type BoxedError = Box<dyn Error + Send + Sync + 'static>;

/// A specialized result type used by [`Parse`] and [`Parser`].
pub type Result<T> = std::result::Result<T, ParseError>;

/// The error type for parsing errors as returned by [`Parser`].
///
/// The format used by perf events doesn't give many opportunities for error
/// checking so most parsing errors will likely result in an error with [`kind`]
/// [`ErrorKind::Eof`]. Otherwise, this type can be used to wrap errors emitted
/// by the [`ParseBuf`] type.
///
/// [`kind`]: ParseError::kind
#[derive(Debug)]
pub struct ParseError {
    code: ErrorKind,
    source: Option<BoxedError>,
}

impl ParseError {
    /// Create a new `ParseError` from an arbitrary error payload.
    pub fn new<E>(error: E) -> Self
    where
        E: Into<BoxedError>,
    {
        Self {
            code: ErrorKind::External,
            source: Some(error.into()),
        }
    }

    /// Create a new `ParseError` with a custom message.
    pub fn custom(msg: impl Display) -> Self {
        Self::new(CustomMessageError(msg.to_string()))
    }

    /// Get the [`ErrorKind`] of this error.
    pub fn kind(&self) -> ErrorKind {
        self.code
    }

    const fn from_code(code: ErrorKind) -> Self {
        Self { code, source: None }
    }

    pub(crate) fn with_code(self, code: ErrorKind) -> Self {
        Self { code, ..self }
    }

    /// More input was needed before the item could be successfully parsed.
    pub fn eof() -> Self {
        Self::from_code(ErrorKind::Eof)
    }
}

/// A list specifying general categories of parse error.
#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash)]
#[non_exhaustive]
pub enum ErrorKind {
    /// There was no more data in the [`ParseBuf`] but more is required in
    /// in order to parse the record.
    ///
    /// Should be returned by [`ParseBuf::chunk`] when there is no data left to
    /// be returned.
    Eof,

    /// A record was parsed, but it was invalid.
    ///
    /// This is for validation errors that occur when parsing the record. Most
    /// errors will result either leftover unparsed data or
    /// [`Eof`](ErrorKind::Eof) errors.
    InvalidRecord,

    /// An external error, forwarded from the [`ParseBuf`] implementation.
    ///
    /// This error will never be emitted by a parse method in this crate.
    External,
}

impl Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.code {
            ErrorKind::Eof => f.write_str("unexpected EOF during parsing")?,
            ErrorKind::InvalidRecord => f.write_str("invalid record")?,
            ErrorKind::External => {
                // This type should always have a source, but, however, if it doesn't then we
                // still need to provide a default message.
                if self.source.is_none() {
                    f.write_str("user-provided error")?;
                }
            }
        }

        if let Some(source) = &self.source {
            if matches!(self.code, ErrorKind::External) {
                f.write_str(": ")?;
            }

            source.fmt(f)?;
        }

        Ok(())
    }
}

impl Error for ParseError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match &self.source {
            Some(source) => Some(&**source),
            None => None,
        }
    }
}

impl From<std::io::Error> for ParseError {
    fn from(error: std::io::Error) -> Self {
        match error.kind() {
            std::io::ErrorKind::UnexpectedEof => Self::new(error).with_code(ErrorKind::Eof),
            _ => Self::new(error),
        }
    }
}

impl From<BoxedError> for ParseError {
    fn from(error: BoxedError) -> Self {
        Self {
            code: ErrorKind::External,
            source: Some(error),
        }
    }
}

#[derive(Debug)]
struct CustomMessageError(String);

impl fmt::Display for CustomMessageError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl Error for CustomMessageError {}
