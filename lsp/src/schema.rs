// Standard Uses

// Crate Uses

// External Uses
use codeshaper::core::shaping::patch::file::BLOCK;


pub const LOCATIONS: [&str; 2] = ["location", "reference-location"];


pub fn regex_field(action: &str) -> Option<&'static str> {
    match action {
        "replacer" => Some("from"),
        "builder" => Some("match"),
        "maker" => Some("prepare"),
        _ => None
    }
}

pub fn template_field(action: &str) -> Option<&'static str> {
    match action {
        "replacer" => Some("to"),
        "builder" => Some("build"),
        "maker" => Some("make"),
        _ => None
    }
}

pub fn result_field(action: &str) -> Option<&'static str> {
    match action {
        "builder" => Some("build"),
        "maker" => Some("make"),
        "switch" | "list" => Some("index"),
        _ => None
    }
}

pub fn top(has_blocks: bool, has_fields: bool) -> Vec<&'static str> {
    let fields = children(None).iter().copied();

    match (has_blocks, has_fields) {
        (true, _) => vec![BLOCK],
        (false, true) => fields.collect(),
        (false, false) => fields.chain([BLOCK]).collect()
    }
}

pub fn children(parent: Option<&str>) -> &'static [&'static str] {
    match parent {
        None => &["enabled", "alias", "file", "actions"],
        Some("actions") => &["builder", "replacer", "maker", "switch", "list"],
        Some("builder") => &["location", "reference-location", "match", "flags", "build", "collect", "separator", "actions"],
        Some("replacer") => &["location", "from", "to", "flags", "reference-location", "reference", "actions"],
        Some("maker") => &["prepare", "flags", "make"],
        Some("switch") => &["cases", "index", "default"],
        Some("list") => &["values", "index", "default"],
        Some(_) => &[]
    }
}
