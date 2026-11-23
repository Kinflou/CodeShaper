// Standard Uses
use std::fs;
use std::path::Path;
use std::thread::{self, JoinHandle};
use std::time::Duration;

// Crate Uses
use crate::diagnostics::UNCOVERED;
use crate::preview::{operation, RENAME};
use crate::{project, BROKEN};

// External Uses
use codeshaper::core::registry::Registry;
use codeshaper_lsp::position::LineIndex;
use lsp_server::{Connection, Message, Notification, Request, RequestId, Response};
use lsp_types::notification::{DidChangeTextDocument, DidOpenTextDocument, DidSaveTextDocument, Exit, Initialized, Notification as _, PublishDiagnostics};
use lsp_types::request::{CodeLensRefresh, CodeLensRequest, PrepareRenameRequest, Rename, Completion, GotoDefinition, HoverRequest, Initialize, References, Request as _, SemanticTokensFullRequest, Shutdown};
use lsp_types::{
    GotoCapability, LocationLink, TextDocumentClientCapabilities,
    CodeLens, CodeLensParams, PrepareRenameResponse, RenameParams, WorkspaceEdit, CodeLensWorkspaceClientCapabilities, WorkspaceClientCapabilities, CompletionParams, GotoDefinitionParams, GotoDefinitionResponse, Hover, HoverContents, HoverParams, Location, NumberOrString, ReferenceContext, ReferenceParams, CompletionResponse, DidOpenTextDocumentParams, InitializeParams, InitializedParams, Position,
    DidChangeTextDocumentParams, DidSaveTextDocumentParams, PublishDiagnosticsParams, SemanticTokensParams,
    TextDocumentContentChangeEvent, VersionedTextDocumentIdentifier, SemanticTokensResult, TextDocumentIdentifier, TextDocumentItem,
    TextDocumentPositionParams, Uri
};
use url::Url;


macro_rules! to_json {
    ($value:expr) => { serde_json::to_value($value).unwrap() };
}


pub struct Client {
    pub connection: Connection,
    server: Option<JoinHandle<Result<(), String>>>,
    next: i32
}

impl Client {
    pub fn start() -> Self {
        Self::start_with(InitializeParams::default())
    }

    pub fn start_with(params: InitializeParams) -> Self {
        Self::start_on(params, codeshaper::registry)
    }

    pub fn start_on(params: InitializeParams, registry: impl Fn() -> Registry + Send + 'static) -> Self {
        let (server, connection) = Connection::memory();
        let handle = thread::spawn(move || codeshaper_lsp::serve_with(&server, registry).map_err(|error| error.to_string()));
        let mut client = Self { connection, server: Some(handle), next: 0 };

        client.request(Initialize::METHOD, to_json!(params));
        client.notify(Initialized::METHOD, to_json!(InitializedParams {}));
        client
    }

    pub fn open(&self, path: &Path, text: &str) -> Uri {
        let uri: Uri = Url::from_file_path(path).unwrap().as_str().parse().unwrap();
        let document = TextDocumentItem::new(uri.clone(), "kdl".to_string(), 1, text.to_string());

        self.notify(DidOpenTextDocument::METHOD, to_json!(DidOpenTextDocumentParams { text_document: document }));
        uri
    }

    pub fn request(&mut self, method: &str, params: serde_json::Value) -> Response {
        self.next += 1;
        let id = RequestId::from(self.next);
        self.connection.sender.send(Request::new(id.clone(), method.to_string(), params).into()).unwrap();

        loop {
            if let Message::Response(response) = self.receive() {
                if response.id == id { return response }
            }
        }
    }

    pub fn notify(&self, method: &str, params: serde_json::Value) {
        self.connection.sender.send(Notification::new(method.to_string(), params).into()).unwrap();
    }

    pub fn receive(&self) -> Message {
        self.connection.receiver.recv_timeout(Duration::from_secs(10)).unwrap()
    }

    pub fn published(&self) -> Vec<String> {
        let Message::Notification(published) = self.receive() else { panic!("expected a notification") };
        let params: PublishDiagnosticsParams = serde_json::from_value(published.params).unwrap();

        params.diagnostics.into_iter()
            .filter_map(|diagnostic| match diagnostic.code {
                Some(NumberOrString::String(code)) => Some(code),
                _ => None
            })
            .collect()
    }

