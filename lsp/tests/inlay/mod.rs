// Standard Uses
use std::sync::Arc;

// Crate Uses
use crate::preview::{operation, RENAME};
use crate::protocol::Client;

// External Uses
use codeshaper::core::shaping::operation::LocationMatches;
use codeshaper::core::shaping::patch::trace::{ActionTrace, Trace};
use codeshaper_lsp::inlay::{inlay_hints, Counts, Hints, Place};
use codeshaper_lsp::position::LineIndex;
use lsp_types::{InitializeParams, InlayHintLabel, InlayHintWorkspaceClientCapabilities, WorkspaceClientCapabilities};
use lsp_server::{Message, Response};
use serde_json::json;


const GROUPS: Hints = Hints { groups: true, matches: false };
const MATCHES: Hints = Hints { groups: false, matches: true };

const PATCH: &str = r##"alias "cpp"
file "*.cpp"
actions {
    builder "b" {
        location "Class"
        match "é\\w(\\d)"
        build "x"
    }
    replacer {
        location "Function"
        from r#"(a)(?:b)
(c)"#
        to "x"
    }
}
"##;

fn marked(hints: Hints, counts: &Counts) -> String {
    traced(PATCH, hints, counts, &[])
}

fn traced(patch: &str, hints: Hints, counts: &Counts, traces: &[Option<Arc<Trace>>]) -> String {
    let index = LineIndex::new(patch);
    let mut text = patch.to_string();

    for hint in inlay_hints(patch, hints, counts, traces).into_iter().rev() {
        let InlayHintLabel::String(label) = hint.label else { panic!("expected a string") };
        text.insert_str(index.offset(hint.position), &format!("‹{label}›"));
    }

    text
}


#[test]
fn group_numbers_come_before_their_group() {
    let text = marked(GROUPS, &Counts::default());

    assert!(text.contains("match \"é\\\\w‹0:›(\\\\d)\""), "{text}");
    assert!(text.contains("from r#\"‹0:›(a)(?:b)\n‹1:›(c)\"#"), "{text}");
}

#[test]
fn counts_follow_their_location() {
    let place = |location: &str, alias: &str| Place { alias: alias.to_string(), file: "*.cpp".to_string(), location: location.to_string() };
    let counts = Counts::from([
        (place("Function", "cpp"), LocationMatches { files: 4, matched_files: 1, places: 3 }),
        (place("Class", "gdscript"), LocationMatches { files: 4, matched_files: 2, places: 1 }),
    ]);
    let text = marked(MATCHES, &counts);

    assert!(text.contains("location \"Function\"‹3 places in 1 file›\n"), "{text}");
    assert!(text.contains("location \"Class\"\n"), "{text}");
}

#[test]
fn hints_are_off_by_default() {
    assert_eq!(marked(Hints::default(), &Counts::default()), PATCH);
}

const ACTIONS: &str = r##"enabled true
file "*.cpp"
actions {
    builder "b" {
        location "Function"
        match r#"(a)
(b)"#
        build "x
y"
        actions {
            maker "inner" { make "z"; }
        }
    }
    maker "m" {
        prepare "(p)"
        make "x"
    }
    switch "s" {
        cases a="b"
        index "@[0]"
    }
    list "l" {
        values "a" "b"
        index "@[1]"
    }
    replacer {
        location "Function"
        from "(a)"
        to "#[b]"
    }
    replacer {
        location "Class"
        from "c"
        to "d"
    }
}
"##;

fn run(actions: &[(&str, ActionTrace)]) -> Option<Arc<Trace>> {
    let actions = actions.iter().map(|(path, action)| (path.to_string(), action.clone())).collect();

    Some(Arc::new(Trace { limit: 3, files: 2, actions, ..Trace::default() }))
}

fn shown(traces: &[Option<Arc<Trace>>]) -> String {
    traced(ACTIONS, MATCHES, &Counts::default(), traces)
}

#[test]
fn results_follow_their_fields() {
    let text = shown(&[run(&[
        ("b", ActionTrace { inputs: 4, matched: 3, built: 3, calls: 2, ..ActionTrace::default() }),
        ("b/inner", ActionTrace { calls: 1, ..ActionTrace::default() }),
        ("m", ActionTrace { inputs: 3, matched: 2, calls: 3, ..ActionTrace::default() }),
        ("s", ActionTrace { calls: 1, ..ActionTrace::default() }),
        ("l", ActionTrace { calls: 5, ..ActionTrace::default() }),
        ("replacer #1", ActionTrace { inputs: 2, matched: 1, matches: 1, ..ActionTrace::default() }),
        ("replacer #2", ActionTrace { inputs: 4, matched: 2, matches: 6, ..ActionTrace::default() })
    ])]);

    for line in [
        "(b)\"#‹3 of 4 match›\n", "y\"‹3 built in 2 calls›\n", "make \"z\"‹1 call›; }", "prepare \"(p)\"‹2 of 3 match›\n", "make \"x\"‹3 calls›\n",
        "index \"@[0]\"‹1 call›\n", "index \"@[1]\"‹5 calls›\n", "from \"(a)\"‹1 match in 1 of 2›\n", "from \"c\"‹6 matches in 2 of 4›\n",
        "to \"#[b]\"\n", "to \"d\"\n", "values \"a\" \"b\"\n", "cases a=\"b\"\n"
    ] {
        assert!(text.contains(line), "{line}\n{text}");
    }
}

