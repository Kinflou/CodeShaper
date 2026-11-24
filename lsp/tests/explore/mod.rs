// Standard Uses
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

// Crate Uses
use crate::protocol::Client;

// External Uses
use codeshaper::core::registry::Registry;
use codeshaper_lsp::explore::{answer, block_at, files, hints, Located, FILES_REQUEST, PLACES_REQUEST, PLACE_REQUEST};
use codeshaper_lsp::position::LineIndex;
use serde_json::{json, Value};
use tempfile::TempDir;
use url::Url;


const A: &str = "\
void one()
{
    int é = 1; call(é);
}

void two() {}
";

const B: &str = "void three() {}\n";

const PATCH: &str = r##"enabled false
file "*.cpp"

actions {
    builder "outer" {
        location "FunctionDefinition.Body"
        match r"(.*)"
        build "$[0]"
        actions {
            builder "inner" {
                location "FunctionDefinition"
                match r"(.*)"
                build "$[0]"
            }
        }
    }
    replacer {
        location "FunctionDeclarator.Declarator"
        reference-location "FunctionDefinition"
        from r"one"
        to "two"
    }
}
"##;


fn write(directory: &TempDir, path: &str, content: &str) -> PathBuf {
    let path = directory.path().join(path);

    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(&path, content).unwrap();
    path
}

fn workspace() -> (TempDir, PathBuf, PathBuf) {
    let directory = tempfile::tempdir().unwrap();

    let configuration = write(&directory, "configuration.kdl", "name \"A\"\nproject \"shaping/\"\ntarget \"code/\"\noutput \"out/\"\nresult \"replace\"\n");
    write(&directory, "shaping/project.kdl", "name \"A\"\ntarget \"text\"\nast-set { cpp \"*.cpp\"; }\ndescription \"\"\n");
    let patch = write(&directory, "shaping/patches/first.kdl", PATCH);
    write(&directory, "code/a.cpp", A);
    write(&directory, "code/b.cpp", B);
    write(&directory, "code/c.cpp", "int none;\n");
    write(&directory, "code/notes.txt", "void text() {}\n");

    (directory, configuration, patch)
}

fn uri(path: &Path) -> String {
    Url::from_file_path(path).unwrap().to_string()
}

fn params(start: &Path, name: &str, text: &str) -> Value {
    json!({ "uri": uri(start), "file": { "path": name, "language": "cpp", "usual": "cpp" }, "text": text })
}

fn at(mut params: Value, text: &str, needle: &str, length: usize) -> Value {
    let start = text.find(needle).unwrap();
    params["range"] = json!(LineIndex::new(text).range(start..start + length));
    params
}

fn ask(client: &mut Client, method: &str, params: Value) -> Value {
    client.request(method, params).response_result.unwrap()
}

fn locations(found: &Value) -> Vec<(&str, u64, u64)> {
    found["paths"].as_array().unwrap().iter()
        .map(|path| (path["location"].as_str().unwrap(), path["number"].as_u64().unwrap(), path["places"].as_u64().unwrap()))
        .collect()
}

fn used(found: &Value) -> Vec<(&str, &str, &str, bool)> {
    found["used"].as_array().unwrap().iter()
        .map(|found| (found["patch"].as_str().unwrap(), found["action"].as_str().unwrap(), found["field"].as_str().unwrap(), found["enabled"].as_bool().unwrap()))
        .collect()
}


