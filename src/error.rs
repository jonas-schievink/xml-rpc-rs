//! Defines error types used by this library.

use Fault;

use quick_xml::errors::Error as XmlError;

use std::io;
use std::fmt::{self, Formatter, Display};
use std::error::Error;
use std::str::Utf8Error;

/// A request could not be executed.
///
/// This can be a lower-level error (for example, the HTTP request failed), a problem with the
/// server (maybe it's not implementing XML-RPC correctly), or just a failure to execute the
/// operation.
#[derive(Debug)]
pub struct RequestError(RequestErrorKind);

impl RequestError {
    /// If this `RequestError` was caused by the server responding with a `<fault>` response,
    /// returns the `Fault` in question.
    pub fn fault(&self) -> Option<&Fault> {
        match self.0 {
            RequestErrorKind::Fault(ref fault) => Some(fault),
            _ => None,
        }
    }
}

impl From<RequestErrorKind> for RequestError {
    fn from(kind: RequestErrorKind) -> Self {
        RequestError(kind)
    }
}

impl Display for RequestError {
    fn fmt(&self, fmt: &mut Formatter) -> fmt::Result {
        self.0.fmt(fmt)
    }
}

impl Error for RequestError {
    fn description(&self) -> &str {
        self.0.description()
    }

    fn cause(&self) -> Option<&Error> {
        self.0.cause()
    }
}

#[derive(Debug)]
pub enum RequestErrorKind {
    /// The response could not be parsed. This can happen when the server doesn't correctly
    /// implement the XML-RPC spec.
    ParseError(ParseError),

    /// A communication error originating from the transport used to perform the request.
    TransportError(Box<Error + 'static>),

    /// The server returned a `<fault>` response, indicating that the execution of the call
    /// encountered a problem (for example, an invalid (number of) arguments was passed).
    Fault(Fault),
}

impl From<ParseError> for RequestErrorKind {
    fn from(e: ParseError) -> Self {
        RequestErrorKind::ParseError(e)
    }
}

impl From<Fault> for RequestErrorKind {
    fn from(f: Fault) -> Self {
        RequestErrorKind::Fault(f)
    }
}

impl Display for RequestErrorKind {
    fn fmt(&self, fmt: &mut Formatter) -> fmt::Result {
        match *self {
            RequestErrorKind::ParseError(ref err) => write!(fmt, "parse error: {}", err),
            RequestErrorKind::TransportError(ref err) => write!(fmt, "transport error: {}", err),
            RequestErrorKind::Fault(ref err) => write!(fmt, "{}", err),
        }
    }
}

impl Error for RequestErrorKind {
    fn description(&self) -> &str {
        match *self {
            RequestErrorKind::ParseError(_) => "parse error",
            RequestErrorKind::TransportError(_) => "transport error",
            RequestErrorKind::Fault(_) => "server returned a fault",
        }
    }

    fn cause(&self) -> Option<&Error> {
        match *self {
            RequestErrorKind::ParseError(ref err) => Some(err),
            RequestErrorKind::TransportError(ref err) => Some(err.as_ref()),
            RequestErrorKind::Fault(ref err) => Some(err),
        }
    }
}

/// Describes possible error that can occur when parsing a `Response`.
#[derive(Debug)]
pub enum ParseError {
    /// Error while parsing (malformed?) XML.
    XmlError(XmlError),

    /// Could not parse the given CDATA as XML-RPC value.
    ///
    /// For example, `<value><int>AAA</int></value>` describes an invalid value.
    InvalidValue {
        /// The type for which an invalid value was supplied (eg. `int` or `dateTime.iso8601`).
        for_type: &'static str,
        /// The value we encountered, as a string.
        found: String,
        /// The byte position of the invalid value inside the XML document.
        position: usize,
    },

    /// Found an unexpected tag, attribute, etc.
    UnexpectedXml {
        /// A short description of the kind of data that was expected.
        expected: String,
        found: Option<String>,
        /// The byte position of the unexpected data inside the XML document.
        position: usize,
    },

    /// Got invalid UTF-8 for a part of the request / response where UTF-8 is required.
    ///
    /// This is limited to the parsing of primitive values. Strings and names should support
    /// arbitrary bytes.
    ///
    // FIXME: Use some encoding lib and remove this
    Utf8Error(Utf8Error),
}

impl From<XmlError> for ParseError {
    fn from(e: XmlError) -> Self {
        ParseError::XmlError(e)
    }
}

impl From<io::Error> for ParseError {
    fn from(e: io::Error) -> Self {
        ParseError::XmlError(XmlError::Io(e))
    }
}

impl From<Utf8Error> for ParseError {
    fn from(e: Utf8Error) -> Self {
        ParseError::Utf8Error(e)
    }
}

impl Display for ParseError {
    fn fmt(&self, fmt: &mut Formatter) -> fmt::Result {
        match *self {
            ParseError::XmlError(ref err) => write!(fmt, "malformed XML: {}", err),
            ParseError::InvalidValue {
                for_type,
                ref found,
                ref position,
            } => write!(fmt, "invalid value for type '{}' at offset {}: {}", for_type, position, found),
            ParseError::UnexpectedXml {
                ref expected,
                ref position,
                found: None,
            } => {
                write!(fmt, "unexpected XML at offset {} (expected {})", position, expected)
            }
            ParseError::UnexpectedXml {
                ref expected,
                ref position,
                found: Some(ref found),
            } => {
                write!(fmt, "unexpected XML at offset {} (expected {}, found {})", position, expected, found)
            }
        }
    }
}

impl Error for ParseError {
    fn description(&self) -> &str {
        match *self {
            ParseError::XmlError(..) => "XML error",
            ParseError::InvalidValue { .. } => "invalid value for type",
            ParseError::UnexpectedXml { .. } => "unexpected XML content",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use std::error::Error;

    #[test]
    fn error_impls_error() {
        fn assert_error<T: Error>() {}

        assert_error::<RequestError>();
    }
}
