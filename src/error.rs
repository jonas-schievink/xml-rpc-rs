//! Defines error types used by this library.

use Value;

use reqwest::Error as ReqwestError;
use xml::reader::Error as XmlError;
use xml::common::TextPosition;

use std::io;
use std::fmt::{self, Formatter, Display};
use std::error::Error;
use std::collections::BTreeMap;

/// A request could not be executed.
///
/// This is either a lower-level error (for example, the HTTP request failed), or a problem with the
/// server (maybe it's not implementing XML-RPC correctly). If the server sends a valid response,
/// this error will not occur.
#[derive(Debug)]
pub enum RequestError {
    /// An HTTP communication error occurred while sending the request or receiving the response.
    HttpError(ReqwestError),

    /// The HTTP status code did not indicate success.
    HttpStatus(String),

    /// The response could not be parsed. This can happen when the server doesn't correctly
    /// implement the XML-RPC spec.
    ParseError(ParseError),

    // TODO make this extensible. anything missing?
}

impl From<ReqwestError> for RequestError {
    fn from(e: ReqwestError) -> Self {
        RequestError::HttpError(e)
    }
}

impl From<ParseError> for RequestError {
    fn from(e: ParseError) -> Self {
        RequestError::ParseError(e)
    }
}

impl Display for RequestError {
    fn fmt(&self, fmt: &mut Formatter) -> fmt::Result {
        match *self {
            RequestError::HttpError(ref err) => write!(fmt, "HTTP error: {}", err),
            RequestError::HttpStatus(ref err) => write!(fmt, "HTTP status: {}", err),
            RequestError::ParseError(ref err) => write!(fmt, "parse error: {}", err),
        }
    }
}

impl Error for RequestError {
    fn description(&self) -> &str {
        match *self {
            RequestError::HttpError(ref err) => err.description(),
            RequestError::HttpStatus(ref err) => &err,
            RequestError::ParseError(ref err) => err.description(),
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

impl Error for ParseError {
    fn description(&self) -> &str {
        match *self {
            ParseError::XmlError(ref err) => err.description(),
            ParseError::InvalidValue { .. } => "invalid value for type",
            ParseError::UnexpectedXml { .. } => "unexpected XML content",
        }
    }
}

/// A `<fault>` response - The call failed.
///
/// The XML-RPC specification requires that a `<faultCode>` and `<faultString>` is returned in the
/// `<fault>` case, further describing the error.
#[derive(Debug, PartialEq, Eq)]
pub struct Fault {
    pub fault_code: i32,
    pub fault_string: String,
}

impl Fault {
    /// Creates a `Fault` from a `Value`.
    ///
    /// The `Value` must be a `Value::Struct` with a `faultCode` and `faultString` field.
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fault_roundtrip() {
        let input = Fault {
            fault_code: -123456,
            fault_string: "The Bald Lazy House Jumps Over The Hyperactive Kitten".to_string()
        };

        assert_eq!(Fault::from_value(&input.to_value()), Some(input));
    }
}
