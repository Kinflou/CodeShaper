// Standard Uses
use std::path::Path;
use std::rc::Rc;

// Crate Uses
use crate::expressions::arguments::shaped;
use crate::operation::{patch, read, registry, run, workspace};

// External Uses
use codeshaper_core::expressions::builtins::{call, closest, Known, BUILTINS};
use codeshaper_core::shaping::operation::Operation;
use codeshaper_core::shaping::patch::Patch;
use codeshaper_core::shaping::patch::controller::PatchController;
use codeshaper_core::shaping::patch::validation::validate_source;


fn called(name: &str, arguments: &[&str]) -> Option<String> {
    let arguments: Vec<String> = arguments.iter().map(|argument| argument.to_string()).collect();

    call(name, &arguments, Known::default())
}

fn cases(text: &str) -> Vec<String> {
    ["snake", "kebab", "camel", "pascal", "constant"].iter().map(|case| called(&format!("case.{case}"), &[text]).unwrap()).collect()
}

fn file(path: &str) -> Vec<String> {
    ["path", "folder", "name", "stem", "extension"].iter()
        .map(|part| call(&format!("file.{part}"), &[], Known { file: Some(Path::new(path)), ..Known::default() }).unwrap())
        .collect()
}

fn found(to: &str, more: &str) -> Vec<(String, String)> {
    let content = format!("file \"*\"\nactions {{\n{more}\nreplacer {{\nlocation \"Line\"\nfrom \"(a)\"\nto \"{to}\"\n}}\n}}\n");
    let (patch, source) = Patch::parse("", &content).unwrap();

    validate_source(&patch, &source, [], &[]).into_iter()
        .filter(|diagnostic| diagnostic.kind.code() != "unused-action")
        .map(|diagnostic| {
            let span = diagnostic.span.unwrap();
            (format!("{}: {}", diagnostic.kind.code(), diagnostic.message), content[span.offset()..span.offset() + span.len()].to_string())
        })
        .collect()
}

fn builder(build: &str, collect: &str) -> String {
    format!("builder \"b\" {{\nlocation \"Line\"\nmatch r\"^k(\\w+)\"\nbuild \"{build}\"\n{collect}\n}}\n")
}


#[test]
fn words_are_joined_in_every_style() {
    for text in ["getPlayer", "GetPlayer", "get_player", "GET_PLAYER", "get-player", "get player", "get.player!"] {
        assert_eq!(cases(text), ["get_player", "get-player", "getPlayer", "GetPlayer", "GET_PLAYER"], "{text}");
    }

    let snake = |text: &str| cases(text).remove(0);
    let words = [
        ("HTTPServer", "http_server"), ("HTTPServer2", "http_server2"), ("http2Server", "http2_server"), ("playerID", "player_id"),
        ("CharacterBody2D", "character_body2d"), ("Vector2DArray", "vector2d_array"), ("x", "x"), ("X", "x"), ("XY", "xy"), ("aB", "a_b"),
        ("_privateThing", "_private_thing"), ("__init__", "__init__"), ("a__b", "a_b"), ("-a-", "a"), ("2d", "2d"), ("ÉtatÜber", "état_über"),
        ("", ""), ("__", "__"), ("日本Go", "日本_go"), ("my-shape.test", "my_shape_test")
    ];
    for (text, expected) in words {
        assert_eq!(snake(text), expected, "{text}");
    }

    assert_eq!(cases("_HTTPServer_"), ["_http_server_", "_http-server_", "_httpServer_", "_HttpServer_", "_HTTP_SERVER_"]);
    assert_eq!((called("case.upper", &["straße é"]), called("case.lower", &["ÀB_c"])), (Some("STRASSE É".to_string()), Some("àb_c".to_string())));
}

#[test]
fn styles_are_stable() {
    for text in ["getPlayer", "HTTPServer2", "_a__b_", "CharacterBody2D", "x2Y", "日本Go", "a1B2c3"] {
        for (index, once) in cases(text).into_iter().enumerate() {
            if index == 0 || index == 1 || index == 4 { assert_eq!(cases(&once)[index], once, "{text}"); }
        }
    }
}

