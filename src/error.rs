//! Defines error types used by this library.
//!
//! The main error type returned by most functions is [`Error`].
//!
//! [`Error`]: struct.Error.html

use Value;

use xml::reader::Error as XmlError;
use xml::common::TextPosition;

use std::{error, io};
use std::fmt::{self, Formatter, Display};
use std::collections::BTreeMap;

/// Errors that can occur when trying to perform an XML-RPC request.
///
/// This can be a lower-level error (for example, the HTTP request failed), a problem with the
/// server (maybe it's not implementing XML-RPC correctly), or just a failure to execute the
/// operation.
#[derive(Debug)]
pub struct Error(ErrorKind);

impl Error {
    /// If this `Error` was caused by the server responding with a `<fault>` response,
    /// returns the [`Fault`] in question.
    ///
    /// [`Fault`]: struct.Fault.html
    pub fn fault(&self) -> Option<&Fault> {
        match self.0 {
            ErrorKind::Fault(ref fault) => Some(fault),
            _ => None,
        }
    }

    /// If this `Error` was caused by a failure to parse a request or response, returns the
    /// corresponding [`ParseError`].
    ///
    /// [`ParseError`]: struct.ParseError.html
    pub fn parse_error(&self) -> Option<&ParseError> {
        match self.0 {
            ErrorKind::ParseError(ref e) => Some(e),
            _ => None,
        }
    }
}

#[doc(hidden)]  // hide internal impl (it's not usable from outside anyways)
impl From<ErrorKind> for Error {
    fn from(kind: ErrorKind) -> Self {
        Error(kind)
    }
}

impl Display for Error {
    fn fmt(&self, fmt: &mut Formatter) -> fmt::Result {
        self.0.fmt(fmt)
    }
}

impl error::Error for Error {
    fn description(&self) -> &str {
        self.0.description()
    }

    fn cause(&self) -> Option<&error::Error> {
        self.0.cause()
    }
}

#[derive(Debug)]
pub(crate) enum ErrorKind {
    /// The response could not be parsed. This can happen when the server doesn't correctly
    /// implement the XML-RPC spec.
    ParseError(ParseError),

    /// A communication error originating from the transport used to perform the request.
    TransportError(Box<error::Error + Send + Sync>),

    /// The server returned a `<fault>` response, indicating that the execution of the call
    /// encountered a problem (for example, an invalid (number of) arguments was passed).
    Fault(Fault),

    /// Error message.
    #[allow(unused)]
    String(String),
}

impl From<ParseErrorKind> for ErrorKind {
    fn from(e: ParseErrorKind) -> Self {
        ErrorKind::ParseError(ParseError::new(e))
    }
}

impl From<Fault> for ErrorKind {
    fn from(f: Fault) -> Self {
        ErrorKind::Fault(f)
    }
}

impl Display for ErrorKind {
    fn fmt(&self, fmt: &mut Formatter) -> fmt::Result {
        match *self {
            ErrorKind::ParseError(ref err) => write!(fmt, "parse error: {}", err),
            ErrorKind::TransportError(ref err) => write!(fmt, "transport error: {}", err),
            ErrorKind::Fault(ref err) => write!(fmt, "{}", err),
            ErrorKind::String(ref err) => write!(fmt, "{}", err),
        }
    }
}

impl error::Error for ErrorKind {
    fn description(&self) -> &str {
        match *self {
            ErrorKind::ParseError(_) => "parse error",
            ErrorKind::TransportError(_) => "transport error",
            ErrorKind::Fault(_) => "server returned a fault",
            ErrorKind::String(ref s) => s,
        }
    }

    fn cause(&self) -> Option<&error::Error> {
        match *self {
            ErrorKind::ParseError(ref err) => Some(err),
            ErrorKind::TransportError(ref err) => Some(err.as_ref()),
            ErrorKind::Fault(ref err) => Some(err),
            ErrorKind::String(_) => None,
        }
    }
}

/// An error that occurred while parsing an XML-RPC request or response.
#[derive(Debug)]
pub struct ParseError {
    kind: ParseErrorKind,
}

impl ParseError {
    fn new(kind: ParseErrorKind) -> Self {
        Self { kind }
    }
}

impl Display for ParseError {
    fn fmt(&self, fmt: &mut Formatter) -> fmt::Result {
        self.kind.fmt(fmt)
    }
}

impl error::Error for ParseError {
    fn description(&self) -> &str {
        self.kind.description()
    }
}

