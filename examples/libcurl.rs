extern crate curl;
extern crate xmlrpc;
use curl::easy::{Easy, List};
use std::{error::Error, io::Cursor};
use xmlrpc::{Request,Transport};

static CACERTFILE: &str = "chain.pem";

pub(crate) struct CurlTransport(pub Easy);

impl Transport for CurlTransport {
    type Stream = Cursor<Vec<u8>>;

    fn transmit(mut self, request: &Request) -> Result<Self::Stream, Box<dyn Error + Send + Sync>> {
        let mut body = Vec::new();

        // This unwrap never panics as we are using `Vec<u8>` as a `Write` implementor,
        // and not doing anything else that could return an `Err` in `write_as_xml()`.
        request.write_as_xml(&mut body).unwrap();

        let mut list = List::new();
        list.append("Content-Type: text/xml; charset=utf-8")?;
        list.append(format!("Content-Length: {}", body.len()).as_str())?;
        self.0.http_headers(list)?;

        self.0.post_fields_copy(body.as_slice())?;

        // start optional libcurl settings
        self.0.capath(CACERTFILE)?;
        self.0.verbose(true)?;
        // end optional libcurl settings

        let mut buf = Vec::new();
        {
            let mut tx = self.0.transfer();
            tx.write_function(|data| {
                buf.extend_from_slice(String::from_utf8_lossy(data).as_bytes());
                Ok(data.len())
            })?;
            tx.perform()?;
        }

        Ok(Cursor::new(buf))
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use xmlrpc::Value;

    #[test]
    fn test_ping_00() {
        let request = xmlrpc::Request::new("DataService.echo").arg("hello world".to_string());

        let easy = Easy::new();

        let tp = CurlTransport(easy);
        let result = request.call(tp);

        assert_eq!(true, result.is_ok());
    }
}
