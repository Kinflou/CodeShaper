// Standard Uses
use std::fs;
use std::path::{Path, PathBuf};

// Crate Uses
use crate::preview::operation;
use crate::protocol::Client;

// External Uses
use codeshaper_lsp::completion::{completion, document};
use codeshaper_lsp::diagnostics::diagnostics;
use codeshaper_lsp::hover::hover;
use codeshaper_lsp::position::LineIndex;
use codeshaper_lsp::symbols::symbols;
use codeshaper_lsp::trace::Traces;
use lsp_server::Message;
use lsp_types::{Diagnostic, NumberOrString, Uri};
use serde_json::{json, Value};
use tempfile::TempDir;
use url::Url;


const DECLARED: &str = "constant \"header\" \"Generated\"\nvariable \"prefix\" default=\"my_\"\nvariable \"engine\"\n/-variable \"gone\"\n";

const CALLING: &str = "file \"*.cpp\"\nactions {\n    maker \"own\" {\n        make \"x\"\n    }\n    replacer {\n        location \"function_definition\"\n        from r\"void old\"\n        to \"#[prefix]#[header]#[engine]#[own]#[none]\"\n    }\n}\n";

fn append(path: &Path, more: &str) -> String {
    let text = fs::read_to_string(path).unwrap() + more;
    fs::write(path, &text).unwrap();

    text
}

fn project(more: &str, configured: &str) -> (TempDir, PathBuf) {
    let (directory, project) = operation();
    append(&project.join("project.kdl"), more);
    append(&directory.path().join("configuration.kdl"), configured);

    (directory, project)
}

fn hovered(project: &Path, name: &str) -> Option<String> {
    let offset = CALLING.find(&format!("#[{name}]")).unwrap() + 3;

    hover(&codeshaper::registry(), project, &mut Traces::default(), CALLING, offset).map(|(text, range)| {
        assert_eq!(&CALLING[range], name);
        text
    })
}

fn codes(found: &[Diagnostic]) -> Vec<(&str, &str)> {
    found.iter()
        .filter_map(|diagnostic| match &diagnostic.code {
            Some(NumberOrString::String(code)) => Some((code.as_str(), diagnostic.message.as_str())),
            _ => None
        })
        .collect()
}

fn marked(text: &str, diagnostic: &Diagnostic) -> String {
    let index = LineIndex::new(text);

    text[index.offset(diagnostic.range.start)..index.offset(diagnostic.range.end)].to_string()
}

fn at(uri: &Uri, text: &str, needle: &str, skip: usize) -> Value {
    json!({ "textDocument": { "uri": uri.as_str() }, "position": LineIndex::new(text).position(text.find(needle).unwrap() + skip) })
}

fn ask(client: &mut Client, method: &str, params: Value) -> Value {
    client.request(method, params).response_result.unwrap()
}

fn published(client: &Client, uri: &Uri, message: &str, wanted: bool) {
    loop {
        let Message::Notification(notification) = client.receive() else { continue };
        if notification.params["uri"] != uri.as_str() { continue }

        let found = notification.params["diagnostics"].as_array().unwrap().iter().any(|diagnostic| diagnostic["message"].as_str().unwrap().contains(message));
        if found == wanted { return }
    }
}


#[test]
fn hovers_tell_where_a_value_comes_from() {
    let (_directory, found) = project(DECLARED, "variables prefix=\"x_\"\n");

    assert_eq!(hovered(&found, "prefix").unwrap(), "Variable of the project, from the configuration\n```\nx_\n```");
    assert_eq!(hovered(&found, "header").unwrap(), "Constant of the project\n```\nGenerated\n```");
    assert_eq!(hovered(&found, "engine").unwrap(), "Variable of the project, without a value");
    assert_eq!(hovered(&found, "none"), None);
    assert!(hovered(&found, "own").unwrap().starts_with("Called from 1 place(s) in this patch"));

    let (_directory, found) = project(&DECLARED.replace("\"Generated\"", "\"a\\n```b\""), "");
    assert_eq!(hovered(&found, "prefix").unwrap(), "Variable of the project, its default\n```\nmy_\n```");
    assert_eq!(hovered(&found, "header").unwrap(), "Constant of the project\n````\na\n```b\n````");

    let (_directory, found) = project(DECLARED, "variables prefix=\"\"\n");
    assert_eq!(hovered(&found, "prefix").unwrap(), "Variable of the project, from the configuration: empty");

    let (_directory, found) = project("", "");
    assert_eq!(hovered(&found, "prefix"), None);
}