    pub fn stop(mut self) {
        self.request(Shutdown::METHOD, serde_json::Value::Null);
        self.notify(Exit::METHOD, serde_json::Value::Null);

        assert_eq!(self.server.take().unwrap().join().unwrap(), Ok(()));
    }
}

pub fn at(uri: &Uri, position: Position) -> TextDocumentPositionParams {
    TextDocumentPositionParams::new(TextDocumentIdentifier::new(uri.clone()), position)
}


#[test]
fn diagnostics_are_published_on_open() {
    let (_directory, project) = project(&[("broken.kdl", BROKEN)]);
    let client = Client::start();

    let uri = client.open(&project.join("patches/broken.kdl"), BROKEN);

    let Message::Notification(published) = client.receive() else { panic!("expected a notification") };
    assert_eq!(published.method, PublishDiagnostics::METHOD);
    let params: PublishDiagnosticsParams = serde_json::from_value(published.params).unwrap();
    assert_eq!(params.uri, uri);
    assert_eq!(params.diagnostics.len(), 3);

    client.stop();
}

#[test]
fn coverage_is_published_on_open_and_save_only() {
    let (_directory, project) = operation();
    let client = Client::start();
    let uri = client.open(&project.join("patches/uncovered.kdl"), UNCOVERED);
    let coverage = ["never-called", "case-unused", "location-unmatched", "from-unmatched"];

    assert_eq!(client.published(), Vec::<String>::new());
    assert_eq!(client.published(), coverage);

    let change = DidChangeTextDocumentParams {
        text_document: VersionedTextDocumentIdentifier::new(uri.clone(), 2),
        content_changes: vec![TextDocumentContentChangeEvent { range: None, range_length: None, text: UNCOVERED.to_string() }]
    };
    client.notify(DidChangeTextDocument::METHOD, to_json!(change));
    assert_eq!(client.published(), Vec::<String>::new());

    let save = DidSaveTextDocumentParams { text_document: TextDocumentIdentifier::new(uri), text: None };
    client.notify(DidSaveTextDocument::METHOD, to_json!(save));
    assert_eq!(client.published(), Vec::<String>::new());
    assert_eq!(client.published(), coverage);

    client.stop();
}

#[test]
fn lenses_are_refreshed_after_tracing() {
    let (_directory, project) = operation();
    let mut params = InitializeParams::default();
    params.capabilities.workspace = Some(WorkspaceClientCapabilities {
        code_lens: Some(CodeLensWorkspaceClientCapabilities { refresh_support: Some(true) }),
        ..WorkspaceClientCapabilities::default()
    });
    let client = Client::start_with(params);

    client.open(&project.join("patches/rename.kdl"), RENAME);
    assert_eq!(client.published(), Vec::<String>::new());

    let Message::Request(refresh) = client.receive() else { panic!("expected a refresh request") };
    assert_eq!(refresh.method, CodeLensRefresh::METHOD);
    client.connection.sender.send(Response::new_ok(refresh.id, ()).into()).unwrap();

    client.stop();
}

#[test]
fn completion_offers_fields() {
    let (_directory, project) = project(&[("partial.kdl", "")]);
    let mut client = Client::start();
    let uri = client.open(&project.join("patches/partial.kdl"), "actions {\n    makers {\n        maker \"m\" {\n            ma\n");

    let params = CompletionParams {
        text_document_position: at(&uri, Position::new(3, 14)),
        work_done_progress_params: Default::default(),
        partial_result_params: Default::default(),
        context: None
    };
    let response = client.request(Completion::METHOD, to_json!(params));
    let Some(CompletionResponse::Array(items)) = serde_json::from_value(response.response_result.unwrap()).unwrap() else {
        panic!("expected completion items")
    };

    assert_eq!(items.iter().map(|item| item.label.as_str()).collect::<Vec<_>>(), vec!["prepare", "flags", "make"]);

    client.stop();
}