#[test]
fn files_come_from_the_configuration_or_the_patch() {
    let (directory, configuration, patch) = workspace();
    let mut client = Client::start();

    let found = ask(&mut client, FILES_REQUEST, json!({ "uri": uri(&configuration) }));
    let names: Vec<&str> = found["files"].as_array().unwrap().iter().map(|file| file["path"].as_str().unwrap()).collect();
    assert_eq!(names, vec!["a.cpp", "b.cpp", "c.cpp"]);
    assert_eq!(found["without"], 1);
    assert_eq!(found["files"][0]["language"], "cpp");
    assert_eq!(found["files"][0]["uri"], uri(&directory.path().join("code/a.cpp")));

    fs::write(&patch, PATCH.replace("file \"*.cpp\"", "file \"b*\"")).unwrap();
    let found = ask(&mut client, FILES_REQUEST, json!({ "uri": uri(&patch) }));
    assert_eq!(found["files"].as_array().unwrap().len(), 1);
    assert_eq!((&found["files"][0]["path"], &found["without"]), (&json!("b.cpp"), &json!(0)));

    client.open(&patch, &PATCH.replace("file \"*.cpp\"", "alias \"cpp\"\nfile \"*.txt\""));
    let found = ask(&mut client, FILES_REQUEST, json!({ "uri": uri(&patch) }));
    assert_eq!(found["files"][0]["path"], "notes.txt");
    assert_eq!((&found["files"][0]["language"], &found["files"][0]["usual"]), (&json!("cpp"), &Value::Null));

    client.stop();
}

#[test]
fn broken_patches_do_not_stop_exploring() {
    let (directory, configuration, patch) = workspace();
    write(&directory, "shaping/patches/second.kdl", "enabled true\nfile \"*.cpp\"\nactions {\n    replacer {\n        location \"FunctionDefinition\"\n        from r\"");
    let mut client = Client::start();

    for start in [&configuration, &patch] {
        assert_eq!(ask(&mut client, FILES_REQUEST, json!({ "uri": uri(start) }))["files"].as_array().unwrap().len(), 3);
    }

    let found = ask(&mut client, "codeshaper/exploreBlock", at(params(&configuration, "a.cpp", A), A, "two", 0));
    assert!(used(&found).contains(&("second.kdl", "replacer #1", "location", true)), "{found}");

    client.stop();
}

#[test]
fn failures_say_why() {
    let (directory, configuration, _) = workspace();
    let mut client = Client::start();
    let mut failure = |method: &str, params: Value| client.request(method, params).response_result.unwrap_err().message;

    assert_eq!(failure(FILES_REQUEST, json!({ "uri": uri(&directory.path().join("shaping/project.kdl")) })), "Not a patch or configuration");
    assert_eq!(failure("codeshaper/exploreBlock", json!({ "uri": uri(&configuration) })), "The request has no text");
    let mut unknown = at(params(&configuration, "a.cpp", A), A, "two", 0);
    unknown["file"]["language"] = json!("cobol");
    assert_eq!(failure("codeshaper/exploreBlock", unknown), "'cobol' is not a known language");
    let mut wrong = at(params(&configuration, "a.cpp", A), A, "two", 0);
    wrong["location"] = json!("Nothing.Like.This");
    assert!(failure(PLACE_REQUEST, wrong.clone()).contains("not a cpp node kind"));
    assert!(failure(PLACES_REQUEST, wrong).contains("not a cpp node kind"));

    fs::write(directory.path().join("shaping/project.kdl"), "name \"A\" }\n").unwrap();
    assert!(failure(FILES_REQUEST, json!({ "uri": uri(&configuration) })).contains("project.kdl:1:10: found `}`"));

    client.stop();
}

