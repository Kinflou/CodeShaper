// Standard Uses
use std::fs;
use std::path::Path;

// Crate Uses
use crate::diagnostics::UNCOVERED;
use crate::preview::{operation, RENAME};
use crate::protocol::Client;

// External Uses
use codeshaper::core::shaping::patch::Patch;
use codeshaper_lsp::diagnostics::{diagnostics, duplicates};
use codeshaper_lsp::position::LineIndex;
use codeshaper_lsp::section::{only, sections, view_at, views};
use codeshaper_lsp::symbols::symbols;
use codeshaper_lsp::syntax::Outline;
use lsp_server::Message;
use lsp_types::{Diagnostic, NumberOrString, SymbolKind, Uri};
use serde_json::{json, Value};


const CALLED: &str = r##"enabled true
alias "cpp"
file "a.cpp"

actions {
    maker "shared" {
        make "fresh"
    }
    replacer {
        location "FunctionDeclarator.Declarator"
        from r"(old)"
        to "#[shared]"
    }
}
"##;


fn block(name: &str, content: &str) -> String {
    format!("patch \"{name}\" {{\n{content}}}\n")
}

fn several() -> String {
    format!("// 日 🦀\n{}\n{}", block("é", RENAME), block("two", CALLED))
}

fn place(text: &str, needle: &str, skip: usize) -> Value {
    let found = text.rfind(needle).unwrap() + skip;

    json!(LineIndex::new(text).position(found))
}

fn at(uri: &Uri, text: &str, needle: &str, skip: usize) -> Value {
    json!({ "textDocument": { "uri": uri.as_str() }, "position": place(text, needle, skip) })
}

fn ask(client: &mut Client, method: &str, params: Value) -> Value {
    client.request(method, params).response_result.unwrap()
}

fn marked(text: &str, range: &Value) -> String {
    let range: lsp_types::Range = serde_json::from_value(range.clone()).unwrap();
    let index = LineIndex::new(text);

    text[index.offset(range.start)..index.offset(range.end)].to_string()
}

fn labels(found: &Value) -> Vec<String> {
    found.as_array().unwrap().iter().map(|item| item["label"].as_str().unwrap().to_string()).collect()
}

fn wait(client: &mut Client, uri: &Uri) {
    ask(client, "textDocument/hover", json!({ "textDocument": { "uri": uri.as_str() }, "position": { "line": 0, "character": 0 } }));
}

fn lenses(client: &mut Client, uri: &Uri) -> Vec<(String, u64, Vec<String>)> {
    wait(client, uri);

    ask(client, "textDocument/codeLens", json!({ "textDocument": { "uri": uri.as_str() } })).as_array().unwrap().iter()
        .map(|lens| (
            lens["command"]["title"].as_str().unwrap().to_string(),
            lens["range"]["start"]["line"].as_u64().unwrap(),
            lens["command"]["arguments"].as_array().unwrap()[1..].iter().map(|name| name.as_str().unwrap().to_string()).collect()
        ))
        .collect()
}

fn codes(found: &[Diagnostic]) -> Vec<&str> {
    found.iter()
        .filter_map(|diagnostic| match &diagnostic.code {
            Some(NumberOrString::String(code)) => Some(code.as_str()),
            _ => None
        })
        .collect()
}

fn save(client: &Client, uri: &Uri, text: &str) {
    client.notify("textDocument/didSave", json!({ "textDocument": { "uri": uri.as_str() }, "text": text }));
}

fn published(client: &Client, uri: &Uri, code: &str, wanted: bool) {
    loop {
        let Message::Notification(notification) = client.receive() else { continue };
        if notification.params["uri"] != uri.as_str() { continue }

        let found = notification.params["diagnostics"].as_array().unwrap().iter().any(|diagnostic| diagnostic["code"] == code);
        if found == wanted { return }
    }
}