#[test]
fn semantic_tokens_for_expressions() {
    let (_directory, project) = project(&[("tokens.kdl", "")]);
    let mut client = Client::start();
    let uri = client.open(&project.join("patches/tokens.kdl"), "to \"$[0] #[names]\"\n");

    let params = SemanticTokensParams {
        text_document: TextDocumentIdentifier::new(uri),
        work_done_progress_params: Default::default(),
        partial_result_params: Default::default()
    };
    let response = client.request(SemanticTokensFullRequest::METHOD, to_json!(params));
    let Some(SemanticTokensResult::Tokens(tokens)) = serde_json::from_value(response.response_result.unwrap()).unwrap() else {
        panic!("expected semantic tokens")
    };

    assert_eq!(tokens.data.iter().map(|token| (token.delta_start, token.length, token.token_type)).collect::<Vec<_>>(), vec![
        (4, 4, 1), (5, 8, 0)
    ]);

    client.stop();
}

#[test]
fn definition_and_references_of_actions() {
    let text = "actions {\n    makers {\n        maker \"pair\" {\n            make \"x\"\n        }\n    }\n    replacers {\n        replacer {\n            to \"#[pair] #[pair]\"\n        }\n    }\n}\n";
    let (_directory, project) = project(&[("navigation.kdl", "")]);
    let mut client = Client::start();
    let uri = client.open(&project.join("patches/navigation.kdl"), text);

    let params = GotoDefinitionParams {
        text_document_position_params: at(&uri, Position::new(8, 19)),
        work_done_progress_params: Default::default(),
        partial_result_params: Default::default()
    };
    let response = client.request(GotoDefinition::METHOD, to_json!(params));
    let Some(GotoDefinitionResponse::Array(locations)) = serde_json::from_value(response.response_result.unwrap()).unwrap() else {
        panic!("expected a definition")
    };
    let location = &locations[0];
    assert_eq!((location.range.start, location.range.end), (Position::new(2, 15), Position::new(2, 19)));

    let params = ReferenceParams {
        text_document_position: at(&uri, Position::new(2, 16)),
        context: ReferenceContext { include_declaration: false },
        work_done_progress_params: Default::default(),
        partial_result_params: Default::default()
    };
    let response = client.request(References::METHOD, to_json!(params));
    let locations: Vec<Location> = serde_json::from_value(response.response_result.unwrap()).unwrap();
    assert_eq!(locations.iter().map(|location| location.range.start).collect::<Vec<_>>(), vec![
        Position::new(8, 18), Position::new(8, 26)
    ]);

    client.stop();
}

#[test]
fn rename_edits_every_use() {
    let text = "actions {\n    makers {\n        maker \"pair\" {\n            make \"x\"\n        }\n    }\n    replacers {\n        replacer {\n            to \"#[pair] #[pair]\"\n        }\n    }\n}\n";
    let (_directory, project) = project(&[("rename.kdl", "")]);
    let mut client = Client::start();
    let uri = client.open(&project.join("patches/rename.kdl"), text);

    let response = client.request(PrepareRenameRequest::METHOD, to_json!(at(&uri, Position::new(8, 19))));
    let Some(PrepareRenameResponse::Range(range)) = serde_json::from_value(response.response_result.unwrap()).unwrap() else {
        panic!("expected a rename range")
    };
    assert_eq!((range.start, range.end), (Position::new(8, 18), Position::new(8, 22)));

    let params = RenameParams {
        text_document_position: at(&uri, Position::new(2, 16)),
        new_name: "couple".to_string(),
        work_done_progress_params: Default::default()
    };
    let response = client.request(Rename::METHOD, to_json!(params));
    let edit: WorkspaceEdit = serde_json::from_value(response.response_result.unwrap()).unwrap();
    let edits = &edit.changes.unwrap()[&uri];

    assert_eq!(edits.len(), 3);
    assert!(edits.iter().all(|edit| edit.new_text == "couple"));

    client.stop();
}

#[test]
fn hover_describes_locations() {
    let text = "alias \"\"\nactions {\n    replacers {\n        replacer {\n            location \"FunctionDefinition\"\n        }\n    }\n}\n";
    let (_directory, project) = project(&[("hover.kdl", "")]);
    let mut client = Client::start();
    let uri = client.open(&project.join("patches/hover.kdl"), text);

    let params = HoverParams {
        text_document_position_params: at(&uri, Position::new(4, 24)),
        work_done_progress_params: Default::default()
    };
    let response = client.request(HoverRequest::METHOD, to_json!(params));
    let Some(Hover { contents: HoverContents::Markup(markup), range }) = serde_json::from_value(response.response_result.unwrap()).unwrap() else {
        panic!("expected a hover")
    };

    assert_eq!(markup.value, "cpp: node kind `function_definition`");
    assert_eq!(range.map(|range| (range.start, range.end)), Some((Position::new(4, 21), Position::new(4, 41))));

    client.stop();
}