/// Describes possible error that can occur when parsing a `Response`.
#[derive(Debug, PartialEq)]
pub(crate) enum ParseErrorKind {
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
        /// The position of the invalid value inside the XML document.
        position: TextPosition,
    },

    /// Found an unexpected tag, attribute, etc.
    UnexpectedXml {
        /// A short description of the kind of data that was expected.
        expected: String,
        found: Option<String>,
        /// The position of the unexpected data inside the XML document.
        position: TextPosition,
    }
}

impl From<XmlError> for ParseErrorKind {
    fn from(e: XmlError) -> Self {
        ParseErrorKind::XmlError(e)
    }
}

impl From<io::Error> for ParseErrorKind {
    fn from(e: io::Error) -> Self {
        ParseErrorKind::XmlError(XmlError::from(e))
    }
}

impl Display for ParseErrorKind {
    fn fmt(&self, fmt: &mut Formatter) -> fmt::Result {
        match *self {
            ParseErrorKind::XmlError(ref err) => write!(fmt, "malformed XML: {}", err),
            ParseErrorKind::InvalidValue {
                for_type,
                ref found,
                ref position,
            } => write!(fmt, "invalid value for type '{}' at {}: {}", for_type, position, found),
            ParseErrorKind::UnexpectedXml {
                ref expected,
                ref position,
                found: None,
            } => {
                write!(fmt, "unexpected XML at {} (expected {})", position, expected)
            }
            ParseErrorKind::UnexpectedXml {
                ref expected,
                ref position,
                found: Some(ref found),
            } => {
                write!(fmt, "unexpected XML at {} (expected {}, found {})", position, expected, found)
            }
        }
    }
}

impl error::Error for ParseErrorKind {
    fn description(&self) -> &str {
        match *self {
            ParseErrorKind::XmlError(ref err) => err.description(),
            ParseErrorKind::InvalidValue { .. } => "invalid value for type",
            ParseErrorKind::UnexpectedXml { .. } => "unexpected XML content",
        }
    }
}

/// A `<fault>` response, indicating that a request failed.
///
/// The XML-RPC specification requires that a `<faultCode>` and `<faultString>` is returned in the
/// `<fault>` case, further describing the error.
#[derive(Debug, PartialEq, Eq)]
pub struct Fault {
    /// An application-specific error code.
    pub fault_code: i32,
    /// Human-readable error description.
    pub fault_string: String,
}

// TODO (breaking): make Fault fields private and provide getters and ctor

impl Fault {
    /// Creates a `Fault` from a `Value`.
    ///
    /// The `Value` must be a `Value::Struct` with a `faultCode` and `faultString` field (and no
    /// other fields).
    ///
    /// Returns `None` if the value isn't a valid `Fault`.
    pub fn from_value(value: &Value) -> Option<Self> {
        match *value {
            Value::Struct(ref map) => {
                if map.len() != 2 {
                    // incorrect field count
                    return None;
                }

                match (map.get("faultCode"), map.get("faultString")) {
                    (Some(&Value::Int(fault_code)), Some(&Value::String(ref fault_string))) => {
                        Some(Fault {
                            fault_code,
                            fault_string: fault_string.to_string(),
                        })
                    }
                    _ => None
                }
            }
            _ => None
        }
    }

    /// Turns this `Fault` into an equivalent `Value`.
    ///
    /// The returned value can be parsed back into a `Fault` using `Fault::from_value` or returned
    /// as a `<fault>` error response by serializing it into a `<fault></fault>` tag.
    pub fn to_value(&self) -> Value {
        let mut map = BTreeMap::new();
        map.insert("faultCode".to_string(), Value::from(self.fault_code));
        map.insert("faultString".to_string(), Value::from(self.fault_string.as_ref()));

        Value::Struct(map)
    }
}

impl Display for Fault {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write!(f, "{} ({})", self.fault_string, self.fault_code)
    }
}

impl error::Error for Fault {
    fn description(&self) -> &str {
        &self.fault_string
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use std::error;

    #[test]
    fn fault_roundtrip() {
        let input = Fault {
            fault_code: -123456,
            fault_string: "The Bald Lazy House Jumps Over The Hyperactive Kitten".to_string()
        };

        assert_eq!(Fault::from_value(&input.to_value()), Some(input));
    }

    #[test]
    fn error_impls_error() {
        fn assert_error<T: error::Error>() {}

        assert_error::<Error>();
    }

    #[test]
    fn error_is_send_sync() {
        fn assert_send_sync<T: Send + Sync>() {}

        assert_send_sync::<Error>();
    }
}
