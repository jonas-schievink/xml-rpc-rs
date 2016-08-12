//! XML-RPC response parser.

use {Value, Response, Fault};
use error::ParseError;

use base64;
use xml::reader::{XmlEvent, EventReader};
use xml::name::OwnedName;
use chrono::DateTime;
use std::io::{self, ErrorKind, Read};
use std::collections::BTreeMap;

pub type ParseResult<T> = Result<T, ParseError>;

/// Reads an `XmlEvent` from a reader, disposing events we want to ignore.
fn pull_event<R: Read>(reader: &mut EventReader<R>) -> ParseResult<XmlEvent> {
    loop {
        let event = try!(reader.next());
        match event {
            XmlEvent::StartDocument { .. }
            | XmlEvent::Comment(_)
            | XmlEvent::Whitespace(_)
            | XmlEvent::ProcessingInstruction { .. } => {},
            XmlEvent::StartElement { .. }
            | XmlEvent::EndElement { .. }
            | XmlEvent::EndDocument
            | XmlEvent::CData(_)
            | XmlEvent::Characters(_) => return Ok(event),
        }
    }
}

/// Expects an opening tag like `<tag>` without attributes (and a local name without namespaces).
fn expect_open<R: Read>(reader: &mut EventReader<R>, tag: &str) -> ParseResult<()> {
    match try!(pull_event(reader)) {
        XmlEvent::StartElement { ref name, ref attributes, .. }
        if name == &OwnedName::local(tag) => {
            if !attributes.is_empty() {
                return unexpected(format!("unexpected attributes in <{}>", tag));
            }

            Ok(())
        }
        _ => return unexpected(format!("expected <{}>", tag)),
    }
}

/// Expects a closing tag like `</tag>` with a local name without namespaces.
fn expect_close<R: Read>(reader: &mut EventReader<R>, tag: &str) -> ParseResult<()> {
    match try!(pull_event(reader)) {
        XmlEvent::EndElement { ref name } if name == &OwnedName::local(tag) => {
            Ok(())
        }
        _ => unexpected(format!("expected </{}>", tag)),
    }
}

/// Builds and returns an `Err(UnexpectedXml)`.
fn unexpected<T, E: ToString>(err: E) -> ParseResult<T> {
    Err(ParseError::UnexpectedXml(err.to_string()))
}

/// Parses a response from an XML reader.
pub fn parse_response<R: Read>(reader: &mut EventReader<R>) -> ParseResult<Response> {
    let response: Response;

    // <methodResponse>
    try!(expect_open(reader, "methodResponse"));

    // <fault> / <params>
    match try!(pull_event(reader)) {
        XmlEvent::StartElement { ref name, ref attributes, .. } => {
            if !attributes.is_empty() {
                return unexpected("unexpected attributes");
            }

            if name == &OwnedName::local("fault") {
                let value = try!(parse_value(reader));
                let fault = try!(Fault::from_value(value).ok_or_else(|| {
                    io::Error::new(ErrorKind::Other, "malformed <fault>")
                }));
                response = Err(fault);
            } else if name == &OwnedName::local("params") {
                // <param>
                try!(expect_open(reader, "param"));

                let value = try!(parse_value(reader));
                response = Ok(value);

                // </param>
                try!(expect_close(reader, "param"));
            } else {
                return unexpected(format!("expected <fault> or <params>, got {}", name));
            }
        }
        _ => return unexpected("expected <fault> or <params>"),
    }

    Ok(response)
}

pub fn parse_value<R: Read>(reader: &mut EventReader<R>) -> ParseResult<Value> {
    let value: Value;

    // <value>
    try!(expect_open(reader, "value"));

    // Raw string or specific type tag
    value = match try!(pull_event(reader)) {
        XmlEvent::StartElement { ref name, ref attributes, .. } => {
            if !attributes.is_empty() {
                return unexpected(format!("unexpected attributes in <{}>", name));
            }

            if name == &OwnedName::local("struct") {
                let mut members = BTreeMap::new();
                loop {
                    match try!(pull_event(reader)) {
                        XmlEvent::EndElement { ref name } if name == &OwnedName::local("struct") => break,
                        XmlEvent::StartElement { ref name, ref attributes, .. } if name == &OwnedName::local("member") => {
                            // <member>
                            if !attributes.is_empty() {
                                return unexpected(format!("unexpected attributes in <{}>", name));
                            }

                            // <name>NAME</name>
                            try!(expect_open(reader, "name"));
                            let name = match try!(pull_event(reader)) {
                                XmlEvent::Characters(string) => string,
                                _ => return unexpected("expected CDATA"),
                            };
                            try!(expect_close(reader, "name"));

                            // Value
                            let value = try!(parse_value(reader));

                            // </member>
                            try!(expect_close(reader, "member"));

                            members.insert(name, value);
                        }
                        _ => return unexpected("expected </struct> or <member>"),
                    }
                }

                Value::Struct(members)
            } else if name == &OwnedName::local("array") {
                unimplemented!();   // TODO
            } else {
                // All other types expect raw characters...
                let data = match try!(pull_event(reader)) {
                    XmlEvent::Characters(string) => string,
                    _ => return unexpected("expected characters"),
                };

                // ...and a corresponding close tag
                try!(expect_close(reader, &name.local_name));

                if name == &OwnedName::local("i4") || name == &OwnedName::local("int") {
                    Value::Int(try!(data.parse::<i32>().map_err(|_| {
                        io::Error::new(ErrorKind::Other, format!("invalid value for integer: {}", data))
                    })))
                } else if name == &OwnedName::local("boolean") {
                    let val = match data.trim() {
                        "0" => false,
                        "1" => true,
                        _ => return Err(io::Error::new(ErrorKind::Other, format!("invalid value for <boolean>: {}", data)).into())
                    };

                    Value::Bool(val)
                } else if name == &OwnedName::local("string") {
                    Value::String(data.clone())
                } else if name == &OwnedName::local("double") {
                    Value::Double(try!(data.parse::<f64>().map_err(|_| {
                        io::Error::new(ErrorKind::Other, format!("invalid value for double: {}", data))
                    })))
                } else if name == &OwnedName::local("dateTime.iso8601") {
                    Value::DateTime(try!(DateTime::parse_from_rfc3339(&data).map_err(|_| {
                        io::Error::new(ErrorKind::Other, format!("invalid value for dateTime.iso8601: {}", data))
                    })))
                } else if name == &OwnedName::local("base64") {
                    Value::Base64(try!(base64::decode(&data).map_err(|_| {
                        io::Error::new(ErrorKind::Other, format!("invalid value for base64: {}", data))
                    })))
                } else {
                    return unexpected("invalid <value> content");
                }
            }
        }
        XmlEvent::Characters(string) => {
            Value::String(string)
        }
        _ => return unexpected("invalid <value> content"),
    };

    // </value>
    try!(expect_close(reader, "value"));

    Ok(value)
}

#[cfg(test)]
mod tests {
    use super::*;

    use {Response, Value};
    use xml::EventReader;
    use error::Fault;

    fn read_response(xml: &str) -> ParseResult<Response> {
        parse_response(&mut EventReader::from_str(xml))
    }

    fn read_value(xml: &str) -> ParseResult<Value> {
        parse_value(&mut EventReader::from_str(xml))
    }

    #[test]
    fn fault_response() {
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

        assert!(read_response(r##"
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
   </methodResponse>"##).is_err());
    }

    #[test]
    fn values() {
        assert_eq!(read_value("<value>  I'm a string!  </value>"),
            Ok(Value::String("  I'm a string!  ".into())));
    }
}
