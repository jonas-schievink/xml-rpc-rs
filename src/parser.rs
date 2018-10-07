//! XML-RPC response parser.

use {Value, Fault};
use error::ParseError;

use base64;
use xml::reader::{XmlEvent, EventReader};
use xml::common::Position;
use xml::ParserConfig;
use iso8601::datetime;
use std::io::{self, ErrorKind, Read};
use std::collections::BTreeMap;

/// A response from the server.
///
/// XML-RPC specifies that a call should either return a single `Value`, or a `<fault>`.
pub type Response = Result<Value, Fault>;

type ParseResult<T> = Result<T, ParseError>;

pub struct Parser<'a, R: Read + 'a> {
    reader: EventReader<&'a mut R>,
    /// Current "token". The parser makes decisions based on this token, then pulls the next one
    /// from `reader`.
    cur: XmlEvent,
}

impl<'a, R: Read> Parser<'a, R> {
    pub fn new(reader: &'a mut R) -> ParseResult<Self> {
        let reader = EventReader::new_with_config(reader, ParserConfig {
            cdata_to_characters: true,
            ..Default::default()
        });

        let mut parser = Parser {
            cur: XmlEvent::EndDocument, // dummy value
            reader,
        };
        parser.next()?;
        Ok(parser)
    }

    /// Disposes `self.cur` and pulls the next event from the XML parser to replace it.
    fn next(&mut self) -> ParseResult<()> {
        loop {
            let event = self.reader.next()?;
            match event {
                XmlEvent::StartDocument { .. }
                | XmlEvent::Comment(_)
                | XmlEvent::Whitespace(_)
                | XmlEvent::ProcessingInstruction { .. } => continue,   // skip these
                XmlEvent::StartElement { ref attributes, ref name, .. } => {
                    if name.namespace.is_some() || name.prefix.is_some() {
                        return self.expected("tag without namespace or prefix");
                    }
                    if !attributes.is_empty() {
                        return self.expected(format!("tag <{}> without attributes", name));
                    }
                },
                XmlEvent::EndElement { ref name } => {
                    if name.namespace.is_some() || name.prefix.is_some() {
                        return self.expected("tag without namespace or prefix");
                    }
                },
                XmlEvent::EndDocument
                | XmlEvent::CData(_)
                | XmlEvent::Characters(_) => {}
            }

            self.cur = event;
            return Ok(());
        }
    }

    /// Expects that the current token is an opening tag like `<tag>` without attributes (and a
    /// local name without namespaces). If not, returns an error.
    fn expect_open(&mut self, tag: &str) -> ParseResult<()> {
        match self.cur {
            XmlEvent::StartElement { ref name, .. } if name.local_name == tag => {},
            _ => return self.expected(format!("<{}>", tag)),
        }
        self.next()?;
        Ok(())
    }

    /// Expects that the current token is a closing tag like `</tag>` with a local name without
    /// namespaces. If not, returns an error.
    fn expect_close(&mut self, tag: &str) -> ParseResult<()> {
        match self.cur {
            XmlEvent::EndElement { ref name } if name.local_name == tag => {},
            _ => return self.expected(format!("</{}>", tag)),
        }
        self.next()?;
        Ok(())
    }

    /// Expects that the current token is a characters sequence. Parses and returns a value.
    fn expect_value<T, E>(&mut self, for_type: &'static str, parse: impl Fn(&str) -> Result<T, E>) -> ParseResult<T> {
        let value = match self.cur {
            XmlEvent::Characters(ref string) => {
                parse(string).map_err(|_| {
                    self.invalid_value(for_type, string.to_owned())
                })?
            },
            _ => return self.expected("characters"),
        };
        self.next()?;
        Ok(value)
    }