#[test]
fn blocks_are_described() {
    let (_directory, configuration, _) = workspace();
    let mut client = Client::start();
    let mut block = |needle: &str, length: usize| ask(&mut client, "codeshaper/exploreBlock", at(params(&configuration, "a.cpp", A), A, needle, length));

    let found = block("{\n", 0);
    assert_eq!((&found["kind"], &found["field"], &found["broken"]), (&json!("CompoundStatement"), &json!("Body"), &json!(false)));
    assert_eq!(found["range"], json!(LineIndex::new(A).range(A.find('{').unwrap()..A.find("}\n").unwrap() + 1)));
    assert_eq!(locations(&found)[..2], [("FunctionDefinition.Body", 1, 2), ("CompoundStatement", 1, 2)]);
    assert_eq!(found["paths"][0]["ranges"][1], json!(LineIndex::new(A).range(A.find("{}").unwrap()..A.find("{}").unwrap() + 2)));
    assert_eq!(found["child"]["start"], json!({ "line": 2, "character": 4 }));
    assert!(found["query"].as_str().unwrap().contains("\"one\""));
    assert_eq!(found["queryLanguage"]["name"], "tree-sitter");
    assert_eq!(found["parents"].as_array().unwrap().len(), 1);
    assert_eq!(found["parents"][0]["kind"], "FunctionDefinition");
    assert_eq!(used(&found), vec![
        ("first.kdl", "outer", "location", false), ("first.kdl", "outer/inner", "location", false), ("first.kdl", "replacer #1", "reference-location", false)
    ]);
    assert_eq!((&found["used"][0]["whole"], &found["used"][1]["whole"]), (&json!(true), &json!(false)));
    assert_eq!(found["used"][0]["range"]["start"], json!({ "line": 5, "character": 17 }));

    let found = block("two()", 0);
    assert_eq!(locations(&found)[0], ("FunctionDeclarator.Declarator", 2, 2));
    assert_eq!(used(&found)[0], ("first.kdl", "replacer #1", "location", false));

    let call = block("call(é)", 0);
    assert_eq!((&call["kind"], &call["child"]), (&json!("Identifier"), &Value::Null));
    let selected = block("call(é)", "call(é)".len());
    assert_eq!(selected["kind"], "CallExpression");
    assert_eq!(selected["range"]["end"], json!({ "line": 2, "character": 22 }));

    assert!(block("\nvoid two", 0).is_null());

    client.stop();
}

#[test]
fn other_patches_use_their_own_language() {
    let (directory, configuration, patch) = workspace();
    fs::remove_file(&patch).unwrap();
    for (name, alias, file) in [("a.kdl", "cpp", "*.txt"), ("b.kdl", "gdscript", "*.txt"), ("c.kdl", "", "*.txt"), ("d.kdl", "cpp", "*.cpp")] {
        let alias = if alias.is_empty() { String::new() } else { format!("alias \"{alias}\"\n") };
        write(&directory, &format!("shaping/patches/{name}"), &format!("{alias}{}", PATCH.replace("*.cpp", file)));
    }

    let mut client = Client::start();
    let mut asked = at(params(&configuration, "notes.txt", "void text() {}\n"), "void text() {}\n", "{}", 0);
    asked["file"]["usual"] = Value::Null;
    let found = ask(&mut client, "codeshaper/exploreBlock", asked);
    assert!(!used(&found).is_empty() && used(&found).iter().all(|(patch, ..)| *patch == "a.kdl"), "{found}");

    client.stop();
}

#[test]
fn unsaved_patches_are_used() {
    let (_directory, configuration, patch) = workspace();
    let mut client = Client::start();
    client.open(&patch, &PATCH.replace("enabled false", "enabled true").replace("FunctionDeclarator.Declarator", "PrimitiveType"));

    let found = ask(&mut client, "codeshaper/exploreBlock", at(params(&configuration, "a.cpp", A), A, "void two", 0));
    assert_eq!(used(&found)[0], ("first.kdl", "replacer #1", "location", true));
    let found = ask(&mut client, "codeshaper/exploreBlock", at(params(&configuration, "a.cpp", A), A, "two", 0));
    assert!(!used(&found).contains(&("first.kdl", "replacer #1", "location", true)));

    client.stop();
}

