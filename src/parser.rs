//! XML-RPC response parser.

use {Value, Fault};
use error::ParseError;

use base64;
use quick_xml::reader::Reader;
use quick_xml::events::Event;
use iso8601::datetime;
use std::io::{self, ErrorKind};
use std::io::prelude::*;
use std::collections::BTreeMap;
use std::str;

// TODO manual CDATA to text
// TODO remove clone

/// A response from the server.
///
/// XML-RPC specifies that a call should either return a single `Value`, or a `<fault>`.
pub type Response = Result<Value, Fault>;

type ParseResult<T> = Result<T, ParseError>;

pub struct Parser<'a, R: BufRead> {
    reader: Reader<R>,
    buf: &'a mut Vec<u8>,
    /// Current "token". The parser makes decisions based on this token, then pulls the next one
    /// from `reader`.
    cur: Event<'a>,
}

impl<'a, R: BufRead> Parser<'a, R> {
    pub fn new(reader: R, buf: &'a mut Vec<u8>) -> ParseResult<Self> {
        let mut reader = Reader::from_reader(reader);
        reader.expand_empty_elements(true);
        let mut parser = Parser {
            cur: Event::Eof, // dummy value
            buf,
            reader,
        };
        parser.next()?;
        Ok(parser)
    }

    /// Disposes `self.cur` and pulls the next event from the XML parser to replace it.
    fn next(&mut self) -> ParseResult<()> {
        loop {
            let event = self.reader.read_event(self.buf)?;
            match event {
                Event::Decl(_)
                | Event::Comment(_)
                | Event::DocType(_)
                | Event::PI(_) => continue,   // skip these
                Event::Start(ref start) => {
                    if start.attributes().next().is_some() {
                        return self.expected(format!("tag <{}> without attributes", String::from_utf8_lossy(start.name())));
                    }
                },
                Event::End(_)
                | Event::Eof
                | Event::CData(_)
                | Event::Text(_) => {}
            }

            self.cur = event;
            return Ok(());
        }
    }

    /// Expects that the current token is an opening tag like `<tag>` without attributes (and a
    /// local name without namespaces). If not, returns an error.
    fn expect_open(&mut self, tag: &str) -> ParseResult<()> {
        match self.cur.clone() {
            Event::Start(ref start) if start.name() == tag.as_bytes() => {
                self.next()?;
                Ok(())
            }
            _ => return self.expected(format!("<{}>", tag)),
        }
    }

    /// Expects that the current token is a closing tag like `</tag>` with a local name without
    /// namespaces. If not, returns an error.
    fn expect_close<T: AsRef<[u8]>>(&mut self, tag: T) -> ParseResult<()> {
        match self.cur.clone() {
            Event::End(ref end) if end.name() == tag.as_ref() => {
                self.next()?;
                Ok(())
            }
            _ => self.expected(format!("</{}>", String::from_utf8_lossy(tag.as_ref()))),
        }
    }

    /// Builds and returns an `Err(UnexpectedXml)`.
    fn expected<T, E: ToString>(&self, expected: E) -> ParseResult<T> {
        let expected = expected.to_string();
        let position = self.reader.buffer_position();

        Err(ParseError::UnexpectedXml {
            expected,
            position,
            found: match self.cur {
                Event::Start(ref start) => Some(format!("<{}>", String::from_utf8_lossy(start.name()))),
                Event::End(ref end) => Some(format!("</{}>", String::from_utf8_lossy(end.name()))),
                Event::Eof => Some("end of data".to_string()),
                Event::Text(ref data)
                | Event::CData(ref data) => Some(format!("\"{}\"", String::from_utf8_lossy(data))),
                _ => None
            },
        })
    }

    fn invalid_value(&self, for_type: &'static str, value: &str) -> ParseError {
        // FIXME: It might be neat to preserve the original error as the cause
        ParseError::InvalidValue {
            for_type,
            found: value.to_string(),
            position: self.reader.buffer_position(),
        }
    }

    fn parse_response(&mut self) -> ParseResult<Response> {
        let response: Response;

        // <methodResponse>
        self.expect_open("methodResponse")?;

        // <fault> / <params>
        match self.cur.clone() {
            Event::Start(ref start) => {
                match start.name() {
                    b"fault" => {
                        self.next()?;
                        let value = self.parse_value()?;
                        let fault = Fault::from_value(&value).ok_or_else(|| {
                            io::Error::new(ErrorKind::Other, "malformed <fault>")
                        })?;
                        response = Err(fault);
                    }
                    b"params" => {
                        self.next()?;
                        // <param>
                        self.expect_open("param")?;

                        let value = self.parse_value()?;
                        response = Ok(value);

                        // </param>
                        self.expect_close("param")?;
                    }
                    _ => {
                        return self.expected(format!("<fault> or <params>, got {}", String::from_utf8_lossy(start.name())));
                    }
                }
            }
            _ => return self.expected("<fault> or <params>"),
        }

        Ok(response)
    }