    /// Builds and returns an `Err(UnexpectedXml)`.
    fn expected<T, E: ToString>(&self, expected: E) -> ParseResult<T> {
        let expected = expected.to_string();
        let position = self.reader.position();

        Err(ParseError::UnexpectedXml {
            expected,
            position,
            found: match self.cur {
                XmlEvent::StartElement { ref name, .. } => Some(format!("<{}>", name)),
                XmlEvent::EndElement { ref name, .. } => Some(format!("</{}>", name)),
                XmlEvent::EndDocument => Some("end of data".to_string()),
                XmlEvent::Characters(ref data)
                | XmlEvent::CData(ref data) => Some(format!("\"{}\"", data)),
                _ => None
            },
        })
    }

    fn invalid_value(&self, for_type: &'static str, value: String) -> ParseError {
        // FIXME: It might be neat to preserve the original error as the cause
        ParseError::InvalidValue {
            for_type,
            found: value,
            position: self.reader.position(),
        }
    }

    fn parse_response(&mut self) -> ParseResult<Response> {
        let response: Response;

        // <methodResponse>
        self.expect_open("methodResponse")?;

        // <fault> / <params>
        match self.cur.clone() {
            XmlEvent::StartElement { ref name, .. } => {
                if name.local_name == "fault" {
                    self.next()?;
                    let value = self.parse_value()?;
                    let fault = Fault::from_value(&value).ok_or_else(|| {
                        io::Error::new(ErrorKind::Other, "malformed <fault>")
                    })?;
                    response = Err(fault);
                } else if name.local_name == "params" {
                    self.next()?;
                    // <param>
                    self.expect_open("param")?;

                    let value = self.parse_value()?;
                    response = Ok(value);

                    // </param>
                    self.expect_close("param")?;
                } else {
                    return self.expected(format!("<fault> or <params>, got {}", name));
                }
            }
            _ => return self.expected("<fault> or <params>"),
        }

        Ok(response)
    }

    fn parse_value(&mut self) -> ParseResult<Value> {
        // <value>
        self.expect_open("value")?;

        if let Ok(()) = self.expect_close("value") {
            // empty value, parse as empty string
            return Ok(Value::String(String::new()));
        }

        let value = self.parse_value_inner()?;

        // </value>
        self.expect_close("value")?;

        Ok(value)
    }

