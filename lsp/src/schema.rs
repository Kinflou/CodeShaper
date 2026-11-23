// Standard Uses

// Crate Uses

// External Uses


pub const LOCATIONS: [&str; 2] = ["location", "reference-location"];


pub fn regex_field(action: &str) -> Option<&'static str> {
    match action {
        "replacer" => Some("from"),
        "builder" => Some("match"),
        "maker" => Some("prepare"),
        _ => None
    }
}

pub fn children(parent: Option<&str>) -> &'static [&'static str] {
    match parent {
        None => &["enabled", "alias", "file", "actions"],
        Some("actions") => &["builders", "replacers", "makers", "switchers", "lists"],
        Some("builders") => &["builder"],
        Some("replacers") => &["replacer"],
        Some("makers") => &["maker"],
        Some("switchers") => &["switch"],
        Some("lists") => &["list"],
        Some("builder") => &["location", "reference-location", "match", "build", "collect", "separator", "actions"],
        Some("replacer") => &["location", "from", "to", "flags", "reference-location", "reference", "actions"],
        Some("maker") => &["prepare", "make"],
        Some("switch") => &["cases", "index", "default"],
        Some("list") => &["values", "index", "default"],
        Some(_) => &[]
    }
}