#[test]
fn places_follow_each_other_through_the_files() {
    let (directory, configuration, _) = workspace();
    let mut client = Client::start();
    let mut step = |name: &str, text: &str, needle: &str, length: usize, forward: bool| {
        let mut asked = at(params(&configuration, name, text), text, needle, length);
        asked["location"] = json!("FunctionDeclarator.Declarator");
        asked["forward"] = json!(forward);
        let found = ask(&mut client, PLACE_REQUEST, asked);
        (found["path"].as_str().unwrap().to_string(), found["range"]["start"]["line"].as_u64().unwrap(), found["number"].as_u64().unwrap(), found["places"].as_u64().unwrap())
    };

    assert_eq!(step("a.cpp", A, "one", 3, true), ("a.cpp".to_string(), 5, 2, 2));
    assert_eq!(step("a.cpp", A, "two", 3, true), ("b.cpp".to_string(), 0, 1, 1));
    assert_eq!(step("b.cpp", B, "three", 5, true), ("a.cpp".to_string(), 0, 1, 2));
    assert_eq!(step("a.cpp", A, "two", 3, false), ("a.cpp".to_string(), 0, 1, 2));
    assert_eq!(step("a.cpp", A, "one", 3, false), ("b.cpp".to_string(), 0, 1, 1));
    assert_eq!(step("b.cpp", B, "three", 5, false), ("a.cpp".to_string(), 5, 2, 2));
    assert_eq!(step("a.cpp", A, "int", 0, true), ("a.cpp".to_string(), 5, 2, 2));
    assert_eq!(step("a.cpp", A, "int", 0, false), ("a.cpp".to_string(), 0, 1, 2));
    assert_eq!(step("gone.cpp", "int x;\n", "int", 0, true), ("a.cpp".to_string(), 0, 1, 2));
    assert_eq!(step("gone.cpp", "int x;\n", "int", 0, false), ("b.cpp".to_string(), 0, 1, 1));

    fs::write(directory.path().join("code/a.cpp"), "int x;\n").unwrap();
    assert_eq!(step("b.cpp", B, "three", 5, true), ("b.cpp".to_string(), 0, 1, 1));

    let mut asked = at(params(&configuration, "a.cpp", A), A, "one", 3);
    asked["location"] = json!("WhileStatement");
    assert!(ask(&mut client, PLACE_REQUEST, asked.clone()).is_null());
    assert_eq!(ask(&mut client, PLACES_REQUEST, asked), json!([]));

    client.stop();
}

#[test]
fn every_place_is_listed() {
    let (directory, configuration, patch) = workspace();
    let mut client = Client::start();

    for start in [&configuration, &patch] {
        let mut asked = params(start, "a.cpp", &A.replace("two", "twice"));
        asked["location"] = json!("FunctionDeclarator.Declarator");
        let found = ask(&mut client, PLACES_REQUEST, asked);
        let found: Vec<(String, u64, u64)> = found.as_array().unwrap().iter()
            .map(|place| (place["uri"].as_str().unwrap().to_string(), place["range"]["start"]["line"].as_u64().unwrap(), place["range"]["end"]["character"].as_u64().unwrap()))
            .collect();
        let (a, b) = (uri(&directory.path().join("code/a.cpp")), uri(&directory.path().join("code/b.cpp")));

        assert_eq!(found, vec![(a.clone(), 0, 8), (a, 5, 10), (b, 0, 10)]);
    }

    client.stop();
}