    fn parse_value(&mut self, cur: &Event<'a>) -> ParseResult<Value> {
        // <value>
        self.expect_open("value")?;

        if let Ok(()) = self.expect_close("value") {
            // empty value, parse as empty string
            return Ok(Value::String(Vec::new()));
        }

        let value = self.parse_value_inner(cur)?;

        // </value>
        self.expect_close("value")?;

        Ok(value)
    }

    fn parse_value_inner(&mut self, cur: &Event<'a>) -> ParseResult<Value> {
        let value = match self.cur.clone() {
            // Raw string or specific type tag

            Event::Start(ref start) => match start.name() {
                b"struct" => {
                    self.next()?;
                    let mut members = BTreeMap::new();
                    loop {
                        if let Ok(_) = self.expect_close("struct") {
                            break;
                        }

                        self.expect_open("member")?;
                        // <member>

                        // <name>NAME</name>
                        self.expect_open("name")?;
                        let name = if let Event::Text(ref text) = self.cur.clone() {
                            text.unescaped()?.into_owned()
                        } else {
                            return self.expected("characters");
                        };
                        self.next()?;
                        self.expect_close("name")?;

                        // Value
                        let value = self.parse_value()?;

                        // </member>
                        self.expect_close("member")?;

                        members.insert(name, value);
                    }

                    Value::Struct(members)
                }
                b"array" => {
                    self.next()?;
                    let mut elements: Vec<Value> = Vec::new();
                    self.expect_open("data")?;
                    loop {
                        if let Ok(_) = self.expect_close("data") {
                            break;
                        }

                        elements.push(self.parse_value()?);
                    }
                    self.expect_close("array")?;
                    Value::Array(elements)
                }
                b"nil" => {
                    self.next()?;
                    self.expect_close("nil")?;
                    Value::Nil
                }
                b"string" => {
                    self.next()?;
                    let bytes = match self.cur {
                        Event::Text(string) => {
                            self.next()?;
                            self.expect_close("string")?;
                            string.unescaped()?.into_owned()
                        },
                        Event::End(ref end) if end.name() == b"string" => {
                            self.next()?;
                            Vec::new()
                        },
                        _ => return self.expected("characters or </string>"),
                    };
                    Value::String(bytes)
                }
                b"base64" => {
                    self.next()?;
                    let data = match self.cur.clone() {
                        Event::Text(ref string) => {
                            self.next()?;
                            self.expect_close("base64")?;
                            // FIXME escaped should suffice here
                            let content = string.unescaped()?;
                            base64::decode(content.as_ref()).map_err(|_| {
                                self.invalid_value("base64", &String::from_utf8_lossy(content.as_ref()).into_owned())
                            })?
                        },
                        Event::End(ref end) if end.name() == b"base64" => {
                            self.next()?;
                            Vec::new()
                        },
                        _ => return self.expected("characters or </base64>"),
                    };
                    Value::Base64(data)
                }
                _ => {
                    self.next()?;
                    // All other types expect text (utf-8 only for now)
                    let data = match self.cur.clone() {
                        Event::Text(string) => str::from_utf8(&string.unescaped()?)?,
                        _ => return self.expected("characters"),
                    };

                    let value = match start.name() {
                        b"i4" | b"int" => {
                            Value::Int(data.parse::<i32>().map_err(|_| {
                                self.invalid_value("integer", data)
                            })?)
                        }
                        b"i8" => {
                            Value::Int64(data.parse::<i64>().map_err(|_| {
                                self.invalid_value("i8", data)
                            })?)
                        }
                        b"boolean" => {
                            let val = match &*data {
                                "0" => false,
                                "1" => true,
                                _ => return Err(self.invalid_value("boolean", data)),
                            };
                            Value::Bool(val)
                        }
                        b"double" => {
                            Value::Double(data.parse::<f64>().map_err(|_| {
                                self.invalid_value("double", data)
                            })?)
                        }
                        b"dateTime.iso8601" => {
                            Value::DateTime(datetime(&data).map_err(|_| {
                                self.invalid_value("dateTime.iso8601", data)
                            })?)
                        }
                        _ => return self.expected("valid type tag or characters"),
                    };

                    self.next()?;
                    // ...and a corresponding close tag
                    self.expect_close(start.name())?;

                    value
                }
            }
            Event::Text(text) => {
                self.next()?;
                Value::String(text.unescaped()?.into_owned())
            }
            _ => return self.expected("type tag or characters"),
        };

        Ok(value)
    }
}

/// Parses a response from an XML reader.
pub fn parse_response<R: BufRead>(reader: &mut R) -> ParseResult<Response> {
    let mut buf = Vec::new();
    Parser::new(reader, &mut buf)?.parse_response()
}

