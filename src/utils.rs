use iso8601::{Date, Time, DateTime};
use xml::escape::escape_str_pcdata;

use std::borrow::Cow;

/// Escape a string for use as XML characters.
///
/// The resulting string is *not* suitable for use in XML attributes, but XML-RPC doesn't use those.
pub fn escape_xml(s: &str) -> Cow<str> {
    escape_str_pcdata(s)
}

pub fn format_datetime(date_time: &DateTime) -> String {
    let Time {
        hour, minute, second, millisecond, tz_offset_hours, tz_offset_minutes
    } = date_time.time;
    
    match date_time.date {
        Date::YMD { year, month, day } => {
            format!("{:04}{:02}{:02}T{:02}:{:02}:{:02}.{:.3}{:+03}:{:02}",
                year, month, day,
                hour, minute, second, millisecond,
                tz_offset_hours, tz_offset_minutes.abs()
            )
        }
        Date::Week { year, ww, d } => {
            format!("{:04}-W{:02}-{}", year, ww, d)
        }
        Date::Ordinal { year, ddd } => {
            format!("{:04}-{:03}", year, ddd)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use iso8601;

    #[test]
    fn formats_datetimes() {
        let date_time = iso8601::datetime("2016-05-02T06:01:05-0830").unwrap();

        let formatted = format_datetime(&date_time);
        assert_eq!(formatted, "20160502T06:01:05.0-08:30");
        assert_eq!(iso8601::datetime(&formatted).unwrap(), date_time);
    }
}