#[test]
fn texts_are_changed() {
    let indent = |text: &str| called("text.indent", &[text, "  "]).unwrap();
    assert_eq!(indent("a\n\n b\n"), "  a\n\n   b\n");
    assert_eq!(indent("a\r\n\r\nb"), "  a\r\n\r\n  b");
    assert_eq!((indent(""), indent("\n"), indent("é")), (String::new(), "\n".to_string(), "  é".to_string()));

    assert_eq!((called("text.or", &["", "b"]).unwrap(), called("text.or", &["a", "b"]).unwrap(), called("text.or", &[" ", "b"]).unwrap()), ("b".into(), "a".into(), " ".into()));
    assert_eq!(called("text.replace", &["a.b.c", ".", "::"]).unwrap(), "a::b::c");
    assert_eq!(called("text.replace", &["aé", "", "x"]).unwrap(), "aé");
    assert_eq!(called("text.replace", &["$0$0", "$0", "$1"]).unwrap(), "$1$1");
}

#[test]
fn files_are_taken_apart() {
    assert_eq!(file("src/deep/Shape.test.h"), ["src/deep/Shape.test.h", "src/deep", "Shape.test.h", "Shape.test", "h"]);
    assert_eq!(file("./é b"), ["é b", "", "é b", "é b", ""]);
    assert_eq!(file(".hidden"), [".hidden", "", ".hidden", ".hidden", ""]);
    #[cfg(windows)]
    assert_eq!(file("src\\a.h"), ["src/a.h", "src", "a.h", "a", "h"]);

    assert_eq!(called("file.path", &[]), None);
    assert_eq!(called("collect.index", &[]), None);
}

#[test]
fn every_builtin_is_explained_and_called() {
    for builtin in BUILTINS {
        let arguments = vec!["a"; builtin.arguments.len()];
        let arguments: Vec<String> = arguments.into_iter().map(str::to_string).collect();
        let known = Known { file: Some(Path::new("a.h")), collected: Some((1, 2)) };

        assert!(call(builtin.name, &arguments, known).is_some(), "{}", builtin.name);
        assert_eq!(call(builtin.name, &vec!["a".to_string(); arguments.len() + 1], known), None, "{}", builtin.name);
        assert!(builtin.text.ends_with('.') && builtin.example.contains(&format!("#[{}", builtin.name)), "{}", builtin.name);
    }

    assert_eq!((closest("file.pth"), closest("case.snak"), closest("text.ident"), closest("a.b")), (Some("file.path"), Some("case.snake"), Some("text.indent"), None));
}

#[test]
fn expressions_call_builtins() {
    assert_eq!(shaped("", "#[case.upper($[0])]#[case.pascal('$[0] $[1]')]"), "ONEOneTwo");
    assert_eq!(shaped("", "#[text.replace(#[case.upper($[0])], 'N', $[1])]()"), "OtwoE()");
    assert_eq!(shaped("maker \"m\" {\nmake \"#[case.upper(@[0])]\"\n}\nmaker \"file\" {\nmake \"f\"\n}\n", "#[m($[0])]#[file].path"), "ONEf.path");
    assert_eq!(shaped("switch \"s\" {\ncases ONE=\"#[text.or('', @[0])]\"\nindex \"#[case.upper(@[0])]\"\n}\n", "#[s($[0])]"), "one");
}

#[test]
fn collected_texts_are_counted() {
    let lines = |build: &str, text: &str| {
        let content = format!("file \"*\"\nactions {{\n{}replacer {{\nlocation \"File\"\nfrom \"(?s)^.*$\"\nto \"#[b]\"\n}}\n}}\n", builder(build, "collect true\nseparator \" \""));
        let patch = Rc::new(Patch::from_kdl("", &content).unwrap());

        PatchController::with(patch).apply(registry().backend("lines").unwrap(), text).unwrap()
    };

    assert_eq!(lines("$[0]#[collect.index]/#[collect.count]", "ka\nkb\nkc"), "a0/3 b1/3 c2/3");
    assert_eq!(lines("$[0]#[collect.index]/#[collect.count]", "ka\nx\nkc\ny"), "a0/2 c1/2");
    assert_eq!(lines("-#[collect.index]/#[collect.count]", "ka\nx"), "-0/2 -1/2");
    assert_eq!(lines("#[text.or('$[0]', '')]#[collect.index]/#[collect.count]", "x\nka"), "a0/1");
    assert_eq!(lines("$[0]#[case.upper('#[collect.index]')]", "x"), "");
}