#[test]
fn preview_lens_and_request() {
    let (_directory, project) = operation();
    let mut client = Client::start();
    let uri = client.open(&project.join("patches/rename.kdl"), RENAME);

    client.request(HoverRequest::METHOD, to_json!(HoverParams {
        text_document_position_params: at(&uri, Position::new(0, 0)),
        work_done_progress_params: Default::default()
    }));

    let params = CodeLensParams {
        text_document: TextDocumentIdentifier::new(uri.clone()),
        work_done_progress_params: Default::default(),
        partial_result_params: Default::default()
    };
    let response = client.request(CodeLensRequest::METHOD, to_json!(params));
    let lenses: Vec<CodeLens> = serde_json::from_value(response.response_result.unwrap()).unwrap();
    let command = lenses[0].command.clone().unwrap();
    assert_eq!((command.title.as_str(), command.command.as_str()), ("Preview result", "codeshaper.preview"));
    assert_eq!(command.arguments, Some(vec![serde_json::json!(uri.as_str())]));

    let operation = lenses[1].command.clone().unwrap();
    assert_eq!((operation.title.as_str(), operation.command.as_str()), ("Preview operation", "codeshaper.previewOperation"));

    let toggle = lenses[2].command.clone().unwrap();
    assert_eq!((toggle.title.as_str(), toggle.command.as_str()), ("Enable patch", "codeshaper.toggle"));

    let counted = lenses[3].command.clone().unwrap();
    assert_eq!(counted.title, "1 replacement(s) in 1 file(s)");
    assert_eq!(lenses[3].range.start, Position::new(6, 8));

    let response = client.request("codeshaper/preview", serde_json::json!({ "uri": uri.as_str() }));
    let result = response.response_result.unwrap();
    assert_eq!(result["matched"], 2);
    assert_eq!(result["files"].as_array().unwrap().len(), 1);
    assert_eq!(result["files"][0]["path"], "a.cpp");
    assert_eq!(result["files"][0]["result"], "void new() {}\n");
    assert!(result["files"][0]["uri"].as_str().unwrap().ends_with("/target/a.cpp"));

    let response = client.request("codeshaper/preview", serde_json::json!({ "uri": "file:///nowhere/rename.kdl" }));
    assert!(response.response_result.is_err());

    client.stop();
}

#[test]
fn file_and_location_lead_to_the_target() {
    let (_directory, project) = operation();
    let mut client = Client::start();
    let uri = client.open(&project.join("patches/rename.kdl"), RENAME);

    let params = GotoDefinitionParams {
        text_document_position_params: at(&uri, Position::new(2, 8)),
        work_done_progress_params: Default::default(),
        partial_result_params: Default::default()
    };
    let response = client.request(GotoDefinition::METHOD, to_json!(params));
    let files: Vec<Location> = serde_json::from_value(response.response_result.unwrap()).unwrap();
    assert_eq!(files.len(), 2);
    assert!(files[0].uri.as_str().ends_with("/target/a.cpp"));

    let params = ReferenceParams {
        text_document_position: at(&uri, Position::new(7, 25)),
        context: ReferenceContext { include_declaration: false },
        work_done_progress_params: Default::default(),
        partial_result_params: Default::default()
    };
    let response = client.request(References::METHOD, to_json!(params));
    let places: Vec<Location> = serde_json::from_value(response.response_result.unwrap()).unwrap();
    assert_eq!(places.iter().map(|place| (place.range.start, place.range.end)).collect::<Vec<_>>(), vec![
        (Position::new(0, 0), Position::new(0, 13)), (Position::new(0, 0), Position::new(0, 14))
    ]);

    client.stop();
}