#[test]
fn files_without_blocks_are_seen_whole() {
    assert_eq!(sections(RENAME), vec![]);
    assert_eq!(views(RENAME).iter().map(|view| view.text.as_ref()).collect::<Vec<_>>(), vec![RENAME]);
    assert_eq!(sections("// patch\nenabled true\n"), vec![]);
}

#[test]
fn blocks_are_found_with_their_places() {
    let text = several();
    let found = sections(&text);

    assert_eq!(found.iter().map(|section| section.name.as_deref()).collect::<Vec<_>>(), vec![Some("é"), Some("two")]);
    assert_eq!(&text[found[0].named.clone()], "\"é\"");
    assert_eq!(text[found[0].whole.clone()], *block("é", RENAME).trim_end());
    assert_eq!(text[found[1].content.clone()], format!("\n{CALLED}"));
}

#[test]
fn blanking_keeps_every_position() {
    for text in [several(), several().replace('\n', "\r\n"), "patch \"é日🦀\" { enabled true; }; patch \"b\" { file \"é\"; }".to_string()] {
        for section in sections(&text) {
            let blanked = only(&text, &section.content);
            let (whole, alone) = (LineIndex::new(&text), LineIndex::new(&blanked));

            assert_eq!(blanked.len(), text.len());
            assert_eq!(blanked.encode_utf16().count(), text.encode_utf16().count());
            for offset in section.content.clone().filter(|offset| text.is_char_boundary(*offset)) {
                assert_eq!(whole.position(offset), alone.position(offset));
                assert_eq!(alone.offset(whole.position(offset)), offset);
            }
        }
    }
}

#[test]
fn blanked_blocks_read_like_the_block_alone() {
    for text in [several(), several().replace('\n', "\r\n")] {
        for view in views(&text) {
            let content = text[view.section.unwrap().content].to_string();
            let names = |text: &str| Outline::parse(text).nodes.into_iter().map(|node| (node.name, node.parent)).collect::<Vec<_>>();

            assert_eq!(names(&view.text), names(&content));
            assert_eq!(Patch::from_kdl("", &view.text).unwrap(), Patch::from_kdl("", &content).unwrap());
        }
    }
}

#[test]
fn broken_text_still_has_blocks() {
    let text = format!("{}patch \"two\" {{\n    enabled true\n    file \"", block("one", RENAME));
    let found = sections(&text);

    assert_eq!(found.iter().map(|section| section.name.as_deref()).collect::<Vec<_>>(), vec![Some("one"), Some("two")]);
    assert_eq!(found[1].content.end, text.len());
    assert!(sections("patch {").iter().all(|section| section.name.is_none()));
    assert_eq!(sections("patch").len(), 1);
    assert_eq!(sections("patch \"").len(), 1);
}

#[test]
fn commented_blocks_are_not_found() {
    assert_eq!(sections(&format!("/-{}", block("gone", RENAME))), vec![]);
}

#[test]
fn offsets_outside_of_blocks_see_the_file() {
    let text = several();

    assert!(view_at(&text, 0).section.is_none());
    assert!(view_at(&text, text.find("patch \"two\"").unwrap() + 3).section.is_none());
    assert_eq!(view_at(&text, text.rfind("actions").unwrap()).name(), Some("two"));
}

#[test]
fn each_block_is_diagnosed() {
    let (_directory, project) = operation();
    let path = project.join("patches/rename.kdl");
    let text = block("one", &CALLED.replace("#[shared]", "#[lost]")) + &block("é", &CALLED.replace("(old)", "("));
    let found = diagnostics(&codeshaper::registry(), &path, &text);
    let places: Vec<String> = found.iter().map(|found| marked(&text, &json!(found.range))).collect();

    for (code, place) in [("action-not-found", "\"#[lost]\""), ("incorrect-regex", "r\"(\"")] {
        let index = codes(&found).iter().position(|found| *found == code).unwrap();
        assert!(place.contains(&places[index]) || places[index].contains(place), "{code}: {places:?}");
    }
}