#[test]
fn calls_suggest_the_names() {
    let (_directory, found) = project(DECLARED, "");
    let offset = CALLING.find("#[prefix").unwrap() + 2;
    let items: Vec<(String, Option<String>)> = completion(&codeshaper::registry(), &found, CALLING, offset).into_iter()
        .map(|item| (item.label, item.detail))
        .filter(|(label, _)| !label.contains('.'))
        .collect();
    let pair = |label: &str, detail: &str| (label.to_string(), Some(detail.to_string()));

    assert_eq!(items, vec![pair("own", "maker"), pair("header", "constant"), pair("prefix", "variable"), pair("engine", "variable")]);
}

#[test]
fn patches_know_the_names() {
    let (_directory, found) = project(DECLARED, "");
    let path = found.join("patches/calling.kdl");
    let diagnosed = diagnostics(&codeshaper::registry(), &path, CALLING);

    assert_eq!(codes(&diagnosed), vec![
        ("variable-not-set", "replacer #1 › to: Variable 'engine' has no value"), ("action-not-found", "replacer #1 › to: No action named 'none'")
    ]);
    assert_eq!(marked(CALLING, &diagnosed[0]), "#[engine]");

    let clash = CALLING.replace("maker \"own\"", "maker \"header\"").replace("#[own]#[none]", "");
    let diagnosed = diagnostics(&codeshaper::registry(), &path, &clash);
    assert!(codes(&diagnosed).contains(&("duplicate-action", "header › name: The name 'header' is already a constant of the project")));

    let blocks = format!("patch \"é\" {{\n{CALLING}}}\npatch \"b\" {{\n{CALLING}}}\n");
    assert_eq!(codes(&diagnostics(&codeshaper::registry(), &path, &blocks)).len(), 4);
}

#[test]
fn configurations_are_checked() {
    let (directory, _) = project(DECLARED, "");
    let path = directory.path().join("configuration.kdl");
    let saved = fs::read_to_string(&path).unwrap();
    let diagnosed = |variables: &str| {
        let text = format!("{saved}{variables}");
        let found = diagnostics(&codeshaper::registry(), &path, &text);

        found.iter().map(|found| (codes(std::slice::from_ref(found))[0].0.to_string(), found.message.clone(), marked(&text, found))).collect::<Vec<_>>()
    };
    let one = |message: &str, marked: &str| vec![("variable-not-found".to_string(), message.to_string(), marked.to_string())];

    assert_eq!(diagnosed("variables prefix=\"a\" engine=\"é\"\n"), vec![]);
    assert_eq!(diagnosed("variables prefix=\"a\" é=\"b\"\n"), one("project.kdl declares no variable named 'é'", "é"));
    assert_eq!(diagnosed("variables \"header\"=\"b\"\n"), one("'header' is a constant, constants can't be set", "\"b\""));
    assert_eq!(diagnosed("variables header=\"b\"\r\n"), one("'header' is a constant, constants can't be set", "header"));
    assert_eq!(diagnosed("/-variables other=\"b\"\n"), vec![]);
}

#[test]
fn projects_are_checked() {
    let (_directory, found) = project("", "");
    let path = found.join("project.kdl");
    let text = fs::read_to_string(&path).unwrap() + "variable \"a b\"\n";
    let diagnosed = diagnostics(&codeshaper::registry(), &path, &text);

    assert_eq!(diagnosed.len(), 1);
    assert!(diagnosed[0].message.contains("'a b' can't be called"), "{}", diagnosed[0].message);
    assert_eq!(marked(&text, &diagnosed[0]), "\"a b\"");
}