#[test]
fn blocks_of_several_lines_are_hinted() {
    let (_directory, configuration, _) = workspace();
    let mut client = Client::start();
    let mut asked = params(&configuration, "a.cpp", A);
    let mut labels = |depth: u64| {
        asked["depth"] = json!(depth);
        let found = ask(&mut client, "codeshaper/exploreHints", asked.clone());
        found.as_array().unwrap().iter()
            .map(|hint| format!("{}:{} {} {}", hint["position"]["line"], hint["position"]["character"], hint["parts"][0]["label"].as_str().unwrap(), hint["parts"][0]["location"].as_str().unwrap()))
            .collect::<Vec<_>>()
    };

    assert_eq!(labels(0), Vec::<String>::new());
    assert_eq!(labels(1), vec!["0:0 FunctionDefinition FunctionDefinition"]);
    assert_eq!(labels(4), vec!["0:0 FunctionDefinition FunctionDefinition", "1:0 Body FunctionDefinition.Body"]);

    let mut asked = params(&configuration, "b.cpp", B);
    asked["all"] = json!(true);
    let found = ask(&mut client, "codeshaper/exploreHints", asked);
    let labels: Vec<(u64, Vec<&str>)> = found.as_array().unwrap().iter()
        .map(|hint| (hint["position"]["character"].as_u64().unwrap(), hint["parts"].as_array().unwrap().iter().map(|part| part["label"].as_str().unwrap()).collect()))
        .collect();
    assert_eq!(labels, vec![(0, vec!["FunctionDefinition", "Type"]), (5, vec!["FunctionDeclarator", "Declarator"]), (10, vec!["Parameters"]), (13, vec!["Body"])]);

    for hint in found.as_array().unwrap() {
        let joined: Vec<&str> = hint["parts"].as_array().unwrap().iter().map(|part| part["label"].as_str().unwrap()).collect();
        if joined.len() > 1 {
            let last = hint["parts"].as_array().unwrap().last().unwrap();
            let places = codeshaper::registry().backend("cpp").unwrap().locate(B, &joined.join(".")).unwrap();
            assert!(places.iter().any(|place| json!(LineIndex::new(B).range(place.clone())) == last["range"]), "{joined:?}");
        }

        for part in hint["parts"].as_array().unwrap() {
            let mut asked = params(&configuration, "b.cpp", B);
            asked["range"] = part["range"].clone();
            asked["node"] = part["node"].clone();
            let block = ask(&mut client, "codeshaper/exploreBlock", asked.clone());
            assert_eq!(block["range"], part["range"]);
            assert_eq!(block["paths"][0]["location"], part["location"]);

            for other in [0, 1000] {
                asked["node"] = json!(other);
                assert_eq!(ask(&mut client, "codeshaper/exploreBlock", asked.clone())["range"], part["range"]);
            }
        }
    }

    client.stop();
}

#[test]
fn every_position_can_be_asked() {
    let (_directory, configuration, _) = workspace();
    let registry = codeshaper::registry();

    for text in ["", "\n", "é", "void a( {\r\n  é\r\n}}", "class A { void b() { int = ; } };", A] {
        let nodes = registry.backend("cpp").unwrap().nodes(text).unwrap();
        for (start, _) in text.char_indices().chain([(text.len(), ' ')]) {
            for (end, _) in text[start..].char_indices().chain([(text.len() - start, ' ')]).take(12) {
                let _ = block_at(&nodes, &(start..start + end));
            }

            let mut asked = params(&configuration, "a.cpp", text);
            asked["range"] = json!(LineIndex::new(text).range(start..start));
            answer(&registry, &mut Located::default(), "codeshaper/exploreBlock", &configuration, &asked, &HashMap::new(), &|| true).unwrap();
        }
        hints(text, &nodes, Some(10));
        hints(text, &nodes, None);
    }
}

#[test]
fn searches_can_be_stopped() {
    let (_directory, configuration, _) = workspace();
    let registry = codeshaper::registry();
    let mut asked = at(params(&configuration, "a.cpp", A), A, "two", 3);
    asked["location"] = json!("FunctionDeclarator.Declarator");

    for method in [PLACE_REQUEST, PLACES_REQUEST] {
        let found = answer(&registry, &mut Located::default(), method, &configuration, &asked, &HashMap::new(), &|| false);
        assert_eq!(found.unwrap_err().to_string(), "Stopped");
    }
}

#[test]
fn languages_without_nodes_have_no_blocks() {
    let (_directory, configuration, _) = workspace();
    let mut registry = Registry::default();
    codeshaper_target_text::register(&mut registry);

    assert_eq!(files(&registry, &configuration, None, &HashMap::new()).unwrap().without, 4);
    assert_eq!(block_at(&[], &(0..0)), None);
    assert_eq!(hints("", &[], Some(4)), vec![]);
}