#[test]
fn patches_are_toggled() {
    let (_directory, project) = operation();
    let mut client = Client::start();
    let uri = client.open(&project.join("patches/rename.kdl"), RENAME);
    assert_eq!(client.published(), Vec::<String>::new());

    let params = serde_json::json!({
        "textDocument": { "uri": uri.as_str() },
        "range": { "start": { "line": 0, "character": 2 }, "end": { "line": 0, "character": 2 } },
        "context": { "diagnostics": [] }
    });
    let actions = client.request("textDocument/codeAction", params).response_result.unwrap();
    assert_eq!(actions[0]["title"], "Enable patch");
    let edit = &actions[0]["edit"]["changes"][uri.as_str()][0];
    assert_eq!((&edit["newText"], &edit["range"]["start"]["character"]), (&serde_json::json!("true"), &serde_json::json!(8)));

    let params = serde_json::json!({ "command": "codeshaper.toggle", "arguments": [uri.as_str()] });
    client.connection.sender.send(Request::new(RequestId::from(90), "workspace/executeCommand".to_string(), params).into()).unwrap();
    let Message::Request(apply) = client.receive() else { panic!("expected an edit") };
    assert_eq!(apply.method, "workspace/applyEdit");
    assert_eq!(apply.params["edit"]["changes"][uri.as_str()][0]["newText"], "true");
    let Message::Response(done) = client.receive() else { panic!("expected a response") };
    assert_eq!(done.id, RequestId::from(90));

    client.stop();
}

#[test]
fn groups_are_highlighted_and_reached() {
    let text = "actions {\n    replacers {\n        replacer {\n            from r\"(a)(b)\"\n            to \"$[1] $[0] $[1]\"\n        }\n    }\n}\n";
    let (_directory, project) = project(&[("groups.kdl", "")]);
    let mut client = Client::start();
    let uri = client.open(&project.join("patches/groups.kdl"), text);

    let place = serde_json::json!({ "textDocument": { "uri": uri.as_str() }, "position": { "line": 4, "character": 18 } });
    let found = client.request("textDocument/documentHighlight", place.clone()).response_result.unwrap();
    let starts: Vec<(u64, u64)> = found.as_array().unwrap().iter()
        .map(|highlight| &highlight["range"]["start"])
        .map(|start| (start["line"].as_u64().unwrap(), start["character"].as_u64().unwrap()))
        .collect();
    assert_eq!(starts, vec![(3, 22), (4, 16), (4, 26)]);

    let definition = client.request("textDocument/definition", place).response_result.unwrap();
    assert_eq!(definition[0]["range"]["start"], serde_json::json!({ "line": 3, "character": 22 }));

    client.stop();
}

#[test]
fn definition_links_cover_the_whole_text() {
    let text = RENAME
        .replace("from r\"old\"", "from r\"(old)\"")
        .replace("to \"new\"", "to \"$[0] #[to-upper]\"")
        .replace("    replacers {", "    makers {\n        maker \"to-upper\" {\n            make \"x\"\n        }\n    }\n    replacers {")
        .replace("*.cpp", "a.cpp");
    let (_directory, project) = operation();
    let mut params = InitializeParams::default();
    params.capabilities.text_document = Some(TextDocumentClientCapabilities {
        definition: Some(GotoCapability { link_support: Some(true), ..GotoCapability::default() }),
        ..TextDocumentClientCapabilities::default()
    });
    let mut client = Client::start_with(params);
    let uri = client.open(&project.join("patches/rename.kdl"), &text);
    let index = LineIndex::new(&text);

    for (needle, covered, target) in [("cpp\"\n\n", "a.cpp", "/a.cpp"), ("$[0]", "$[0]", "/rename.kdl"), ("upper]", "to-upper", "/rename.kdl")] {
        let position = index.position(text.find(needle).unwrap() + 2);
        let params = serde_json::json!({ "textDocument": { "uri": uri.as_str() }, "position": position });
        let response = client.request(GotoDefinition::METHOD, params);
        let links: Vec<LocationLink> = serde_json::from_value(response.response_result.unwrap()).unwrap();
        let origin = links[0].origin_selection_range.unwrap();

        assert_eq!(&text[index.offset(origin.start)..index.offset(origin.end)], covered);
        assert!(links[0].target_uri.as_str().ends_with(target));
    }

    client.stop();
}

