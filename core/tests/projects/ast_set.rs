// Standard Uses
use std::path::PathBuf;

// Crate Uses

// External Uses
use codeshaper_core::shaping::project::ast_set::AstSet;
use codeshaper_core::shaping::project::settings::from_kdl;
use codeshaper_core::target::File;


fn decoded(ast_set: &str) -> Result<AstSet, String> {
    let content = format!("name \"A\"\ntarget \"text\"\n{ast_set}\ndescription \"\"\n");

    from_kdl(&content)
        .map(|settings| settings.ast_set)
        .map_err(|report| report.related().into_iter().flatten().map(|related| {
            let label = related.labels().into_iter().flatten().next().map(|label| content[label.offset()..label.offset() + label.len()].to_string());
            format!("{} | {related}", label.unwrap_or_default())
        }).collect::<Vec<_>>().join("\n"))
}

fn file(path: &str, language: Option<&str>) -> File {
    File { path: PathBuf::from(path), language: language.map(str::to_string) }
}


#[test]
fn one_language_covers_every_file() {
    let ast_set = decoded(r#"ast-set "cpp""#).unwrap();

    assert_eq!(ast_set, AstSet::Language("cpp".to_string()));
    assert_eq!(ast_set.language(&file("a.txt", Some("other"))), Some("cpp"));
}

#[test]
fn the_target_names_languages() {
    let ast_set = decoded(r#"ast-set "target""#).unwrap();

    assert_eq!(ast_set, AstSet::Target);
    assert_eq!(ast_set.language(&file("a.cpp", Some("gdscript"))), Some("gdscript"));
    assert_eq!(ast_set.language(&file("a.cpp", None)), None);
}

#[test]
fn later_lines_win() {
    for block in ["ast-set {\n    cpp \"*.cpp\" \"*.h\"\n    c \"legacy/**/*.h\" \"*.c\"\n}", "ast-set { cpp \"*.cpp\" \"*.h\"; \"c\" \"legacy/**/*.h\" \"*.c\"; }"] {
        for block in [block.to_string(), block.replace('\n', "\r\n")] {
            let ast_set = decoded(&block).unwrap();

            assert_eq!(ast_set.names(), vec!["cpp", "c"]);
            assert_eq!(ast_set.language(&file("src/a.cpp", None)), Some("cpp"));
            assert_eq!(ast_set.language(&file("src/a.h", None)), Some("cpp"));
            assert_eq!(ast_set.language(&file("legacy/old/a.h", None)), Some("c"));
            assert_eq!(ast_set.language(&file("a.rs", Some("rust"))), None);
        }
    }
}

#[test]
fn mistakes_are_pointed_at() {
    for (ast_set, expected) in [
        ("ast-set \"\"", "\"\" | 'ast-set' is empty"),
        ("ast-set \"cpp\" \"c\"", "\"c\" | 'ast-set' takes one language"),
        ("ast-set \"cpp\" {\n    c \"*.c\"\n}", "\"cpp\" | 'ast-set' takes either"),
        ("ast-set language=\"cpp\"", "language | 'ast-set' takes no properties"),
        ("ast-set {\n    cpp \"[\"\n}", "\"[\" |"),
        ("ast-set {\n    cpp 1\n}", "1 | expected text"),
        ("ast-set {\n    cpp {\n        a\n    }\n}", "cpp | a language takes file patterns on its line"),
    ] {
        let error = decoded(ast_set).unwrap_err();
        assert!(error.contains(expected), "{ast_set}: {error}");
    }

    for ast_set in ["ast-set", "ast-set {\n}", "ast-set {\n    cpp\n}", ""] {
        assert!(decoded(ast_set).is_err(), "{ast_set}");
    }
}
