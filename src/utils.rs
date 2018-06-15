use iso8601::{Date, Time, DateTime};
use xml::escape::escape_str_pcdata;

use std::borrow::Cow;
use std::fmt::Write;

/// Escape a string for use as XML characters.
///
/// The resulting string is *not* suitable for use in XML attributes, but XML-RPC doesn't use those.
pub fn escape_xml(s: &str) -> Cow<str> {
    escape_str_pcdata(s)
}

/// Formats a `DateTime` for use in XML-RPC.
///
/// Note that XML-RPC is extremely underspecified when it comes to datetime values. Apparently,
/// some clients [don't even support timezone information][wp-bug] (we do). For maximum
/// interoperability, this will omit fractional time and time zone if not specified.
///
/// [wp-bug]: https://core.trac.wordpress.org/ticket/1633#comment:4
pub fn format_datetime(date_time: &DateTime) -> String {
    let Time {
        hour, minute, second, millisecond, tz_offset_hours, tz_offset_minutes
    } = date_time.time;
    
    match date_time.date {
        Date::YMD { year, month, day } => {
            // The base format is based directly on the example in the spec and should always work:
            let mut string = format!("{:04}{:02}{:02}T{:02}:{:02}:{:02}",
                year, month, day,
                hour, minute, second
            );
            // Only append milliseconds when they're >0
            if millisecond > 0 {
                write!(string, ".{:.3}", millisecond).unwrap();
            }
            // Only append time zone info if the offset is specified and not 00:00
            if tz_offset_hours != 0 || tz_offset_minutes != 0 {
                write!(string, "{:+03}:{:02}", tz_offset_hours, tz_offset_minutes.abs()).unwrap();
            }

            string
        }
        // Other format are just not supported at all:
        Date::Week { .. }
        | Date::Ordinal { .. } => {
            unimplemented!()
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
        assert_eq!(formatted, "20160502T06:01:05-08:30");
        assert_eq!(iso8601::datetime(&formatted).unwrap(), date_time);

        // milliseconds / fraction
        let date_time = iso8601::datetime("20160502T06:01:05.400").unwrap();
        let formatted = format_datetime(&date_time);
        assert_eq!(formatted, "20160502T06:01:05.400");
        assert_eq!(iso8601::datetime(&formatted).unwrap(), date_time);

        // milliseconds / fraction + time zone
        let date_time = iso8601::datetime("20160502T06:01:05.400+01:02").unwrap();
        let formatted = format_datetime(&date_time);
        assert_eq!(formatted, "20160502T06:01:05.400+01:02");
        assert_eq!(iso8601::datetime(&formatted).unwrap(), date_time);
    }
}