#[test]
fn one_broken_block_hides_no_others() {
    let (_directory, project) = operation();
    let path = project.join("patches/rename.kdl");
    let text = block("one", &RENAME.replace("enabled false\n", "")) + &block("two", &CALLED.replace("#[shared]", "#[lost]"));
    let found = diagnostics(&codeshaper::registry(), &path, &text);

    assert!(found.iter().any(|found| found.message.contains("`enabled` is required") && found.range.start.line == 0));
    assert!(codes(&found).contains(&"action-not-found"));

    let unclosed = text.trim_end().trim_end_matches('}').to_string();
    assert!(codes(&diagnostics(&codeshaper::registry(), &path, &unclosed)).is_empty());
}

#[test]
fn loose_fields_are_diagnosed() {
    let (_directory, project) = operation();
    let text = format!("enabled true\n{}", block("one", CALLED));
    let found = diagnostics(&codeshaper::registry(), &project.join("patches/rename.kdl"), &text);

    assert_eq!(found.iter().filter(|found| found.message.contains("outside of the file's patches")).count(), 1);
}

#[test]
fn names_of_other_files_are_taken() {
    let (_directory, project) = operation();
    let patches = project.join("patches");
    fs::create_dir_all(patches.join("sub")).unwrap();
    fs::write(patches.join("sub/other.kdl"), block("two", RENAME)).unwrap();

    let text = block("one", RENAME) + &block("two", CALLED);
    let found = duplicates(&project, &patches.join("rename.kdl"), &text);
    assert_eq!(codes(&found), vec!["duplicate-patch"]);
    assert_eq!(found[0].message, "the project already has a patch named 'two', in sub/other.kdl");
    assert_eq!(marked(&text, &json!(found[0].range)), "\"two\"");

    assert_eq!(duplicates(&project, &patches.join("rename.kdl"), &block("one", RENAME)), vec![]);
    let clash = duplicates(&project, &patches.join("sub/other.kdl"), &block("rename.kdl", RENAME));
    assert_eq!(clash[0].message, "the project already has a patch named 'rename.kdl', in rename.kdl");
    assert_eq!(duplicates(&project, Path::new("/nowhere/rename.kdl"), &text), vec![]);
}

#[test]
fn saved_names_are_checked_in_other_files() {
    let (_directory, project) = operation();
    let (first, second) = (project.join("patches/rename.kdl"), project.join("patches/second.kdl"));
    let (one, two) = (block("one", RENAME), block("two", RENAME));
    fs::write(&first, &one).unwrap();
    fs::write(&second, &two).unwrap();

    let client = Client::start();
    let first_uri = client.open(&first, &one);
    let second_uri = client.open(&second, &two);
    published(&client, &second_uri, "duplicate-patch", false);

    fs::write(&second, &one).unwrap();
    save(&client, &second_uri, &one);
    published(&client, &first_uri, "duplicate-patch", true);

    fs::write(&second, &two).unwrap();
    save(&client, &second_uri, &two);
    published(&client, &first_uri, "duplicate-patch", false);

    client.stop();
}

#[test]
fn symbols_are_grouped_by_patch() {
    let text = several();
    let found = symbols(&text);
    let children = |index: usize| -> Vec<String> {
        let children: &Vec<lsp_types::DocumentSymbol> = found[index].children.as_ref().unwrap();
        children.iter().map(|child| child.name.clone()).collect()
    };

    assert_eq!(found.iter().map(|symbol| (symbol.name.as_str(), symbol.kind)).collect::<Vec<_>>(), vec![
        ("é", SymbolKind::NAMESPACE), ("two", SymbolKind::NAMESPACE)
    ]);
    assert_eq!(children(0), vec!["enabled", "alias", "file", "replacer #1"]);
    assert_eq!(children(1), vec!["enabled", "alias", "file", "shared", "replacer #1"]);
}