    fn parse_value_inner(&mut self) -> ParseResult<Value> {
        let value = match self.cur.clone() {
            // Raw string or specific type tag

            XmlEvent::StartElement { ref name, .. } => {
                let name = &*name.local_name;
                match name {
                    "struct" => {
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
                            let name = match self.cur {
                                XmlEvent::Characters(ref string) => string.clone(),
                                _ => return self.expected("characters"),
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
                    "array" => {
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
                    "nil" => {
                        self.next()?;
                        self.expect_close("nil")?;
                        Value::Nil
                    }
                    "string" => {
                        self.next()?;
                        let string = match self.cur.clone() {
                            XmlEvent::Characters(string) => {
                                self.next()?;
                                self.expect_close("string")?;
                                string
                            },
                            XmlEvent::EndElement { ref name } if name.local_name == "string" => {
                                self.next()?;
                                String::new()
                            },
                            _ => return self.expected("characters or </string>"),
                        };
                        Value::String(string)
                    }
                    "base64" => {
                        self.next()?;
                        let data = match self.cur.clone() {
                            XmlEvent::Characters(ref string) => {
                                self.next()?;
                                self.expect_close("base64")?;

                                let config = base64::Config::new(
                                    base64::CharacterSet::Standard,
                                    true,       // enable padding (default)
                                    true,       // accept and remove whitespace/linebreaks
                                    base64::LineWrap::NoWrap,   // ignored for `decode`
                                );
                                base64::decode_config(string, config).map_err(|_| {
                                    self.invalid_value("base64", string.to_string())
                                })?
                            },
                            XmlEvent::EndElement { ref name } if name.local_name == "base64" => {
                                self.next()?;
                                Vec::new()
                            },
                            _ => return self.expected("characters or </base64>"),
                        };
                        Value::Base64(data)
                    }
                    "i4" | "int" => {
                        self.next()?;
                        let value = self.expect_value("integer", |data| data.parse::<i32>().map(Value::Int))?;
                        self.expect_close(name)?;
                        value
                    }
                    "i8" => {
                        self.next()?;
                        let value = self.expect_value("i8", |data| data.parse::<i64>().map(Value::Int64))?;
                        self.expect_close(name)?;
                        value
                    }
                    "boolean" => {
                        self.next()?;
                        let value = self.expect_value("boolean", |data| {
                            match data {
                                "0" => Ok(Value::Bool(false)),
                                "1" => Ok(Value::Bool(true)),
                                _ => Err(()),
                            }
                        })?;
                        self.expect_close(name)?;
                        value
                    }
                    "double" => {
                        self.next()?;
                        let value = self.expect_value("double", |data| data.parse::<f64>().map(Value::Double))?;
                        self.expect_close(name)?;
                        value
                    }
                    "dateTime.iso8601" => {
                        self.next()?;
                        let value = self.expect_value("dateTime.iso8601", |data| datetime(data).map(Value::DateTime))?;
                        self.expect_close(name)?;
                        value
                    }
                    _ => return self.expected("valid type tag"),
                }
            }
            XmlEvent::Characters(string) => {
                self.next()?;
                Value::String(string)
            }
            _ => return self.expected("type tag or characters"),
        };

        Ok(value)
    }
}

/// Parses a response from an XML reader.
pub fn parse_response<R: Read>(reader: &mut R) -> ParseResult<Response> {
    Parser::new(reader)?.parse_response()
}

#[cfg(test)]
mod tests {
    use super::*;

    use Value;
    use error::Fault;

    use std::fmt::Debug;
    use std::iter;

    fn read_response(xml: &str) -> ParseResult<Response> {
        parse_response(&mut xml.as_bytes())
    }

    fn read_value(xml: &str) -> ParseResult<Value> {
        Parser::new(&mut xml.as_bytes())?.parse_value()
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
        Ok(Err(Fault {
            fault_code: 4,
            fault_string: "Too many parameters.".into(),
        })));
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

        assert_eq!(read_value("<value><base64> 0J/Qv tC10YXQ sNC70 Lgh  </base64></value>"),
           Ok(Value::Base64("Поехали!".bytes().collect())));

        assert_eq!(read_value("<value><base64>\n0J/QvtC10\nYXQsNC7\n0Lgh\n</base64></value>"),
           Ok(Value::Base64("Поехали!".bytes().collect())));
    }

    #[test]
    fn parses_empty_base64() {
        assert_eq!(read_value("<value><base64></base64></value>"),
                   Ok(Value::Base64(Vec::new())));
        assert_eq!(read_value("<value><base64/></value>"),
                   Ok(Value::Base64(Vec::new())));
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
            Ok(Value::String(String::new())));
        assert_eq!(read_value("<value><string/></value>"),
            Ok(Value::String(String::new())));
    }

    #[test]
    fn parses_empty_value_as_string() {
        assert_eq!(read_value("<value></value>"),
                   Ok(Value::String(String::new())));
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

        assert_eq!(
            errstr(r#"<value><SURPRISE></SURPRISE></value>"#),
            "unexpected XML at 1:8 (expected valid type tag, found <SURPRISE>)"
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

    #[test]
    fn duplicate_struct_member() {
        // Duplicate struct members are overwritten with the last one
        assert_eq!(read_value(r#"
            <value>
                <struct>
                    <member>
                        <name>A</name>
                        <value>first</value>
                    </member>
                    <member>
                        <name>A</name>
                        <value>second</value>
                    </member>
                </struct>
            </value>
            "#), Ok(Value::Struct(
                iter::once((
                    "A".into(), "second".into()
                )).collect()
            ))
        );
    }
}
