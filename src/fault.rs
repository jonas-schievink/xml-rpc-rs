use Value;

use std::str;
use std::fmt::{self, Formatter, Display};
use std::error::Error;
use std::collections::BTreeMap;

/// A `<fault>` response, indicating that a request failed.
///
/// The XML-RPC specification requires that a `<faultCode>` and `<faultString>` is returned in the
/// `<fault>` case, further describing the error.
#[derive(Debug, PartialEq, Eq)]
pub struct Fault {
    /// `fault_code` received from the server.
    code: i32,
    /// Undecoded `fault_string` sent by the server.
    raw_string: Vec<u8>,
    /// UTF-8-decoded `fault_string`. Required for `Error::description`.
    // We could lazily compute this and use a `RefCell`, but it's probably not important.
    dec_string: String,
}

impl Fault {
    /// Creates a new `Fault` from an error code and a UTF-8 message.
    pub fn new(code: i32, string: String) -> Fault {
        Fault {
            code,
            dec_string: string.clone(),
            raw_string: string.into_bytes(),
        }
    }

    /// Creates a `Fault` from an error code and a raw message.
    pub fn from_raw_string(code: i32, string: Vec<u8>) -> Fault {
        Fault {
            code,
            dec_string: String::from_utf8_lossy(&string).into_owned(),
            raw_string: string,
        }
    }

    /// Returns the fault code.
    ///
    /// The meaning of this code is not specified by XML-RPC and depends on the service you are
    /// implementing/using.
    pub fn code(&self) -> i32 {
        self.code
    }

    /// Returns the error message as a `String`, if it is valid UTF-8.
    ///
    /// The `fault_string` field in a `<fault>` can contain any byte and might not be valid UTF-8.
    /// In that case, this function returns `None`. If you need to deal with non-UTF-8 data, use
    /// `string_as_bytes`.
    pub fn string(&self) -> Option<String> {
        str::from_utf8(self.string_as_bytes()).ok().map(String::from)
    }

    /// Returns the error message as a `String`, replacing invalid byte sequences with the Unicode
    /// replacement character.
    pub fn string_lossy(&self) -> String {
        self.dec_string.clone()
    }

    /// Returns the `fault_string` field as raw bytes.
    pub fn string_as_bytes(&self) -> &[u8] {
        &self.raw_string
    }

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

                match (map.get(&b"faultCode"[..]), map.get(&b"faultString"[..])) {
                    (Some(&Value::Int(fault_code)), Some(&Value::String(ref fault_string))) => {
                        Some(Fault::from_raw_string(fault_code, fault_string.to_vec()))
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
        map.insert(b"faultCode".to_vec(), Value::from(self.code()));
        map.insert(b"faultString".to_vec(), Value::String(self.string_as_bytes().to_vec()));

        Value::Struct(map)
    }
}

impl Display for Fault {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write!(f, "{} ({})", self.string_lossy(), self.code())
    }
}

impl Error for Fault {
    fn description(&self) -> &str {
        &self.dec_string
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fault_roundtrip() {
        let input = Fault::new(-123456, "The Bald Lazy House Jumps Over The Hyperactive Kitten".to_string());

        assert_eq!(Fault::from_value(&input.to_value()), Some(input));
    }
}