#[test]
fn configurations_suggest_variables() {
    let (directory, _) = project(DECLARED, "");
    let path = directory.path().join("configuration.kdl");
    let saved = fs::read_to_string(&path).unwrap();
    let suggested = |line: &str| {
        let text = format!("{saved}{}", line.replace('|', ""));
        let offset = saved.len() + line.find('|').unwrap();

        document(&codeshaper::registry(), &path, &text, offset).into_iter().map(|item| item.label).collect::<Vec<_>>()
    };

    assert_eq!(suggested("variables |"), vec!["prefix", "engine"]);
    assert_eq!(suggested("variables pre|"), vec!["prefix", "engine"]);
    assert_eq!(suggested("variables prefix=\"é\" |\n"), vec!["engine"]);
    assert_eq!(suggested("variables en| prefix=\"a\"\n"), vec!["engine"]);

    for line in ["variables prefix=\"|\"\n", "variables|\n", "variables\n|", "variables; |", "|variables", "variables prefix=\"a\"|\n"] {
        assert_eq!(suggested(line), Vec::<String>::new(), "{line}");
    }
}

#[test]
fn declarations_are_symbols() {
    let found: Vec<(String, Option<String>)> = symbols(DECLARED).into_iter().map(|symbol| (symbol.name, symbol.detail)).collect();
    let pair = |name: &str, kind: &str| (name.to_string(), Some(kind.to_string()));

    assert_eq!(found, vec![pair("header", "constant"), pair("prefix", "variable"), pair("engine", "variable")]);
}

#[test]
fn calls_lead_to_their_declaration() {
    let (_directory, found) = project(DECLARED, "");
    let mut client = Client::start();
    let uri = client.open(&found.join("patches/calling.kdl"), CALLING);
    let settings = Url::from_file_path(found.join("project.kdl")).unwrap().to_string();
    let saved = fs::read_to_string(found.join("project.kdl")).unwrap();
    let line = |text: &str, name: &str| text.lines().position(|line| line.contains(&format!(" \"{name}\""))).unwrap();

    for (name, needle) in [("prefix", "#[prefix"), ("header", "#[header")] {
        let place = ask(&mut client, "textDocument/definition", at(&uri, CALLING, needle, 4));
        assert_eq!((&place[0]["uri"], &place[0]["range"]["start"]["line"]), (&json!(settings), &json!(line(&saved, name))));
    }
    assert_eq!(ask(&mut client, "textDocument/definition", at(&uri, CALLING, "#[none", 4)), Value::Null);
    assert_eq!(ask(&mut client, "textDocument/definition", at(&uri, CALLING, "#[own", 4))[0]["uri"], uri.as_str());

    let edited = format!("\n\n{saved}");
    client.open(&found.join("project.kdl"), &edited);
    let place = ask(&mut client, "textDocument/definition", at(&uri, CALLING, "#[prefix", 4));
    assert_eq!(place[0]["range"]["start"], json!({ "line": line(&edited, "prefix"), "character": 10 }));

    client.stop();
}

#[test]
fn declared_names_are_not_renamed_from_patches() {
    let (_directory, found) = project(DECLARED, "");
    let mut client = Client::start();
    let uri = client.open(&found.join("patches/calling.kdl"), CALLING);

    for (needle, refused) in [("#[prefix", true), ("#[header", true), ("#[own", false), ("#[none", false)] {
        let mut params = at(&uri, CALLING, needle, 4);
        assert_eq!(ask(&mut client, "textDocument/prepareRename", params.clone()).is_null(), refused, "{needle}");

        params["newName"] = json!("other");
        assert_eq!(ask(&mut client, "textDocument/rename", params).is_null(), refused, "{needle}");
    }

    let references = ask(&mut client, "textDocument/references", {
        let mut params = at(&uri, CALLING, "#[prefix", 4);
        params["context"] = json!({ "includeDeclaration": true });
        params
    });
    assert_eq!(references.as_array().unwrap().len(), 1);

    client.stop();
}

