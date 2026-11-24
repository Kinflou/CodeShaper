// Standard Uses

// Crate Uses

// External Uses
use codeshaper_lsp::groups::link_at;
use codeshaper_lsp::syntax::Outline;


fn linked(patch: &str, needle: &str, shift: usize) -> Option<(String, Vec<String>)> {
    let offset = patch.find(needle).unwrap() + shift;
    let link = link_at(&Outline::parse(patch), patch, offset)?;

    Some((patch[link.group].to_string(), link.uses.into_iter().map(|range| patch[range].to_string()).collect()))
}

fn replacer(from: &str, to: &str, flags: &str) -> String {
    format!("actions {{\n    replacer {{\n        from {from}\n        to {to}\n        flags \"{flags}\"\n    }}\n}}\n")
}


#[test]
fn locals_lead_to_their_group_and_back() {
    let patch = replacer(r#"r"(\w+) ((x)y)""#, r##""é\t$[1] #[a]($[0]) $[1]""##, "");

    assert_eq!(linked(&patch, "$[0]", 2), Some((r"(\w+)".to_string(), vec!["$[0]".to_string()])));
    assert_eq!(linked(&patch, "$[1]", 0), Some(("((x)y)".to_string(), vec!["$[1]".to_string(), "$[1]".to_string()])));
    assert_eq!(linked(&patch, r"\w", 1), Some((r"(\w+)".to_string(), vec!["$[0]".to_string()])));
    assert_eq!(linked(&patch, "(x)", 1), Some(("(x)".to_string(), vec![])));
    assert_eq!(linked(&patch, "#[a]", 2), None);
}

#[test]
fn escaped_patterns_keep_their_ranges() {
    let patch = replacer(r#""\\t\"(\\w+)""#, r#""$[0]""#, "");

    assert_eq!(linked(&patch, "$[0]", 2), Some((r"(\\w+)".to_string(), vec!["$[0]".to_string()])));
}

#[test]
fn windows_endings_keep_their_ranges() {
    for (from, to) in [("r\"é\r\n(a)\"", "\"\r\n$[0]\""), ("\"é\r\n(a)\"", "r\"$[0]\"")] {
        assert_eq!(linked(&replacer(from, to, ""), "$[0]", 2), Some(("(a)".to_string(), vec!["$[0]".to_string()])));
    }
}

#[test]
fn missing_groups_and_broken_patterns_have_no_link() {
    assert_eq!(linked(&replacer(r#"r"(a)""#, r#""$[1]""#, ""), "$[1]", 2), None);
    assert_eq!(linked(&replacer(r#"r"(a""#, r#""$[0]""#, ""), "$[0]", 2), None);
    assert_eq!(linked(&replacer("r\"(a) # (b)\"", r#""$[1]""#, "x"), "$[1]", 2), None);
}

#[test]
fn names_and_numbers_share_a_link() {
    let patch = replacer(r#"r"(a)(?P<n>b)""#, r#""$[n] $[1] $[0]""#, "");

    assert_eq!(linked(&patch, "$[n]", 2), Some(("(?P<n>b)".to_string(), vec!["$[n]".to_string(), "$[1]".to_string()])));
}