#[test]
fn names_are_completed_for_their_place() {
    let (_directory, project) = operation();
    let mut client = Client::start();
    let mut complete = |text: &str, offset: usize| {
        let uri = client.open(&project.join("patches/rename.kdl"), text);
        let position = LineIndex::new(text).position(offset);

        labels(&ask(&mut client, "textDocument/completion", json!({ "textDocument": { "uri": uri.as_str() }, "position": position })))
    };
    let text = several() + "\n";

    assert_eq!(complete(&text, text.len()), vec!["patch"]);
    assert_eq!(complete("patch \"one\" {\n    \n}\n", 18), vec!["enabled", "alias", "file", "actions"]);
    assert_eq!(complete("patch \"é\" { ", 13), vec!["enabled", "alias", "file", "actions"]);
    assert_eq!(complete("patch \"one\" {\n    actions {\n        ", 37)[0], "builder");

    let called = text.rfind("#[shared").unwrap() + 2;
    assert_eq!(complete(&text.replace("#[shared]", "#["), called), vec!["shared"]);
    let first = text.replacen("to \"new\"", "to \"#[", 1);
    assert!(complete(&first, first.find("to \"#[").unwrap() + 6).is_empty());

    client.stop();
}

#[test]
fn actions_belong_to_their_patch() {
    let (_directory, project) = operation();
    let mut client = Client::start();
    let text = block("one", CALLED) + &block("é", CALLED);
    let uri = client.open(&project.join("patches/rename.kdl"), &text);
    let here = at(&uri, &text, "#[shared", 4);
    let second = text.find("patch \"é\"").unwrap();

    let found = ask(&mut client, "textDocument/definition", here.clone());
    let start: lsp_types::Position = serde_json::from_value(found[0]["range"]["start"].clone()).unwrap();
    assert!(LineIndex::new(&text).offset(start) > second);

    let found = ask(&mut client, "textDocument/references", json!({
        "textDocument": here["textDocument"], "position": here["position"], "context": { "includeDeclaration": true }
    }));
    assert_eq!(found.as_array().unwrap().len(), 2);
    assert!(found.as_array().unwrap().iter().all(|found| found["range"]["start"]["line"].as_u64() > Some(18)));

    let found = ask(&mut client, "textDocument/documentHighlight", here.clone());
    assert_eq!(found.as_array().unwrap().len(), 2);

    let found = ask(&mut client, "textDocument/prepareRename", here.clone());
    assert_eq!(marked(&text, &found), "shared");

    let found = ask(&mut client, "textDocument/rename", json!({
        "textDocument": here["textDocument"], "position": here["position"], "newName": "renamed"
    }));
    assert_eq!(found["changes"][uri.as_str()].as_array().unwrap().len(), 2);

    let found = ask(&mut client, "textDocument/hover", here);
    assert!(found["contents"]["value"].as_str().unwrap().contains("Called from 1 place(s) in this patch"));
    assert_eq!(marked(&text, &found["range"]), "shared");

    client.stop();
}

#[test]
fn files_and_locations_use_their_patch() {
    let (_directory, project) = operation();
    let mut client = Client::start();
    let text = several();
    let uri = client.open(&project.join("patches/rename.kdl"), &text);

    let found = ask(&mut client, "textDocument/definition", at(&uri, &text, "\"a.cpp\"", 2));
    assert_eq!(found.as_array().unwrap().len(), 1);

    let here = at(&uri, &text, "\"FunctionDeclarator.Declarator\"", 3);
    let found = ask(&mut client, "textDocument/references", json!({
        "textDocument": here["textDocument"], "position": here["position"], "context": { "includeDeclaration": false }
    }));
    assert_eq!(found.as_array().unwrap().len(), 1);

    let found = ask(&mut client, "textDocument/hover", at(&uri, &text, "\"*.cpp\"", 2));
    assert!(found["contents"]["value"].as_str().unwrap().contains("Matches 2 target file(s)"));

    client.stop();
}