#[test]
fn operations_are_previewed_from_unsaved_text() {
    let (directory, project) = operation();
    let mut client = Client::start();
    let broken = project.join("patches/second.kdl");
    fs::write(&broken, "enabled true\nalias \"\"\nfile \"*.cpp\"\nactions {").unwrap();

    let enabled = RENAME.replace("enabled false", "enabled true");
    let patch = client.open(&project.join("patches/rename.kdl"), &enabled);
    client.open(&broken, &enabled.replace("r\"old\"", "r\"keep\"").replace("\"new\"", "\"kept\""));
    let text = fs::read_to_string(directory.path().join("configuration.kdl")).unwrap();
    let configuration = client.open(&directory.path().join("configuration.kdl"), &text);

    for uri in [&patch, &configuration] {
        let response = client.request("codeshaper/previewOperation", serde_json::json!({ "uri": uri.as_str() }));
        let result = response.response_result.unwrap();
        let files: Vec<(&str, &str)> = result["files"].as_array().unwrap().iter()
            .map(|file| (file["path"].as_str().unwrap(), file["result"].as_str().unwrap()))
            .collect();

        assert_eq!(files, vec![("a.cpp", "void new() {}\n"), ("b.cpp", "void kept() {}\n")]);
        assert_eq!(result["errors"], serde_json::json!([]));
        assert!(result["files"][0]["uri"].as_str().unwrap().ends_with("/target/a.cpp"));
    }
    assert_eq!(fs::read_to_string(directory.path().join("target/a.cpp")).unwrap(), "void old() {}\n");

    let lenses = client.request("textDocument/codeLens", serde_json::json!({ "textDocument": { "uri": configuration.as_str() } }));
    assert_eq!(lenses.response_result.unwrap()[0]["command"]["title"], "Preview operation");

    client.stop();
}

#[test]
fn refused_operations_say_why() {
    let (directory, project) = operation();
    let mut client = Client::start();
    let text = RENAME.replace("enabled false", "enabled true").replace("\"new\"", "\"#[gone]\"");
    let uri = client.open(&project.join("patches/rename.kdl"), &text);

    let response = client.request("codeshaper/previewOperation", serde_json::json!({ "uri": uri.as_str() }));
    let message = response.response_result.unwrap_err().message;
    assert!(message.contains("No action named 'gone'"), "{message}");

    let settings = fs::read_to_string(project.join("settings.kdl")).unwrap();
    let other = client.open(&directory.path().join("shaping_project/settings.kdl"), &settings);
    let response = client.request("codeshaper/previewOperation", serde_json::json!({ "uri": other.as_str() }));
    assert_eq!(response.response_result.unwrap_err().message, "Not an open patch or configuration");

    client.stop();
}

#[test]
fn redundant_aliases_are_removed() {
    let (_directory, project) = operation();
    let text = RENAME.replace("alias \"\"", "alias \"cpp\"");
    let mut client = Client::start();
    let uri = client.open(&project.join("patches/rename.kdl"), &text);
    assert_eq!(client.published(), vec!["redundant-alias"]);

    let params = serde_json::json!({
        "textDocument": { "uri": uri.as_str() },
        "range": { "start": { "line": 1, "character": 2 }, "end": { "line": 1, "character": 2 } },
        "context": { "diagnostics": [] }
    });
    let actions = client.request("textDocument/codeAction", params).response_result.unwrap();
    assert_eq!(actions.as_array().unwrap().len(), 1);
    assert_eq!(actions[0]["title"], "Remove alias");
    let edit = &actions[0]["edit"]["changes"][uri.as_str()][0];
    assert_eq!(edit["newText"], "");
    assert_eq!(edit["range"], serde_json::json!({ "start": { "line": 1, "character": 0 }, "end": { "line": 2, "character": 0 } }));

    client.stop();
}

#[test]
fn saved_settings_recheck_open_patches() {
    let (_directory, project) = operation();
    let client = Client::start();
    client.open(&project.join("patches/rename.kdl"), RENAME);
    assert_eq!(client.published(), Vec::<String>::new());

    let path = project.join("settings.kdl");
    let text = fs::read_to_string(&path).unwrap().replace("ast-set \"cpp\"", "ast-set \"target\"");
    fs::write(&path, &text).unwrap();
    let uri = client.open(&path, &text);
    let save = DidSaveTextDocumentParams { text_document: TextDocumentIdentifier::new(uri), text: None };
    client.notify(DidSaveTextDocument::METHOD, to_json!(save));

    let mut seen = vec![];
    while !seen.contains(&"language-not-found".to_string()) { seen.extend(client.published()); }

    client.stop();
}
