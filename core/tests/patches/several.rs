// Standard Uses
use std::fs;

// Crate Uses
use crate::operation::{patch, registry, rename, report, run, workspace};

// External Uses
use codeshaper_core::shaping::operation::Operation;
use codeshaper_core::shaping::operation::report::Outcome;
use codeshaper_core::shaping::patch::Patch;
use codeshaper_core::shaping::patch::file::NamedPatch;
use codeshaper_core::shaping::project::Project;
use codeshaper_core::shaping::patch::validation::DiagnosticKind;
use miette::Report;
use tempfile::TempDir;


const FIELDS: &str = "enabled true\nfile \"*\"\nactions {\n}\n";

fn block(name: &str, content: &str) -> String {
    format!("patch \"{name}\" {{\n{content}}}\n")
}

fn parsed(text: &str) -> Vec<NamedPatch> {
    Patch::parse_all("several.kdl", text).unwrap()
}

fn names(text: &str) -> Vec<Option<String>> {
    parsed(text).into_iter().map(|named| named.name).collect()
}

fn errors(text: &str) -> Vec<(String, String)> {
    let report = Patch::parse_all("several.kdl", text).unwrap_err();

    let found = report.related().unwrap()
        .map(|error| {
            let label = error.labels().and_then(|mut labels| labels.next());
            let marked = label.map(|label| text[label.offset()..label.offset() + label.len()].to_string());

            (error.to_string(), marked.unwrap_or_default())
        })
        .collect();
    assert!(report.source_code().is_some());

    found
}

fn error(text: &str) -> (String, String) {
    let mut found = errors(text);
    assert_eq!(found.len(), 1, "{found:?}");

    found.remove(0)
}

fn pair(message: &str, marked: &str) -> (String, String) {
    (message.to_string(), marked.to_string())
}

fn project(directory: &TempDir) -> Result<Project, Report> {
    Project::from_directory(&directory.path().join("shaping"))
}

fn patched(directory: &TempDir) -> Vec<String> {
    run(directory).unwrap().into_iter()
        .find_map(|file| match file.outcome {
            Outcome::Patched(patches) => Some(patches),
            Outcome::Unchanged => None
        })
        .unwrap_or_default()
}


#[test]
fn files_without_blocks_have_one_unnamed_patch() {
    assert_eq!(names(FIELDS), vec![None]);
}

#[test]
fn blocks_are_named() {
    assert_eq!(names(&block("one", FIELDS)), vec![Some("one".to_string())]);
    assert_eq!(
        names(&(block("b", FIELDS) + &block("a", FIELDS) + &block("é 日", FIELDS))),
        vec![Some("b".to_string()), Some("a".to_string()), Some("é 日".to_string())]
    );
}

#[test]
fn blocks_keep_their_own_content() {
    let one = rename("lines", "Line");
    let two = patch("lines", "File", "a", "b").replace("enabled true", "enabled false");
    let found = parsed(&(block("one", &one) + &block("two", &two)));

    assert_eq!(found[0].patch, Patch::from_kdl("", &one).unwrap());
    assert_eq!(found[1].patch, Patch::from_kdl("", &two).unwrap());
}

#[test]
fn raw_strings_and_comments_are_not_blocks() {
    let tricky = patch("lines", "Line", "x", "y").replace("r\"x\"", "r#\"} patch \"two\" {\"#");
    let text = format!("// patch \"a\" {{}}\n/* patch \"b\" {{ */\n/-{}{}", block("gone", FIELDS), block("one", &tricky));
    let found = parsed(&text);

    assert_eq!(found.len(), 1);
    assert_eq!(found[0].name.as_deref(), Some("one"));
    assert_eq!(found[0].patch.actions.replacers[0].from, "} patch \"two\" {");
}