#[test]
fn lenses_belong_to_their_patch() {
    let (_directory, project) = operation();
    let mut client = Client::start();
    let text = several();
    let uri = client.open(&project.join("patches/rename.kdl"), &text);
    let named = |title: &str, line: u64, name: &str| (title.to_string(), line, vec![name.to_string()]);

    assert_eq!(lenses(&mut client, &uri), vec![
        ("Preview operation".to_string(), 0, vec![]),
        named("Preview result", 1, "é"), named("Explore target", 1, "é"),
        named("Enable patch", 2, "é"), named("1 replacement(s) in 1 file(s)", 7, "é"),
        named("Preview result", 15, "two"), named("Explore target", 15, "two"),
        named("Disable patch", 16, "two"), named("1 replacement(s) in 1 file(s)", 24, "two")
    ]);

    client.stop();
}

#[test]
fn previews_take_one_patch() {
    let (_directory, project) = operation();
    let mut client = Client::start();
    let text = several();
    let uri = client.open(&project.join("patches/rename.kdl"), &text);
    let mut preview = |extra: Value| {
        let mut params = json!({ "uri": uri.as_str() });
        params.as_object_mut().unwrap().extend(extra.as_object().unwrap().clone());

        client.request("codeshaper/preview", params).response_result
    };

    let matched = |found: Result<Value, lsp_server::ResponseError>| found.unwrap()["matched"].as_u64();
    assert_eq!(matched(preview(json!({ "name": "é" }))), Some(2));
    assert_eq!(matched(preview(json!({ "name": "two" }))), Some(1));
    assert_eq!(matched(preview(json!({ "position": place(&text, "shared", 0) }))), Some(1));
    assert_eq!(matched(preview(json!({ "position": place(&text, "patch \"two\"", 0) }))), Some(1));
    assert_eq!(matched(preview(json!({ "name": "é", "position": place(&text, "shared", 0) }))), Some(2));
    assert_eq!(preview(json!({})).unwrap(), json!({ "patches": ["é", "two"] }));
    assert_eq!(preview(json!({ "position": { "line": 0, "character": 0 } })).unwrap(), json!({ "patches": ["é", "two"] }));
    assert_eq!(preview(json!({ "position": place(&text, "shared", 0) })).unwrap()["patch"], "two");
    assert!(preview(json!({ "name": "three" })).err().unwrap().message.contains("no patch named 'three'"));

    client.stop();
}

#[test]
fn preview_errors_point_into_the_file() {
    let (_directory, project) = operation();
    let mut client = Client::start();
    let text = several().replace("from r\"(old)\"", "from 1");
    let uri = client.open(&project.join("patches/rename.kdl"), &text);

    let error = client.request("codeshaper/preview", json!({ "uri": uri.as_str(), "name": "two" })).response_result.err().unwrap();
    let line = text[..text.find("from 1").unwrap()].matches('\n').count() + 1;
    assert!(error.message.contains(&format!("two:{line}:14")), "{}", error.message);

    client.stop();
}

#[test]
fn patches_are_toggled_by_name() {
    let (_directory, project) = operation();
    let client = Client::start();
    let text = several();
    let uri = client.open(&project.join("patches/rename.kdl"), &text);
    let toggle = |arguments: Value| {
        client.connection.sender.send(lsp_server::Request::new(900.into(), "workspace/executeCommand".to_string(), json!({
            "command": "codeshaper.toggle", "arguments": arguments
        })).into()).unwrap();

        loop {
            match client.receive() {
                Message::Request(request) if request.method == "workspace/applyEdit" => {
                    return Some(request.params["edit"]["changes"][uri.as_str()][0].clone())
                }
                Message::Response(response) if response.id == 900.into() && response.response_result.is_err() => return None,
                _ => {}
            }
        }
    };

    let edit = toggle(json!([uri.as_str(), "two"])).unwrap();
    assert_eq!((edit["newText"].as_str(), edit["range"]["start"]["line"].as_u64()), (Some("false"), Some(16)));
    let edit = toggle(json!([uri.as_str(), "é"])).unwrap();
    assert_eq!((edit["newText"].as_str(), edit["range"]["start"]["line"].as_u64()), (Some("true"), Some(2)));
    assert_eq!(toggle(json!([uri.as_str()])), None);
    assert_eq!(toggle(json!([uri.as_str(), "three"])), None);

    client.stop();
}