#[test]
fn files_are_known_in_runs_previews_and_traces() {
    let content = patch("lines", "Line", r"old (\w+)", "#[file.path]|#[file.folder]|#[file.stem]");
    let directory = workspace("replace", &[("a.kdl", content.clone()), ("b.kdl", content.replace("old (\\w+)", "(keep)"))]);
    let operation = Operation::from_file(&directory.path().join("configuration.kdl")).unwrap();
    let patch = Patch::from_kdl("", &content).unwrap();

    assert_eq!(operation.preview(&registry(), "a.kdl", &patch).unwrap()[0].result.as_ref().unwrap(), "a.txt||a\nkeep\n");
    let trace = operation.trace(&registry(), "a.kdl", &patch, 9).unwrap();
    assert_eq!(trace.action("replacer #1").unwrap().samples[0].output, "a.txt||a");

    run(&directory).unwrap();
    assert_eq!(read(&directory, "target/a.txt"), "a.txt||a\na.txt||a\n");
    assert_eq!(read(&directory, "target/nested/b.txt"), "nested/b.txt|nested|b\n");
}

#[test]
fn texts_without_a_file_are_refused() {
    let applied = |to: &str| {
        let patch = Rc::new(Patch::from_kdl("", &patch("lines", "Line", "(old)", to)).unwrap());
        let registry = registry();
        let backend = registry.backend("lines").unwrap();

        (
            PatchController::with(patch.clone()).apply(backend, "old").map_err(|error| error.to_string()),
            PatchController::with(patch).for_file(Path::new("x/a.h")).apply(backend, "old").unwrap()
        )
    };

    let refused = Err("'#[file.stem]' needs the file being patched, but the text is in none".to_string());
    assert_eq!(applied("#[case.upper(#[file.stem])]"), (refused, "A".to_string()));
    assert_eq!(applied("#[case.upper($[0])]"), (Ok("OLD".to_string()), "OLD".to_string()));
}

#[test]
fn wrong_calls_are_reported() {
    let one = |message: &str, marked: &str| vec![(message.to_string(), marked.to_string())];

    assert_eq!(found("#[file.pth]", ""), one("action-not-found: No built-in named 'file.pth', did you mean 'file.path'?", "#[file.pth]"));
    assert_eq!(found("#[a.b('x')]", ""), one("action-not-found: No built-in named 'a.b'", "#[a.b"));
    assert_eq!(found("#[case.snake]", ""), one("incorrect-args: '#[case.snake]' takes 1 argument(s) (text), but is called with 0", "#[case.snake]"));
    assert_eq!(found("#[file.path('x')]", ""), one("incorrect-args: '#[file.path]' takes no arguments, but is called with 1", "#[file.path"));
    assert_eq!(found("#[text.replace('a', 'b')]", "")[0].0, "incorrect-args: '#[text.replace]' takes 3 argument(s) (text, old, new), but is called with 2");
    assert_eq!(found("#[case.upper('#[gone]')]", ""), one("action-not-found: No action named 'gone'", "#[gone]"));

    let only = |name: &str| format!("incorrect-expression: '#[{name}]' is only known in the 'build' of a builder with 'collect true'");
    assert_eq!(found("#[collect.index]", ""), one(&only("collect.index"), "#[collect.index]"));
    assert_eq!(found("#[b]", &builder("#[collect.count]", "")), one(&only("collect.count"), "#[collect.count]"));
    assert_eq!(found("#[b]", &builder("#[m]", "collect true\nactions {\nmaker \"m\" {\nmake \"#[collect.count]\"\n}\n}")), one(&only("collect.count"), "#[collect.count]"));
    assert_eq!(found("#[b]", &builder("#[collect.count]", "collect true")), vec![]);

    for incomplete in ["#[file.]", "#[.path]", "#[file..path]", "#[file.path.]", "$[a.b]", "@[a.b]"] {
        assert!(found(incomplete, "")[0].0.contains("is not a complete call"), "{incomplete}");
    }
    assert_eq!(found("#[file.path.x]", "")[0].0, "action-not-found: No built-in named 'file.path.x', did you mean 'file.path'?");
}
