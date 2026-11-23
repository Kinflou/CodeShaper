// Standard Uses

// Crate Uses
use crate::project;

// External Uses
use codeshaper_lsp::completion::completion;
use codeshaper_lsp::diagnostics::diagnostics;
use codeshaper_lsp::hover::hover;
use codeshaper_lsp::trace::Traces;


const PATCH: &str = r##"alias "cpp"
actions {
    makers {
        maker "pair" {
            make "@[0]=@[1]"
        }
    }
    replacers {
        replacer {
            location "FunctionDefinition.body"
            reference-location "Nope"
            to "#[pair](a, b)"
        }
    }
}
"##;

const TRACED: &str = r##"enabled false
alias "cpp"
file "*.cpp"

actions {
    builders {
        builder "names" {
            location "FunctionDefinition"
            match r"void (\w+)"
            build "$[0]"
            collect true
            separator ", "
        }
    }
    replacers {
        replacer {
            location "FunctionDefinition"
            from r"void (\w+)\(\)"
            to "int $[0]() /* #[names] */"
            flags "i"
        }
    }
}
"##;

const INDEXED: &str = r##"enabled false
alias "cpp"
file "*.cpp"

actions {
    makers {
        maker "pair" {
            prepare r"(\w+)=(\w+)"
            make "@[0]_@[1]"
        }
    }
    switchers {
        switch "kind" {
            cases old="legacy" keep="current" gone="removed"
            index "@[0]"
        }
    }
    lists {
        list "order" {
            values "first" "second"
            index "@[0]"
        }
    }
    replacers {
        replacer {
            location "FunctionDefinition"
            from r"void (\w+)\(\)"
            to "void #[pair]($[0], #[kind]($[0]))_#[order](\"1\")()"
        }
    }
}
"##;

fn hovered(patch: &str, needle: &str, shift: usize) -> (String, String) {
    let (_directory, project) = crate::preview::operation();
    let offset = patch.find(needle).unwrap() + shift;
    let (text, range) = hover(&codeshaper::registry(), &project, &mut Traces::default(), patch, offset).unwrap();

    (text, patch[range].to_string())
}

fn traced(needle: &str, shift: usize) -> (String, String) {
    hovered(TRACED, needle, shift)
}

fn at(needle: &str, shift: usize) -> Option<(String, String)> {
    let (_directory, project) = project(&[]);
    let offset = PATCH.find(needle).unwrap() + shift;

    hover(&codeshaper::registry(), &project, &mut Traces::default(), PATCH, offset).map(|(text, range)| (text, PATCH[range].to_string()))
}


#[test]
fn actions_show_their_call_sites() {
    let (text, range) = at("#[pair]", 3).unwrap();

    assert_eq!(text, "Called from 1 place(s) in this patch");
    assert_eq!(range, "pair");
}

#[test]
fn locations_are_described_by_the_backend() {
    let (text, range) = at("FunctionDefinition", 2).unwrap();
    assert_eq!(text, "cpp: node kind `function_definition` › field `body`");
    assert_eq!(range, "\"FunctionDefinition.body\"");

    let (text, _) = at("Nope", 1).unwrap();
    assert!(text.starts_with("Not a cpp location: "), "{text}");
}

#[test]
fn other_values_have_no_hover() {
    assert_eq!(at("@[0]=", 1), None);
}

#[test]
fn locations_count_matches_in_target_files() {
    let (_directory, project) = crate::preview::operation();
    let text = crate::preview::RENAME;
    let offset = text.find("FunctionDefinition").unwrap() + 2;

    let (hovered, _) = hover(&codeshaper::registry(), &project, &mut Traces::default(), text, offset).unwrap();

    assert_eq!(hovered, "cpp: node kind `function_definition`\n\nMatches 2 place(s) in 2 of 2 target file(s)");
}

#[test]
fn replacements_show_samples_as_diffs() {
    let (text, range) = traced("int $[0]", 1);

    assert_eq!(range, "\"int $[0]() /* #[names] */\"");
    assert_eq!(text, "**2 replacement(s)** across 2 target file(s)\n\n\
        `a.cpp`\n```diff\n- void old()\n+ int old() /* old */\n```\n\n\
        `b.cpp`\n```diff\n- void keep()\n+ int keep() /* keep */\n```");
}

#[test]
fn from_shows_groups_and_matches() {
    let (text, _) = traced(r"void (\w+)\(", 1);

    assert_eq!(text, "- `$[0]` = `(\\w+)`\n\n\
        Matches in 2 of 2 located content(s) across 2 target file(s), 2 match(es) in all\n\n\
        First match in `a.cpp`: `void old()`\n- `$[0]` = `old`");
}

#[test]
fn locals_show_their_group_and_values() {
    let (text, range) = traced("$[0]()", 2);

    assert_eq!(range, "$[0]");
    assert_eq!(text, "`$[0]` is group 1 of `from`: `(\\w+)`\n\nSeen as `old`, `keep`");
}

