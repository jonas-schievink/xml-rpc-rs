use iso8601;

pub fn escape_xml(s: &str) -> String {
    s.replace("&", "&amp;").replace("<", "&lt;")
}

pub fn format_datetime(date_time: &iso8601::DateTime) -> String {
    let iso8601::Time { hour, minute, second, .. } = date_time.time;
    
    match date_time.date {
        iso8601::Date::YMD { year, month, day } => {
            format!("{:04}{:02}{:02}T{:02}:{:02}:{:02}",
                year, month, day,
                hour, minute, second
            )
        }
        _ => { unimplemented!() }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use iso8601;

    #[test]
    fn formats_datetimes() {
        let date_time = iso8601::datetime("2016-05-02T06:01:05-0800").unwrap();

        assert_eq!(format_datetime(&date_time), "20160502T06:01:05");
    }
}
