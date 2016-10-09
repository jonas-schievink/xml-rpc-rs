pub fn escape_xml(s: &str) -> String {
    // FIXME: This shouldn't need to allocate in all cases
    s.replace("&", "&amp;").replace("<", "&lt;")
}
