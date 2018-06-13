//! Defines error types used by this library.

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
pub struct Error(RequestErrorKind);

impl Error {
    /// If this `Error` was caused by the server responding with a `<fault>` response,
    /// returns the `Fault` in question.
    pub fn fault(&self) -> Option<&Fault> {
        match self.0 {
            RequestErrorKind::Fault(ref fault) => Some(fault),
            _ => None,
        }
    }
}

#[doc(hidden)]  // hide internal impl
impl From<RequestErrorKind> for Error {
    fn from(kind: RequestErrorKind) -> Self {
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
pub enum RequestErrorKind {
    /// The response could not be parsed. This can happen when the server doesn't correctly
    /// implement the XML-RPC spec.
    ParseError(ParseError),

    /// A communication error originating from the transport used to perform the request.
    TransportError(Box<error::Error + Send + Sync>),

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

impl error::Error for RequestErrorKind {
    fn description(&self) -> &str {
        match *self {
            RequestErrorKind::ParseError(_) => "parse error",
            RequestErrorKind::TransportError(_) => "transport error",
            RequestErrorKind::Fault(_) => "server returned a fault",
        }
    }

    fn cause(&self) -> Option<&error::Error> {
        match *self {
            RequestErrorKind::ParseError(ref err) => Some(err),
            RequestErrorKind::TransportError(ref err) => Some(err.as_ref()),
            RequestErrorKind::Fault(ref err) => Some(err),
        }
    }
}

/// Describes possible error that can occur when parsing a `Response`.
#[derive(Debug, PartialEq)]
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

impl From<XmlError> for ParseError {
    fn from(e: XmlError) -> Self {
        ParseError::XmlError(e)
    }
}

impl From<io::Error> for ParseError {
    fn from(e: io::Error) -> Self {
        ParseError::XmlError(XmlError::from(e))
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
            } => write!(fmt, "invalid value for type '{}' at {}: {}", for_type, position, found),
            ParseError::UnexpectedXml {
                ref expected,
                ref position,
                found: None,
            } => {
                write!(fmt, "unexpected XML at {} (expected {})", position, expected)
            }
            ParseError::UnexpectedXml {
                ref expected,
                ref position,
                found: Some(ref found),
            } => {
                write!(fmt, "unexpected XML at {} (expected {}, found {})", position, expected, found)
            }
        }
    }
}

impl error::Error for ParseError {
    fn description(&self) -> &str {
        match *self {
            ParseError::XmlError(ref err) => err.description(),
            ParseError::InvalidValue { .. } => "invalid value for type",
            ParseError::UnexpectedXml { .. } => "unexpected XML content",
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