#[test]
fn actions_on_one_line_keep_the_block() {
    let (_directory, project) = operation();
    let mut client = Client::start();
    let text = "patch \"é\" { alias \"cpp\"; }\npatch \"two\" { enabled true; alias \"cpp\"; file \"*\"; actions { }; }\n";
    let uri = client.open(&project.join("patches/rename.kdl"), text);
    let found = ask(&mut client, "textDocument/codeAction", json!({
        "textDocument": { "uri": uri.as_str() }, "context": { "diagnostics": [] },
        "range": { "start": { "line": 0, "character": 0 }, "end": { "line": 1, "character": 0 } }
    }));
    let edits: Vec<(String, String)> = found.as_array().unwrap().iter()
        .map(|action| (action["title"].as_str().unwrap().to_string(), marked(text, &action["edit"]["changes"][uri.as_str()][0]["range"])))
        .collect();
    let pair = |title: &str, removed: &str| (title.to_string(), removed.to_string());

    assert_eq!(edits, vec![pair("Remove alias", "alias \"cpp\";"), pair("Disable patch", "true"), pair("Remove alias", "alias \"cpp\";")]);

    client.stop();
}

#[test]
fn hints_count_for_their_patch() {
    let (_directory, project) = operation();
    let mut client = Client::start_with(lsp_types::InitializeParams {
        initialization_options: Some(json!({ "inlayHints": { "groups": true, "matches": true } })),
        ..Default::default()
    });
    let text = several();
    let uri = client.open(&project.join("patches/rename.kdl"), &text);
    wait(&mut client, &uri);

    let found = ask(&mut client, "textDocument/inlayHint", json!({
        "textDocument": { "uri": uri.as_str() },
        "range": { "start": { "line": 0, "character": 0 }, "end": { "line": 99, "character": 0 } }
    }));
    let shown: Vec<(u64, String)> = found.as_array().unwrap().iter()
        .map(|hint| (hint["position"]["line"].as_u64().unwrap(), hint["label"].as_str().unwrap().to_string()))
        .collect();

    assert_eq!(shown.iter().map(|(line, label)| (*line, label.as_str())).collect::<Vec<_>>(), vec![
        (8, "2 places in 2 files"), (9, "1 match in 1 of 2"), (22, "1 call"),
        (25, "1 place in 1 file"), (26, "0:"), (26, "1 match in 1 of 1")
    ]);

    client.stop();
}

#[test]
fn coverage_is_found_for_each_patch() {
    let (_directory, project) = operation();
    let text = block("one", RENAME) + &block("é", UNCOVERED);

    for code in ["never-called", "case-unused", "location-unmatched", "from-unmatched"] {
        let client = Client::start();
        let uri = client.open(&project.join("patches/rename.kdl"), &text);
        published(&client, &uri, code, true);
        client.stop();
    }
}

#[test]
fn exploring_starts_from_one_patch_or_all() {
    let (directory, project) = operation();
    fs::write(directory.path().join("target/c.txt"), "text\n").unwrap();
    let text = block("first", &CALLED.replace("a.cpp", "b.cpp")) + &block("é", CALLED);
    let mut client = Client::start();
    let uri = client.open(&project.join("patches/rename.kdl"), &text);
    let mut files = |patch: Option<&str>| {
        let found = client.request("codeshaper/explore", json!({ "uri": uri.as_str(), "patch": patch })).response_result;

        found.map(|found| found["files"].as_array().unwrap().iter().map(|file| file["path"].as_str().unwrap().to_string()).collect::<Vec<_>>())
    };

    assert_eq!(files(Some("first")).unwrap(), vec!["b.cpp"]);
    assert_eq!(files(Some("é")).unwrap(), vec!["a.cpp"]);
    assert_eq!(files(None).unwrap(), vec!["a.cpp", "b.cpp"]);
    assert!(files(Some("gone")).err().unwrap().message.contains("no patch named 'gone'"));

    client.stop();
}