#[test]
fn line_endings_are_read_alike() {
    let text = block("one", &rename("lines", "Line")) + &block("two", &patch("lines", "File", "a\nb", "c"));
    let windows = parsed(&text.replace('\n', "\r\n"));
    let unix = parsed(&text);

    assert_eq!(windows.len(), 2);
    for (windows, unix) in windows.iter().zip(&unix) {
        assert_eq!((&windows.name, &windows.patch), (&unix.name, &unix.patch));
    }
}

#[test]
fn spans_point_into_the_whole_file() {
    let text = "// é\r\n".to_string() + &block("é", FIELDS) + &block("two", &rename("lines", "Line"));
    let found = parsed(&text);

    let marked = |span: &miette::SourceSpan| &text[span.offset()..span.offset() + span.len()];
    let spans = &found[1].source.spans;

    assert_eq!(found[1].source.text, text);
    assert_eq!(marked(&spans.fields["name"]), "\"two\"");
    assert_eq!(marked(&spans.fields["file"]), "\"**/*.txt\"");
    assert_eq!(marked(&spans.actions.replacers[0].fields["from"]), r#"r"old (\w+)""#);
}

#[test]
fn loose_fields_beside_blocks() {
    let text = format!("enabled true\n{}file \"*\"\n", block("one", FIELDS));

    assert_eq!(errors(&text), vec![
        pair("'enabled' is outside of the file's patches", "enabled"),
        pair("'file' is outside of the file's patches", "file")
    ]);
}

#[test]
fn names_are_needed() {
    assert_eq!(error(&block("one", FIELDS).replace("\"one\"", "")), pair("the patch has no name", "patch"));
    assert_eq!(error(&block("", FIELDS)), pair("the patch's name is empty", "\"\""));
    assert_eq!(error(&block("  ", FIELDS)), pair("the patch's name is empty", "\"  \""));
    assert_eq!(error(&block("one", FIELDS).replace("\"one\"", "\"one\" \"two\"")), pair("a patch takes its name only", "\"two\""));
    assert_eq!(error(&block("one", FIELDS).replace("\"one\"", "\"one\" file=\"*\"")), pair("a patch takes its name only", "file"));
    assert_eq!(error(&block("one", FIELDS).replace("\"one\"", "1")).1, "1");
}

#[test]
fn names_are_used_once_in_a_file() {
    let text = block("one", FIELDS) + &block("two", FIELDS) + &block("one", FIELDS);

    assert_eq!(error(&text), pair("the file already has a patch named 'one'", "\"one\""));
}

#[test]
fn blocks_hold_no_blocks() {
    let (message, marked) = error(&block("one", &(FIELDS.to_string() + &block("two", FIELDS))));

    assert_eq!(message, "unexpected node `patch`");
    assert!(marked.starts_with("patch \"two\""));
}

#[test]
fn missing_fields_point_at_their_block() {
    let text = block("one", FIELDS) + &block("two", &FIELDS.replace("file \"*\"\n", "")) + "patch \"three\"\n";

    assert_eq!(errors(&text), vec![
        pair("child node `file` is required", "patch"),
        pair("child node `file` is required", "patch")
    ]);
}

#[test]
fn every_block_is_reported() {
    let text = block("one", &FIELDS.replace("true", "1")) + &block("two", &FIELDS.replace("\"*\"", "true"));
    let marked: Vec<String> = errors(&text).into_iter().map(|(_, marked)| marked).collect();

    assert_eq!(marked, vec!["1", "true"]);
}

#[test]
fn empty_files_have_no_patch() {
    assert_eq!(error(""), pair("child node `file` is required", ""));
}

#[test]
fn one_patch_is_wanted() {
    let several = block("one", FIELDS) + &block("two", FIELDS);
    let refused = "Expected one patch, several.kdl has several: one, two";

    assert_eq!(Patch::from_kdl("several.kdl", &several).unwrap_err().to_string(), refused);
    assert_eq!(Patch::parse("several.kdl", &several).unwrap_err().to_string(), refused);
    assert_eq!(Patch::from_kdl("one.kdl", &block("one", FIELDS)).unwrap(), Patch::from_kdl("", FIELDS).unwrap());
}

#[test]
fn projects_list_patches_in_order() {
    let directory = workspace("replace", &[
        ("a/b.kdl", FIELDS.to_string()),
        ("b.kdl", block("z", FIELDS) + &block("y", FIELDS)),
        ("a.kdl", FIELDS.to_string())
    ]);
    let project = project(&directory).unwrap();
    let names: Vec<&str> = project.patches.iter().map(|patch| patch.name.as_str()).collect();

    assert_eq!(names, vec!["a.kdl", "a/b.kdl", "z", "y"]);
    assert!(project.patch("y").unwrap().path.ends_with("patches/b.kdl"));
    assert!(project.patch("b.kdl").is_none());
}

#[test]
fn names_are_used_once_in_a_project() {
    let directory = workspace("replace", &[("a.kdl", block("one", FIELDS)), ("sub/b.kdl", block("two", FIELDS) + &block("one", FIELDS))]);
    assert_eq!(project(&directory).err().unwrap().to_string(), "Patch 'one' is in both a.kdl and sub/b.kdl");

    let directory = workspace("replace", &[("a.kdl", block("b.kdl", FIELDS)), ("b.kdl", FIELDS.to_string())]);
    assert_eq!(project(&directory).err().unwrap().to_string(), "Patch 'b.kdl' is in both a.kdl and b.kdl");
}

#[test]
fn patches_run_as_written() {
    let first = patch("lines", "Line", "old one", "middle");
    let second = patch("lines", "Line", "middle", "new");

    let directory = workspace("replace", &[("both.kdl", block("z", &first) + &block("a", &second))]);
    assert_eq!(patched(&directory), vec!["z", "a"]);
    assert_eq!(fs::read_to_string(directory.path().join("target/a.txt")).unwrap(), "new\nkeep\n");

    let directory = workspace("replace", &[("both.kdl", block("a", &second) + &block("z", &first))]);
    assert_eq!(patched(&directory), vec!["z"]);
    assert_eq!(fs::read_to_string(directory.path().join("target/a.txt")).unwrap(), "middle\nkeep\n");
}

#[test]
fn one_file_runs_like_several() {
    let first = rename("lines", "Line");
    let second = patch("lines", "File", "keep", "kept");
    let apart = workspace("replace", &[("a.kdl", first.clone()), ("b.kdl", second.clone())]);
    let together = workspace("replace", &[("c.kdl", block("a.kdl", &first) + &block("b.kdl", &second))]);

    assert_eq!(run(&apart).unwrap(), run(&together).unwrap());
    for file in ["target/a.txt", "target/nested/b.txt", "target/c.md"] {
        assert_eq!(fs::read(apart.path().join(file)).unwrap(), fs::read(together.path().join(file)).unwrap());
    }
}

#[test]
fn disabled_blocks_do_not_run() {
    let disabled = rename("lines", "Line").replace("enabled true", "enabled false");
    let directory = workspace("replace", &[("both.kdl", block("off", &disabled) + &block("on", &patch("lines", "Line", "keep", "kept")))]);

    assert_eq!(run(&directory).unwrap()[0], report("a.txt", Outcome::Patched(vec!["on".to_string()])));
}

#[test]
fn checks_name_the_patch_and_its_place() {
    let broken = rename("lines", "Line").replace("new $[0]", "#[missing]");
    let text = block("fine", &rename("lines", "Line")) + &block("broken", &broken);
    let directory = workspace("replace", &[("both.kdl", text.clone())]);
    let operation = Operation::from_file(&directory.path().join("configuration.kdl")).unwrap();
    let found = operation.check(&registry()).unwrap();

    assert_eq!(found.len(), 1);
    assert_eq!((found[0].0.as_str(), found[0].1.kind), ("broken", DiagnosticKind::ActionNotFound));
    assert!(found[0].1.span.unwrap().offset() > text.find("\"broken\"").unwrap());

    let error = format!("{:?}", run(&directory).err().unwrap());
    assert!(error.contains("Patch broken: "), "{error}");
}