#[test]
fn open_patches_follow_saved_declarations() {
    let (directory, found) = project("", "");
    let mut client = Client::start();
    let uri = client.open(&found.join("patches/calling.kdl"), CALLING);
    published(&client, &uri, "No action named 'prefix'", true);

    let path = found.join("project.kdl");
    let settings = client.open(&path, &fs::read_to_string(&path).unwrap());
    let text = append(&path, DECLARED);
    client.notify("textDocument/didSave", json!({ "textDocument": { "uri": settings.as_str() }, "text": text }));
    published(&client, &uri, "Variable 'engine' has no value", true);

    let path = directory.path().join("configuration.kdl");
    let configuration = client.open(&path, &fs::read_to_string(&path).unwrap());
    let text = append(&path, "variables engine=\"4\"\n");
    client.notify("textDocument/didSave", json!({ "textDocument": { "uri": configuration.as_str() }, "text": text }));
    published(&client, &uri, "Variable 'engine' has no value", false);

    let hovered = ask(&mut client, "textDocument/hover", at(&uri, CALLING, "#[engine", 4));
    assert!(hovered["contents"]["value"].as_str().unwrap().contains("from the configuration\n```\n4\n```"));

    client.stop();
}

#[test]
fn traces_follow_the_values() {
    let (directory, found) = project(DECLARED, "");
    let text = CALLING.replace("#[engine]#[own]#[none]", "#[own]");
    let registry = codeshaper::registry();
    let mut traces = Traces::default();
    let output = |traces: &mut Traces| traces.get(&registry, &found, &text).unwrap().action("replacer #1").unwrap().samples[0].output.clone();
    assert_eq!(output(&mut traces), "my_Generatedx");

    std::thread::sleep(std::time::Duration::from_millis(20));
    append(&directory.path().join("configuration.kdl"), "variables prefix=\"x_\"\n");
    assert_eq!(output(&mut traces), "x_Generatedx");
}

fn set(client: &Client, project: &Path, values: Value) {
    let uri = Url::from_file_path(project.join("project.kdl")).unwrap().to_string();
    client.notify("codeshaper/setVariables", json!({ "project": uri, "values": values }));
}

fn listed(client: &mut Client, project: &Path) -> Value {
    let uri = Url::from_file_path(project.join("project.kdl")).unwrap().to_string();

    ask(client, "codeshaper/variables", json!({ "project": uri }))
}

type Row = (String, String, Option<String>, String, Option<String>);

fn rows(found: &Value) -> Vec<Row> {
    let text = |value: &Value| value.as_str().map(str::to_string);

    found["variables"].as_array().unwrap().iter()
        .map(|row| (text(&row["name"]).unwrap(), text(&row["kind"]).unwrap(), text(&row["value"]), text(&row["source"]).unwrap(), text(&row["otherwise"])))
        .collect()
}

fn row(name: &str, kind: &str, value: Option<&str>, source: &str, otherwise: Option<&str>) -> Row {
    (name.to_string(), kind.to_string(), value.map(str::to_string), source.to_string(), otherwise.map(str::to_string))
}

