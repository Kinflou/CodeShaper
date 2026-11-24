// Standard Uses
use std::fs::{self, File};
use std::path::Path;
use std::sync::Arc;
use std::time::{Duration, SystemTime};

// Crate Uses
use crate::preview::{operation, RENAME};

// External Uses
use codeshaper::core::shaping::patch::Patch;
use codeshaper_lsp::trace::{diff, inline, truncate, Limits, Traces};


const SMALL: Limits = Limits { samples: 1, lines: 2, characters: 10 };


fn age(path: &Path) {
    let earlier = SystemTime::now() - Duration::from_secs(60);

    File::options().write(true).open(path).unwrap().set_modified(earlier).unwrap();
}


#[test]
fn diffs_keep_shared_lines() {
    let limits = Limits::default();

    assert_eq!(diff("a\nb\nc", "a\nx\ny\nc", &limits), "```diff\n  a\n- b\n+ x\n+ y\n  c\n```");
    assert_eq!(diff("same", "same", &limits), "```diff\n  same\n```");
}

#[test]
fn long_text_is_cut_at_the_limits() {
    assert_eq!(truncate("one\ntwo\nthree", &SMALL), "one\ntwo\n…");
    assert_eq!(truncate("0123456789abc", &SMALL), "0123456789\n…");
    assert_eq!(truncate("short", &SMALL), "short");
}

#[test]
fn inline_values_show_whitespace() {
    assert_eq!(inline("a\n\tb"), "`a⏎⇥b`");
    assert_eq!(inline("a`b"), "`` a`b ``");
}

#[test]
fn traces_keep_the_sample_limit_and_are_reused() {
    let (_directory, project) = operation();
    let mut traces = Traces::new(SMALL);

    let first = traces.get(&codeshaper::registry(), &project, RENAME).unwrap();
    let again = traces.get(&codeshaper::registry(), &project, RENAME).unwrap();

    assert!(Arc::ptr_eq(&first, &again));
    assert_eq!(first.action("replacer #1").unwrap().samples.len(), 1);
    assert_eq!(first.files, 2);
}

#[test]
fn traces_follow_the_project() {
    let (_directory, project) = operation();
    let mut traces = Traces::new(SMALL);
    let first = traces.get(&codeshaper::registry(), &project, RENAME).unwrap();

    age(&project.join("project.kdl"));
    assert!(!Arc::ptr_eq(&first, &traces.get(&codeshaper::registry(), &project, RENAME).unwrap()));
}

#[test]
fn counts_are_reused_until_something_changes() {
    let (directory, project) = operation();
    let registry = codeshaper::registry();
    let patch = Patch { file: "*.cpp".to_string(), ..Patch::default() };
    let mut traces = Traces::default();
    let mut places = |locations: &[&str], keep_going: bool| {
        traces.guard(move || keep_going);
        let (_, found) = traces.places(&registry, &project, &patch, locations)?;

        Some(found.into_iter().map(|found| (found.matches.places, found.matches.matched_files, found.matches.files)).collect::<Vec<_>>())
    };

    assert_eq!(places(&["FunctionDefinition", "Nothing"], true), Some(vec![(2, 2, 2), (0, 0, 2)]));
    assert_eq!(places(&["Nothing", "FunctionDefinition"], false), Some(vec![(0, 0, 2), (2, 2, 2)]));
    assert_eq!(places(&["ClassSpecifier"], false), None);
    assert_eq!(places(&["FunctionDefinition", "ClassSpecifier"], true), Some(vec![(2, 2, 2), (0, 0, 2)]));

    let file = directory.path().join("target/a.cpp");
    fs::write(&file, "void a() {}\nvoid b() {}\n").unwrap();
    age(&file);
    assert_eq!(places(&["FunctionDefinition"], false), None);
    assert_eq!(places(&["FunctionDefinition"], true), Some(vec![(3, 2, 2)]));

    fs::write(directory.path().join("target/c.cpp"), "int c;\n").unwrap();
    assert_eq!(places(&["FunctionDefinition"], true), Some(vec![(3, 2, 3)]));

    age(&project.join("project.kdl"));
    assert_eq!(places(&["FunctionDefinition"], false), None);
}

#[test]
fn first_places_are_kept() {
    let (directory, project) = operation();
    let registry = codeshaper::registry();
    let patch = Patch { file: "*.cpp".to_string(), ..Patch::default() };
    fs::write(directory.path().join("target/a.cpp"), "void a() {}\r\nvoid b() {\r\n}\r\nvoid c() {}\r\n").unwrap();

    for (samples, expected) in [(0, vec![]), (2, vec!["a.cpp void a() {}", "a.cpp void b() {\n}"]), (9, vec!["a.cpp void a() {}", "a.cpp void b() {\n}", "a.cpp void c() {}", "b.cpp void keep() {}"])] {
        let mut traces = Traces::new(Limits { samples, ..Limits::default() });
        let (folder, located) = traces.places(&registry, &project, &patch, &["FunctionDefinition"]).unwrap();
        let found: Vec<String> = located[0].found.iter().map(|found| format!("{} {}", found.file.display(), found.matched.text)).collect();

        assert_eq!((folder, located[0].matches.places, found), (directory.path().join("target"), 4, expected.into_iter().map(str::to_string).collect()));
    }
}
