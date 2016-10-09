pub fn escape_xml(s: &str) -> String {
    s.replace("&", "&amp;").replace("<", "&lt;")
}