#[test]
fn variables_are_listed_and_set() {
    let (_directory, found) = project(DECLARED, "variables engine=\"4\"\n");
    let (_other_directory, other) = project(DECLARED, "");
    let mut client = Client::start();
    let header = || row("header", "constant", Some("Generated"), "constant", Some("Generated"));

    let first = listed(&mut client, &found);
    assert_eq!(first["name"], "Preview");
    assert_eq!(rows(&first), vec![
        header(), row("prefix", "variable", Some("my_"), "default", Some("my_")), row("engine", "variable", Some("4"), "configuration", Some("4"))
    ]);
    let saved = fs::read_to_string(found.join("project.kdl")).unwrap();
    let range: lsp_types::Range = serde_json::from_value(first["variables"][1]["range"].clone()).unwrap();
    let index = LineIndex::new(&saved);
    assert_eq!(&saved[index.offset(range.start)..index.offset(range.end)], "prefix");

    set(&client, &found, json!({ "engine": "é\n", "prefix": "", "gone": "x", "header": "y" }));
    let changed = listed(&mut client, &found);
    assert_eq!(rows(&changed), vec![
        header(), row("prefix", "variable", Some(""), "set", Some("my_")), row("engine", "variable", Some("é\n"), "set", Some("4"))
    ]);
    assert_eq!(changed["unknown"], json!(["gone", "header"]));
    assert_eq!(rows(&listed(&mut client, &other))[2], row("engine", "variable", None, "missing", None));

    set(&client, &found, json!({}));
    assert_eq!(listed(&mut client, &found), first);

    let missing = json!({ "project": Url::from_file_path(found.join("patches/none.kdl")).unwrap().to_string() });
    for wrong in [json!({}), json!({ "project": "é" }), json!({ "project": 1 }), missing] {
        assert!(client.request("codeshaper/variables", wrong).response_result.is_err());
    }
    client.notify("codeshaper/setVariables", json!({}));
    client.notify("codeshaper/setVariables", json!({ "project": Url::from_file_path(found.join("project.kdl")).unwrap().to_string(), "values": 3 }));
    assert_eq!(listed(&mut client, &found), first);

    fs::write(other.join("project.kdl"), "other {}").unwrap();
    assert!(client.request("codeshaper/variables", json!({ "project": Url::from_file_path(other.join("project.kdl")).unwrap().to_string() })).response_result.is_err());

    client.stop();
}

#[test]
fn set_variables_are_used_by_previews_and_hovers() {
    let (directory, found) = project(DECLARED, "");
    let text = CALLING.replace("#[own]#[none]", "").replace("    maker \"own\" {\n        make \"x\"\n    }\n", "");
    let path = found.join("patches/calling.kdl");
    fs::write(&path, &text).unwrap();
    fs::remove_file(found.join("patches/rename.kdl")).unwrap();
    let mut client = Client::start();
    let uri = client.open(&path, &text);
    let configuration = directory.path().join("configuration.kdl");
    let configuration = client.open(&configuration, &fs::read_to_string(&configuration).unwrap());
    let settings = Url::from_file_path(dunce::canonicalize(found.join("project.kdl")).unwrap()).unwrap().to_string();
    published(&client, &uri, "Variable 'engine' has no value", true);

    for (method, asked) in [("codeshaper/preview", &uri), ("codeshaper/previewOperation", &uri), ("codeshaper/previewOperation", &configuration)] {
        let error = client.request(method, json!({ "uri": asked.as_str() })).response_result.unwrap_err();
        assert!(error.message.starts_with("Variable 'engine' has no value"), "{}", error.message);
        assert_eq!(error.data.unwrap(), json!({ "project": settings, "variables": ["engine"] }));
    }

    set(&client, &found, json!({ "engine": "4" }));
    published(&client, &uri, "Variable 'engine' has no value", false);

    for (method, asked) in [("codeshaper/preview", &uri), ("codeshaper/previewOperation", &uri), ("codeshaper/previewOperation", &configuration)] {
        assert_eq!(ask(&mut client, method, json!({ "uri": asked.as_str() }))["files"][0]["result"], "my_Generated4() {}\n");
    }
    let hovered = ask(&mut client, "textDocument/hover", at(&uri, &text, "#[engine", 4));
    assert_eq!(hovered["contents"]["value"], "Variable of the project, as set\n```\n4\n```");

    let sample = |client: &mut Client| ask(client, "textDocument/hover", at(&uri, &text, "to \"", 4))["contents"]["value"].as_str().unwrap().to_string();
    assert!(sample(&mut client).contains("my_Generated4"), "{}", sample(&mut client));
    set(&client, &found, json!({ "engine": "5" }));
    assert!(sample(&mut client).contains("my_Generated5"), "{}", sample(&mut client));

    let broken = client.open(&path, &text.replace("void old", "void ("));
    let error = client.request("codeshaper/preview", json!({ "uri": broken.as_str() })).response_result.unwrap_err();
    assert_eq!(error.data, None);

    client.stop();
}