#[test]
fn flags_are_listed() {
    let (text, _) = traced("\"i\"", 1);

    assert!(text.contains("- ✓ `i` case insensitive"), "{text}");
    assert!(text.contains("- · `m`"), "{text}");
}

#[test]
fn called_actions_show_a_result() {
    let (text, range) = traced("#[names]", 3);
    assert_eq!(range, "names");

    assert!(text.starts_with("Called from 1 place(s) in this patch\n\n"), "{text}");
    assert!(text.ends_with("Called 2 time(s) across 2 target file(s)\n\n`a.cpp`, without arguments:\n```cpp\nold\n```\n\n\
        `b.cpp`, without arguments:\n```cpp\nkeep\n```"), "{text}");
}

#[test]
fn builder_match_and_build_show_inputs() {
    let (text, _) = traced(r#"void (\w+)""#, 1);
    assert!(text.starts_with("- `$[0]` = `(\\w+)`\n\nMatches in 2 of 2 input(s) across 2 target file(s)"), "{text}");

    let (text, _) = traced("\"$[0]\"", 0);
    assert_eq!(text, "**Built from 2 input(s)** across 2 target file(s)\n\n\
        `a.cpp`: `void old() {}`\n```cpp\nold\n```\n\n\
        `b.cpp`: `void keep() {}`\n```cpp\nkeep\n```");
}

#[test]
fn collect_and_separator_show_the_joined_result() {
    let (text, range) = traced("true", 1);

    assert_eq!(range, "true");
    assert_eq!(text, "Collects every input of its location, joined by `, `\n\nFirst result in `a.cpp`:\n```cpp\nold\n```");

    let (separator, _) = traced("\", \"", 1);
    assert_eq!(separator, text);
}

#[test]
fn makers_show_results_with_their_arguments() {
    let (text, _) = hovered(INDEXED, "@[0]_", 1);
    assert_eq!(text, "Called 2 time(s) across 2 target file(s)\n\n\
        `a.cpp`, with `old`, `legacy`:\n```cpp\nold_legacy\n```\n\n\
        `b.cpp`, with `keep`, `current`:\n```cpp\nkeep_current\n```");

    let (text, _) = hovered(INDEXED, "(\\w+)=", 1);
    assert_eq!(text, "- `$[0]` = `(\\w+)`\n- `$[1]` = `(\\w+)`");
}

#[test]
fn indexers_mark_the_choices_used() {
    let (text, range) = hovered(INDEXED, "legacy", 1);
    assert_eq!(range, "\"legacy\"");
    assert!(text.starts_with("- ✓ `old` → `legacy`\n- ✓ `keep` → `current`\n- · `gone` → `removed`\n\n\
        1 of 3 choice(s) never used across 2 target file(s)\n\nCalled 2 time(s)"), "{text}");

    let (text, _) = hovered(INDEXED, "second", 1);
    assert!(text.starts_with("- · `0` → `first`\n- ✓ `1` → `second`\n\n1 of 2 choice(s) never used"), "{text}");
}

#[test]
fn patch_fields_show_files_and_backend() {
    let (text, _) = hovered(INDEXED, "*.cpp", 1);
    assert!(text.starts_with("Matches 2 target file(s)\n\n- [`a.cpp`](<file:///"), "{text}");
    assert!(text.contains("/target/a.cpp>)\n- [`b.cpp`](<file:///"), "{text}");

    let (text, _) = hovered(INDEXED, "\"cpp\"", 1);
    assert!(text.starts_with("Language backend `cpp`, chosen by default for ") && text.contains("`.cpp`"), "{text}");

    let (text, _) = hovered(&INDEXED.replace("\"cpp\"", "\"cobol\""), "cobol", 1);
    assert_eq!(text, "No language backend named `cobol` is registered");
}

#[test]
fn broken_patches_show_no_samples() {
    let (_directory, project) = crate::preview::operation();
    let broken = TRACED.replace("build \"$[0]\"", "build \"$[0\"");
    let offset = broken.find("build").unwrap() + 7;

    assert_eq!(hover(&codeshaper::registry(), &project, &mut Traces::default(), &broken, offset), None);
}

#[test]
fn half_typed_patches_never_panic() {
    let (_directory, project) = crate::preview::operation();
    let registry = codeshaper::registry();
    let path = project.join("patches/half.kdl");

    for patch in [TRACED, INDEXED] {
        for (offset, removed) in patch.char_indices().step_by(23) {
            let text = format!("{}{}", &patch[..offset], &patch[offset + removed.len_utf8()..]);

            diagnostics(&registry, &path, &text);
            completion(&registry, &project, &text, offset);
            hover(&registry, &project, &mut Traces::default(), &text, offset);
            Traces::default().get(&registry, &project, &text);
        }
    }
}
