// Standard Uses

// Crate Uses
use crate::preview::{operation, RENAME};
use crate::protocol::Client;

// External Uses
use codeshaper::core::shaping::operation::LocationMatches;
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
    builders {
        builder "b" {
            location "Class"
            match "é\\w(\\d)"
            build "x"
        }
    }
    replacers {
        replacer {
            location "Function"
            from r#"(a)(?:b)
(c)"#
            to "x"
        }
    }
}
"##;

fn marked(hints: Hints, counts: &Counts) -> String {
    let index = LineIndex::new(PATCH);
    let mut text = PATCH.to_string();

    for hint in inlay_hints(PATCH, hints, counts).into_iter().rev() {
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

    assert_eq!(labels(None), vec!["2 places in 2 files"]);
    assert_eq!(labels(Some(RENAME.replace("\"new\"", "\"newer\""))), vec!["2 places in 2 files"]);
    assert_eq!(labels(Some(RENAME.replace("FunctionDefinition", "FunctionDefinitio"))), Vec::<String>::new());

    client.stop();
}