#[test]
fn uses_name_their_patch() {
    let (_directory, project) = operation();
    let text = block("first", RENAME) + &block("é", CALLED);
    let mut client = Client::start();
    let uri = client.open(&project.join("patches/rename.kdl"), &text);
    let code = "void old() {}\n";

    let found = ask(&mut client, "codeshaper/exploreBlock", json!({
        "uri": uri.as_str(), "file": { "path": "a.cpp", "language": "cpp", "usual": "cpp" }, "text": code,
        "range": { "start": { "line": 0, "character": 5 }, "end": { "line": 0, "character": 8 } }
    }));
    let used: Vec<(&str, &str, bool, String)> = found["used"].as_array().unwrap().iter()
        .map(|used| (used["patch"].as_str().unwrap(), used["action"].as_str().unwrap(), used["enabled"].as_bool().unwrap(), marked(&text, &used["range"])))
        .collect();

    assert_eq!(used, vec![
        ("é", "replacer #1", true, "\"FunctionDeclarator.Declarator\"".to_string()),
        ("first", "replacer #1", false, "\"FunctionDefinition\"".to_string())
    ]);

    client.stop();
}

#[test]
fn half_typed_blocks_are_survived() {
    let (_directory, project) = operation();
    let mut client = Client::start();
    let text = several();

    for end in (0..=text.len()).filter(|end| text.is_char_boundary(*end)).step_by(7) {
        let uri = client.open(&project.join("patches/rename.kdl"), &text[..end]);
        let position = LineIndex::new(&text[..end]).position(end);
        let here = json!({ "textDocument": { "uri": uri.as_str() }, "position": position });

        for method in ["textDocument/completion", "textDocument/definition", "textDocument/documentHighlight", "textDocument/prepareRename", "textDocument/documentSymbol", "textDocument/codeLens", "textDocument/foldingRange"] {
            assert!(client.request(method, here.clone()).response_result.is_ok(), "{method} at {end}");
        }
    }

    client.stop();
}

#[test]
fn replacers_are_named_in_their_patch() {
    let (_directory, project) = operation();
    let named = |content: &str| content.replace("    replacer {", "    replacer \"same\" {");
    let text = block("first", &named(RENAME)) + &block("é", &named(CALLED)) + &block("plain", RENAME);
    let mut client = Client::start();
    let uri = client.open(&project.join("patches/rename.kdl"), &text);

    let counted: Vec<(String, Vec<String>)> = lenses(&mut client, &uri).into_iter()
        .filter(|(title, _, _)| title.contains("replacement"))
        .map(|(title, _, arguments)| (title, arguments))
        .collect();
    assert_eq!(counted, ["first", "é", "plain"].map(|name| ("1 replacement(s) in 1 file(s)".to_string(), vec![name.to_string()])));

    let found = ask(&mut client, "codeshaper/exploreBlock", json!({
        "uri": uri.as_str(), "file": { "path": "a.cpp", "language": "cpp", "usual": "cpp" }, "text": "void old() {}\n",
        "range": { "start": { "line": 0, "character": 5 }, "end": { "line": 0, "character": 8 } }
    }));
    let used: Vec<(&str, &str)> = found["used"].as_array().unwrap().iter()
        .map(|used| (used["patch"].as_str().unwrap(), used["action"].as_str().unwrap()))
        .collect();
    assert_eq!(used, vec![("é", "same"), ("first", "same"), ("plain", "replacer #1")]);

    let renamed = ask(&mut client, "textDocument/rename", {
        let mut params = at(&uri, &text, "\"same\"", 2);
        params["newName"] = json!("other");
        params
    });
    let edits = renamed["changes"][uri.as_str()].as_array().unwrap();
    assert_eq!(edits.len(), 1);
    assert_eq!(edits[0]["range"]["start"], place(&text, "\"same\"", 1));

    client.stop();
}