#[test]
fn results_follow_mixed_actions() {
    let patch = "file \"*\"\nactions {\n    replacer {\n        from \"a\"\n        actions {\n            maker \"m\" { make \"z\"; }\n        }\n    }\n    \
        maker \"n\" {\n        make \"y\"\n    }\n    replacer {\n        from \"b\"\n    }\n}\n";
    let matches = |matches: usize| ActionTrace { inputs: 9, matched: 1, matches, ..ActionTrace::default() };
    let calls = |calls: usize| ActionTrace { calls, ..ActionTrace::default() };
    let text = traced(patch, MATCHES, &Counts::default(), &[run(&[
        ("replacer #1", matches(1)), ("replacer #2", matches(2)), ("replacer #1/m", calls(3)), ("n", calls(4))
    ])]);

    for line in ["from \"a\"‹1 match in 1 of 9›\n", "from \"b\"‹2 matches in 1 of 9›\n", "make \"z\"‹3 calls›; }", "make \"y\"‹4 calls›\n"] {
        assert!(text.contains(line), "{line}\n{text}");
    }
}

#[test]
fn unused_actions_say_so() {
    let text = shown(&[run(&[("m", ActionTrace { inputs: 2, ..ActionTrace::default() }), ("b", ActionTrace { inputs: 1, matched: 1, ..ActionTrace::default() })])]);
    assert!(text.contains("prepare \"(p)\"‹0 of 2 match›\n"), "{text}");
    assert!(text.contains("make \"x\"‹not called›\n"), "{text}");
    assert!(text.contains("y\"‹0 built›\n"), "{text}");
    assert!(text.contains("from \"(a)\"\n"), "{text}");

    let text = shown(&[run(&[])]);
    assert!(text.contains("(b)\"#\n"), "{text}");
    assert!(text.contains("y\"‹never built›\n"), "{text}");
    assert!(text.contains("index \"@[0]\"‹not called›\n"), "{text}");
}

#[test]
fn results_need_a_traced_file() {
    let nowhere = Some(Arc::new(Trace::new(3)));

    for traces in [vec![], vec![None], vec![nowhere]] {
        assert_eq!(shown(&traces), ACTIONS);
    }
    assert_eq!(traced(ACTIONS, GROUPS, &Counts::default(), &[run(&[])]), traced(ACTIONS, GROUPS, &Counts::default(), &[]));
}

#[test]
fn results_belong_to_their_block() {
    let text = format!("patch \"é\" {{\n{ACTIONS}}}\npatch \"two\" {{\n{ACTIONS}}}\n");
    let calls = |calls: usize| run(&[("m", ActionTrace { calls, ..ActionTrace::default() })]);
    let found = traced(&text, MATCHES, &Counts::default(), &[calls(7), calls(8)]);

    let second = found.find("patch \"two\"").unwrap();
    assert!(found[..second].contains("make \"x\"‹7 calls›") && found[second..].contains("make \"x\"‹8 calls›"), "{found}");
    assert!(!traced(&text, MATCHES, &Counts::default(), &[None, calls(8)])[..second].contains("‹"));
}

#[test]
fn counts_survive_edits_elsewhere() {
    let (_directory, project) = operation();
    let mut params = InitializeParams { initialization_options: Some(json!({ "inlayHints": { "matches": true } })), ..InitializeParams::default() };
    params.capabilities.workspace = Some(WorkspaceClientCapabilities {
        inlay_hint: Some(InlayHintWorkspaceClientCapabilities { refresh_support: Some(true) }),
        ..WorkspaceClientCapabilities::default()
    });
    let mut client = Client::start_with(params);
    let uri = client.open(&project.join("patches/rename.kdl"), RENAME);
    assert_eq!(client.published(), Vec::<String>::new());

    let Message::Request(refresh) = client.receive() else { panic!("expected a refresh request") };
    assert_eq!(refresh.method, "workspace/inlayHint/refresh");
    client.connection.sender.send(Response::new_ok(refresh.id, ()).into()).unwrap();

    let mut labels = |text: Option<String>| {
        if let Some(text) = text {
            client.notify("textDocument/didChange", json!({
                "textDocument": { "uri": uri.as_str(), "version": 2 }, "contentChanges": [{ "text": text }]
            }));
        }
        let range = json!({ "start": { "line": 0, "character": 0 }, "end": { "line": 99, "character": 0 } });
        let found = client.request("textDocument/inlayHint", json!({ "textDocument": { "uri": uri.as_str() }, "range": range }));

        found.response_result.unwrap().as_array().unwrap().iter().map(|hint| hint["label"].as_str().unwrap().to_string()).collect::<Vec<_>>()
    };

    assert_eq!(labels(None), vec!["2 places in 2 files", "1 match in 1 of 2"]);
    assert_eq!(labels(Some(RENAME.replace("\"new\"", "\"newer\""))), vec!["2 places in 2 files"]);
    assert_eq!(labels(Some(RENAME.replace("FunctionDefinition", "FunctionDefinitio"))), Vec::<String>::new());

    client.notify("textDocument/didSave", json!({ "textDocument": { "uri": uri.as_str() }, "text": RENAME }));
    client.request("textDocument/hover", json!({ "textDocument": { "uri": uri.as_str() }, "position": { "line": 0, "character": 0 } }));
    let range = json!({ "start": { "line": 0, "character": 0 }, "end": { "line": 99, "character": 0 } });
    let found = client.request("textDocument/inlayHint", json!({ "textDocument": { "uri": uri.as_str() }, "range": range })).response_result.unwrap();
    assert_eq!(found.as_array().unwrap().len(), 2);

    client.stop();
}
