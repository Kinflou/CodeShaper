// Standard Uses

// Crate Uses

// External Uses
use codeshaper::core::shaping::patch::Patch;
use codeshaper_lsp::section::views;
use codeshaper_lsp::syntax::Outline;
use codeshaper_lsp::toggle::{toggle, Toggle};


const FIELDS: &str = "file \"*.cpp\"\nalias \"cpp\"\nactions {\n}\n";

fn toggles(text: &str) -> Vec<Option<Toggle>> {
    views(text).iter().map(|view| toggle(&Outline::parse(&view.text), text)).collect()
}

fn applied(text: &str, number: usize, title: &str) -> String {
    let Toggle { title: found, value, text: new, .. } = toggles(text)[number].clone().unwrap();
    assert_eq!(found, title);

    let mut text = text.to_string();
    text.replace_range(value, &new);
    text
}

fn enabled(text: &str) -> Vec<bool> {
    Patch::parse_all("", text).unwrap().into_iter().map(|named| named.patch.enabled).collect()
}

fn check(text: &str, number: usize, expected: &str) {
    let before = Patch::parse_all("", text).unwrap();
    assert!(before.iter().all(|named| named.patch.enabled), "{text}");

    let disabled = applied(text, number, "Disable patch");
    assert_eq!(disabled, expected);
    for (index, (before, after)) in before.iter().zip(Patch::parse_all("", &disabled).unwrap()).enumerate() {
        assert_eq!(Patch { enabled: index != number, ..before.patch.clone() }, after.patch);
    }

    let (start, end) = expected.split_at(expected.rfind("enabled false").unwrap());
    assert_eq!(applied(&disabled, number, "Enable patch"), start.to_string() + &end.replacen("false", "true", 1));
    assert!(enabled(&applied(&disabled, number, "Enable patch")).into_iter().all(|enabled| enabled));
}


#[test]
fn missing_fields_are_added() {
    check(FIELDS, 0, &format!("enabled false\n{FIELDS}"));
    check(&FIELDS.replace('\n', "\r\n"), 0, &format!("enabled false\n{FIELDS}").replace('\n', "\r\n"));
    check("file \"*\"; actions", 0, "enabled false\nfile \"*\"; actions");
    check(&format!("// é 𝄞\n\n{FIELDS}"), 0, &format!("// é 𝄞\n\nenabled false\n{FIELDS}"));
    check("/* é */ file \"*\"\nactions\n", 0, "/* é */ enabled false; file \"*\"\nactions\n");
    check("/-enabled false\nfile \"*\"\nactions\n", 0, "/-enabled false\nenabled false\nfile \"*\"\nactions\n");
}

#[test]
fn blocks_get_their_own_field() {
    let blocks = "patch \"é\" {\n\tfile \"*\"\n\tactions\n}\npatch \"b\" { file \"*\"; actions; }\npatch \"c\" {\n\n    // d\n    actions\n    file \"*\"\n}\n";

    check(blocks, 0, &blocks.replacen("\tfile", "\tenabled false\n\tfile", 1));
    check(blocks, 1, &blocks.replacen("{ file", "{ enabled false; file", 1));
    check(blocks, 2, &blocks.replacen("    actions", "    enabled false\n    actions", 1));
    check(&blocks.replace('\n', "\r\n"), 2, &blocks.replacen("    actions", "    enabled false\n    actions", 1).replace('\n', "\r\n"));
}

#[test]
fn written_fields_are_changed() {
    let text = FIELDS.to_string() + "enabled true\n";

    assert_eq!(applied(&text, 0, "Disable patch"), FIELDS.to_string() + "enabled false\n");
    assert_eq!(applied(&applied(&text, 0, "Disable patch"), 0, "Enable patch"), text);
}

#[test]
fn nothing_is_toggled_without_a_patch() {
    for text in ["", "// file\n", "enabled\nfile \"*\"\n", "enabled 1\nfile \"*\"\n", "enabled fal\nfile \"*\"\n", "patch \"a\" {\n}\n", "patch \"a\"\n", "/-file \"*\"\n"] {
        assert_eq!(toggles(text), vec![None], "{text}");
    }
}