#[cfg(test)]
mod tests {
    use super::*;

    use {Value, Fault};
    use std::fmt::Debug;

    fn read_response(xml: &str) -> ParseResult<Response> {
        parse_response(&mut xml.as_bytes())
    }

    fn read_value(xml: &str) -> ParseResult<Value> {
        let mut buf = Vec::new();
        Parser::new(&mut xml.as_bytes(), &mut buf)?.parse_value()
    }

    /// Test helper function that will panic with the `Err` if a `Result` is not an `Ok`.
    fn assert_ok<T: Debug, E: Debug>(result: Result<T, E>) {
        match result {
            Ok(_) => {},
            Err(e) => panic!("assert_ok called on Err value: {:?}", e),
        }
    }

    /// Test helper function that will panic with the `Ok` if a `Result` is not an `Err`.
    fn assert_err<T: Debug, E: Debug>(result: Result<T, E>) {
        match result {
            Ok(t) => panic!("assert_err called on Ok value: {:?}", t),
            Err(_) => {},
        }
    }

    #[test]
    fn parses_base64_response() {
        assert_ok(read_response(r##"
<?xml version="1.0" encoding="UTF-8"?>
<methodResponse>
    <params>
        <param>
            <value><base64>0J/QvtC10YXQsNC70Lgh</base64></value>
        </param>
    </params>
</methodResponse>
"##));
    }

    #[test]
    fn parses_response() {
        assert_ok(read_response(r##"
<?xml version="1.0"?>
<methodResponse>
    <params>
        <param>
            <value>teststring</value>
        </param>
    </params>
</methodResponse>
"##));
    }

    #[test]
    fn parses_fault() {
        assert_eq!(read_response(r##"
<?xml version="1.0"?>
<methodResponse>
   <fault>
      <value>
         <struct>
            <member>
               <name>faultCode</name>
               <value><int>4</int></value>
               </member>
            <member>
               <name>faultString</name>
               <value><string>Too many parameters.</string></value>
               </member>
            </struct>
         </value>
      </fault>
   </methodResponse>"##),
        Ok(Err(Fault::new(4, "Too many parameters.".into()))));
    }

    #[test]
    fn rejects_additional_fault_fields() {
        // "A <fault> struct may not contain members other than those specified."

        assert_err(read_response(r##"
<?xml version="1.0"?>
<methodResponse>
   <fault>
      <value>
         <struct>
            <member>
               <name>faultCode</name>
               <value><int>4</int></value>
               </member>
            <member>
               <name>faultString</name>
               <value><string>Too many parameters.</string></value>
               </member>
            <member>
               <name>unnecessaryParameter</name>
               <value><string>Too many parameters.</string></value>
               </member>
            </struct>
         </value>
      </fault>
   </methodResponse>"##));
    }

    #[test]
    fn rejects_invalid_faults() {
        // Make sure to reject type errors in <fault>s - They're specified to contain specifically
        // typed fields.
        assert_err(read_response(r##"
<?xml version="1.0"?>
<methodResponse>
   <fault>
      <value>
         <struct>
            <member>
               <name>faultCode</name>
               <value><string>I'm not an int!</string></value>
               </member>
            <member>
               <name>faultString</name>
               <value><string>Too many parameters.</string></value>
               </member>
            </struct>
         </value>
      </fault>
   </methodResponse>"##));

        assert_err(read_response(r##"
<?xml version="1.0"?>
<methodResponse>
   <fault>
      <value>
         <struct>
            <member>
               <name>faultCode</name>
               <value><int>4</int></value>
               </member>
            <member>
               <name>faultString</name>
               <value><base64>I'm not a string!</base64></value>
               </member>
            </struct>
         </value>
      </fault>
   </methodResponse>"##));
    }

    #[test]
    fn parses_string_value_with_whitespace() {
        assert_eq!(read_value("<value><string>  I'm a string!  </string></value>"),
            Ok(Value::String("  I'm a string!  ".into())));
    }

    #[test]
    fn parses_64bit_int() {
        assert_eq!(read_value("<value><i8>12345</i8></value>"),
            Ok(Value::Int64(12345)));
        assert_eq!(read_value("<value><i8>-100100100100</i8></value>"),
            Ok(Value::Int64(-100100100100)));
    }

    #[test]
    fn parses_int_with_plus_sign() {
        // "You can include a plus or minus at the beginning of a string of numeric characters."
        assert_eq!(read_value("<value><int>+1234</int></value>"),
            Ok(Value::Int(1234)));
    }

    #[test]
    fn parses_date_values() {
        assert_ok(read_value("<value><dateTime.iso8601>2015-02-18T23:16:09Z</dateTime.iso8601></value>"));
        assert_ok(read_value("<value><dateTime.iso8601>19980717T14:08:55</dateTime.iso8601></value>"));
        assert_err(read_value("<value><dateTime.iso8601></dateTime.iso8601></value>"));
        assert_err(read_value("<value><dateTime.iso8601>ILLEGAL VALUE :(</dateTime.iso8601></value>"));
    }

    #[test]
    fn parses_base64() {
        assert_eq!(read_value("<value><base64>0J/QvtC10YXQsNC70Lgh</base64></value>"),
            Ok(Value::Base64("Поехали!".bytes().collect())));
    }

    #[test]
    fn parses_array_values() {
        assert_eq!(read_value(r#"
                <value><array><data>
                    <value><i4>5</i4></value>
                    <value><string>a</string></value>
                </data></array></value>"#),
            Ok(Value::Array(vec![Value::Int(5), Value::String("a".into())])));
    }

    #[test]
    fn parses_raw_value_as_string() {
        assert_eq!(read_value("<value>\t  I'm a string!  </value>"),
            Ok(Value::String("\t  I'm a string!  ".into())));
    }

    #[test]
    fn parses_nil_values() {
        assert_eq!(read_value("<value><nil/></value>"), Ok(Value::Nil));
        assert_eq!(read_value("<value><nil></nil></value>"), Ok(Value::Nil));
        assert_err(read_value("<value><nil>ILLEGAL</nil></value>"));
    }

    #[test]
    fn unescapes_values() {
        assert_eq!(read_value("<value><string>abc&lt;abc&amp;abc</string></value>"),
            Ok(Value::String("abc<abc&abc".into())));
    }

    #[test]
    fn parses_empty_string() {
        assert_eq!(read_value("<value><string></string></value>"),
            Ok(Value::String(Vec::new())));
        assert_eq!(read_value("<value><string/></value>"),
            Ok(Value::String(Vec::new())));
    }

    #[test]
    fn parses_empty_value_as_string() {
        assert_eq!(read_value("<value></value>"),
                   Ok(Value::String(Vec::new())));
    }

    #[test]
    fn parses_empty_base64() {
        assert_eq!(read_value("<value><base64></base64></value>"),
            Ok(Value::Base64(Vec::new())));
        assert_eq!(read_value("<value><base64/></value>"),
            Ok(Value::Base64(Vec::new())));
    }

    #[test]
    fn rejects_attributes() {
        assert_err(read_value(r#"<value name="ble">\t  I'm a string!  </value>"#));

        assert_err(read_response(r##"
<?xml version="1.0"?>
<methodResponse invalid="1">
    <params>
        <param>
            <value>teststring</value>
        </param>
    </params>
</methodResponse>
"##));
        assert_err(read_response(r##"
<?xml version="1.0"?>
<methodResponse>
    <params invalid="1">
        <param>
            <value>teststring</value>
        </param>
    </params>
</methodResponse>
"##));
        assert_err(read_response(r##"
<?xml version="1.0"?>
<methodResponse>
    <params>
        <param invalid="1">
            <value>teststring</value>
        </param>
    </params>
</methodResponse>
"##));
        assert_err(read_response(r##"
<?xml version="1.0"?>
<methodResponse>
    <params>
        <param>
            <value invalid="1">teststring</value>
        </param>
    </params>
</methodResponse>
"##));
        assert_err(read_response(r##"
<?xml version="1.0"?>
<methodResponse>
    <params>
        <param>
            <value><int invalid="1">4</int></value>
        </param>
    </params>
</methodResponse>
"##));
    }

    #[test]
    fn error_messages() {
        fn errstr(value: &str) -> String {
            read_value(value).unwrap_err().to_string()
        }

        assert_eq!(
            errstr(r#"<value name="ble">\t  I'm a string!  </value>"#),
            "unexpected XML at 1:1 (expected tag <value> without attributes, found end of data)"
        );

        // FIXME: This one could use some improvement:
        assert_eq!(
            errstr(r#"<value><SURPRISE></SURPRISE></value>"#),
            "unexpected XML at 1:18 (expected characters, found </SURPRISE>)"
        );

        assert_eq!(
            errstr(r#"<value><int>bla</int></value>"#),
            "invalid value for type \'integer\' at 1:13: bla"
        );
    }

    #[test]
    fn parses_empty_value_response() {
        assert_ok(read_response(r##"
<?xml version="1.0"?>
<methodResponse>
    <params>
        <param>
            <value></value>
        </param>
    </params>
</methodResponse>
"##));
    }

    #[test]
    fn parses_empty_value_in_struct_response() {
        assert_ok(read_response(r##"
<?xml version="1.0"?>
<methodResponse>
    <params>
        <param><value>
        <struct><member>
            <name>Test</name>
            <value></value>
        </member></struct>
        </value></param>
    </params>
</methodResponse>
"##));
    }
}
