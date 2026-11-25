// Standard Uses

// Crate Uses
use crate::project;

// External Uses
use codeshaper_lsp::completion::completion;
use codeshaper_lsp::diagnostics::diagnostics;
use codeshaper_lsp::hover::hover;
use codeshaper_lsp::trace::{Limits, Traces};


const PATCH: &str = r##"alias ""
actions {
    maker "pair" {
        make "@[0]=@[1]"
    }
    replacer {
        location "function_definition.body"
        reference-location "Nope"
        to "#[pair](a, b)"
    }
}
"##;

const TRACED: &str = r##"enabled false
alias ""
file "*.cpp"

actions {
    builder "names" {
        location "function_definition"
        match r"void (\w+)"
        build "$[0]"
        collect true
        separator ", "
    }
    replacer {
        location "function_definition"
        from r"void (\w+)\(\)"
        to "int $[0]() /* #[names] */"
        flags "i"
    }
}
"##;

const INDEXED: &str = r##"enabled false
alias ""
file "*.cpp"

actions {
    maker "pair" {
        prepare r"(\w+)=(\w+)"
        make "@[0]_@[1]"
    }
    switch "kind" {
        cases old="legacy" keep="current" gone="removed"
        index "@[0]"
    }
    list "order" {
        values "first" "second"
        index "@[0]"
    }
    replacer {
        location "function_definition"
        from r"void (\w+)\(\)"
        to "void #[pair($[0], #[kind($[0])])]_#[order(\"1\")]()"
    }
}
"##;

fn hovered(patch: &str, needle: &str, shift: usize) -> (String, String) {
    let (_directory, project) = crate::preview::operation();
    let offset = patch.find(needle).unwrap() + shift;
    let (text, range) = hover(&codeshaper::registry(), &project, &mut Traces::default(), patch, offset).unwrap();

    (plain(&text), patch[range].to_string())
}

fn plain(text: &str) -> String {
    text.split("<file:").enumerate()
        .map(|(index, part)| match part.split_once("/target/").filter(|_| index > 0 && part.contains('#')) {
            Some((_, rest)) => format!("<{rest}"),
            None if index > 0 => format!("<file:{part}"),
            None => part.to_string()
        })
        .collect()
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
    let (text, range) = at("function_definition", 2).unwrap();
    assert_eq!(text, "cpp: node kind `function_definition` › field `body`");
    assert_eq!(range, "\"function_definition.body\"");

    let (text, _) = at("Nope", 1).unwrap();
    assert!(text.starts_with("Not a cpp location: "), "{text}");
}

#[test]
fn other_values_have_no_hover() {
    assert_eq!(at("\"@[0]=", 0), None);
}

#[test]
fn locations_count_matches_in_target_files() {
    let (_directory, project) = crate::preview::operation();
    let text = crate::preview::RENAME;
    let offset = text.find("function_definition").unwrap() + 2;

    let (hovered, _) = hover(&codeshaper::registry(), &project, &mut Traces::default(), text, offset).unwrap();

    assert_eq!(plain(&hovered), "cpp: node kind `function_definition`\n\n\
        **Places**\n\nIn 2 of 2 target file(s), 2 in all\n\n\
        [`a.cpp:1:1-14`](<a.cpp#L1,1-L1,14>)\n```cpp\nvoid old() {}\n```\n\n\
        [`b.cpp:1:1-15`](<b.cpp#L1,1-L1,15>)\n```cpp\nvoid keep() {}\n```");
}

#[test]
fn replacements_show_samples_as_diffs() {
    let (text, range) = traced("int $[0]", 1);

    assert_eq!(range, "\"int $[0]() /* #[names] */\"");
    assert_eq!(text, "**2 replacement(s)** across 2 target file(s)\n\n\
        [`a.cpp:1:1-11`](<a.cpp#L1,1-L1,11>)\n```diff\n- void old()\n+ int old() /* old */\n```\n\n\
        [`b.cpp:1:1-12`](<b.cpp#L1,1-L1,12>)\n```diff\n- void keep()\n+ int keep() /* keep */\n```");
}

#[test]
fn from_shows_groups_and_matches() {
    let (text, _) = traced(r"void (\w+)\(", 1);

    assert_eq!(text, "**Captures**\n\n- `$[0]` = `(\\w+)`\n\n\
        **Matches**\n\nIn 2 of 2 located content(s) across 2 target file(s), 2 in all\n\n\
        [`a.cpp:1:1-11`](<a.cpp#L1,1-L1,11>)\n```cpp\nvoid old()\n```\n- `$[0]` = `old` [`1:6-9`](<a.cpp#L1,6-L1,9>)\n\n\
        [`b.cpp:1:1-12`](<b.cpp#L1,1-L1,12>)\n```cpp\nvoid keep()\n```\n- `$[0]` = `keep` [`1:6-10`](<b.cpp#L1,6-L1,10>)");
}

#[test]
fn locals_show_their_group_and_values() {
    let (text, range) = traced("$[0]()", 2);

    assert_eq!(range, "$[0]");
    assert_eq!(text, "`$[0]` is group 1 of `from`: `(\\w+)`\n\n\
        Seen as [`old`](<a.cpp#L1,6-L1,9>), [`keep`](<b.cpp#L1,6-L1,10>)");
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
    assert_eq!(text, "**Captures**\n\n- `$[0]` = `(\\w+)`\n\n\
        **Matches**\n\nIn 2 of 2 input(s) across 2 target file(s), 2 in all\n\n\
        [`a.cpp:1:1-9`](<a.cpp#L1,1-L1,9>)\n```cpp\nvoid old\n```\n- `$[0]` = `old` [`1:6-9`](<a.cpp#L1,6-L1,9>)\n\n\
        [`b.cpp:1:1-10`](<b.cpp#L1,1-L1,10>)\n```cpp\nvoid keep\n```\n- `$[0]` = `keep` [`1:6-10`](<b.cpp#L1,6-L1,10>)");

    let (text, _) = traced("\"$[0]\"", 0);
    assert_eq!(text, "**Built from 2 input(s)** across 2 target file(s)\n\n\
        [`a.cpp:1:1-14`](<a.cpp#L1,1-L1,14>)\n```cpp\nvoid old() {}\n```\n\nBuilds\n```cpp\nold\n```\n\n\
        [`b.cpp:1:1-15`](<b.cpp#L1,1-L1,15>)\n```cpp\nvoid keep() {}\n```\n\nBuilds\n```cpp\nkeep\n```");
}

#[test]
fn collect_and_separator_show_the_joined_result() {
    let (text, range) = traced("true", 1);

    assert_eq!(range, "true");
    assert_eq!(text, "Collects every input of its location, joined by `, `\n\nFirst result in `a.cpp`:\n```cpp\nold\n```");

    let (separator, _) = traced("\", \"", 1);
    assert_eq!(separator, text);

    let (_directory, project) = crate::preview::operation();
    let referring = TRACED.replace("collect true", "collect true\n        reference-location \"identifier\"");
    let offset = referring.find("collect true").unwrap() + 9;
    let (text, _) = hover(&codeshaper::registry(), &project, &mut Traces::default(), &referring, offset).unwrap();
    assert!(text.starts_with("Collects every reference location inside the place of its location, joined by `, `"), "{text}");
}

#[test]
fn makers_show_results_with_their_arguments() {
    let (text, _) = hovered(INDEXED, "\"@[0]_", 0);
    assert_eq!(text, "Called 2 time(s) across 2 target file(s)\n\n\
        `a.cpp`, with `old`, `legacy`:\n```cpp\nold_legacy\n```\n\n\
        `b.cpp`, with `keep`, `current`:\n```cpp\nkeep_current\n```");

    let (text, _) = hovered(INDEXED, "(\\w+)=", 1);
    assert_eq!(text, "**Captures**\n\n- `$[0]` = `(\\w+)`\n- `$[1]` = `(\\w+)`\n\n\
        **Matches**\n\nIn 0 of 2 first argument(s) across 2 target file(s), 0 in all");
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

    let (text, _) = hovered(&INDEXED.replace("alias \"\"", "alias \"cpp\""), "\"cpp\"", 1);
    assert_eq!(text, "Language backend `cpp`, used instead of the project's `ast-set`");

    let (text, _) = hovered(&INDEXED.replace("alias \"\"", "alias \"cobol\""), "cobol", 1);
    assert!(text.starts_with("No language backend named `cobol` is registered, known languages are `cpp`"), "{text}");
}

#[test]
fn broken_patches_show_no_samples() {
    let (_directory, project) = crate::preview::operation();
    let broken = TRACED.replace("build \"$[0]\"", "build \"$[0\"");
    let offset = broken.find("match r").unwrap() + 8;

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

#[test]
fn named_groups_are_shown() {
    let named = TRACED.replace(r"void (\w+)\(\)", r"void (?P<name>\w+)\(\)").replace("int $[0]", "int $[name]");

    assert!(hovered(&named, r"void (?P", 1).0.starts_with("**Captures**\n\n- `$[0]`, `$[name]` = `(?P<name>\\w+)`"));
    assert_eq!(hovered(&named, "$[name]", 2).0, "`$[name]` is group 1 of `from`: `(?P<name>\\w+)`\n\n\
        Seen as [`old`](<a.cpp#L1,6-L1,9>), [`keep`](<b.cpp#L1,6-L1,10>)");
}

#[test]
fn matches_are_shown_without_groups() {
    let plain = TRACED.replace(r"void (\w+)\(\)", "void old").replace("int $[0]", "int x");
    let (text, _) = hovered(&plain, "void old", 1);

    assert!(text.ends_with("**Matches**\n\nIn 1 of 2 located content(s) across 2 target file(s), 1 in all\n\n[`a.cpp:1:1-9`](<a.cpp#L1,1-L1,9>)\n```cpp\nvoid old\n```"), "{text}");
}

#[test]
fn first_arguments_have_no_place() {
    let matching = INDEXED.replace(r"(\w+)=(\w+)", r"^(\w)(\d)?");
    let (text, _) = hovered(&matching, r"^(\w)", 1);

    assert!(text.ends_with("2 in all\n\n\
        `a.cpp`\n```cpp\no\n```\n- `$[0]` = `o`\n- `$[1]` = unset\n\n\
        `b.cpp`\n```cpp\nk\n```\n- `$[0]` = `k`\n- `$[1]` = unset"), "{text}");
}

#[test]
fn earlier_replacers_leave_no_place() {
    let earlier = TRACED.replace("    replacer {\n", r##"    replacer {
        location "function_definition"
        from "old"
        to "renamed"
    }
    replacer {
"##);
    let (text, _) = hovered(&earlier, r"void (\w+)\(", 1);

    assert!(text.ends_with("\n\n\
        `a.cpp`, changed by an earlier replacer\n```cpp\nvoid renamed()\n```\n- `$[0]` = `renamed`\n\n\
        [`b.cpp:1:1-12`](<b.cpp#L1,1-L1,12>)\n```cpp\nvoid keep()\n```\n- `$[0]` = `keep` [`1:6-10`](<b.cpp#L1,6-L1,10>)"), "{text}");
}

#[test]
fn places_follow_the_saved_file() {
    let (directory, project) = crate::preview::operation();
    let offset = TRACED.find(r#"void (\w+)""#).unwrap() + 1;
    let hovered = || hover(&codeshaper::registry(), &project, &mut Traces::default(), TRACED, offset).unwrap().0;

    std::fs::write(directory.path().join("target/a.cpp"), "// é𝄞\r\n/* é𝄞 */ void\r\nold()\r\n{}\r\n").unwrap();
    std::fs::write(directory.path().join("target/b (1).cpp"), "").unwrap();
    let spread = TRACED.replace(r"void (\w+)\(\)", r"void\s(\w+)").replace(r#"void (\w+)""#, r#"void\s(\w+)\(\)\n""#);
    let offset_spread = spread.find(r"void\s(\w+)\(\)\n").unwrap() + 1;
    let text = hover(&codeshaper::registry(), &project, &mut Traces::default(), &spread, offset_spread).unwrap().0;
    let text = plain(&text);

    assert!(text.contains("[`a.cpp:2:11-4:1`](<a.cpp#L2,11-L4,1>)\n```cpp\nvoid\nold()\n```\n- `$[0]` = `old` [`3:1-4`](<a.cpp#L3,1-L3,4>)"), "{text}");

    std::fs::write(directory.path().join("target/b (1).cpp"), "\n  void other() {}\n").unwrap();
    assert!(hovered().contains("/target/b%20(1).cpp#L2,3-L2,13>)\n```cpp\nvoid other\n```"), "{}", hovered());
}

#[test]
fn empty_matches_have_a_place() {
    let empty = TRACED.replace(r#"void (\w+)""#, r#"(x?)""#);
    let (text, _) = hovered(&empty, "(x?)", 1);

    assert!(text.contains("[`a.cpp:1:1-1`](<a.cpp#L1,1-L1,1>): empty\n- `$[0]` = empty [`1:1-1`](<a.cpp#L1,1-L1,1>)"), "{text}");
}

#[test]
fn captures_over_several_lines_are_snippets() {
    let (directory, project) = crate::preview::operation();
    std::fs::write(directory.path().join("target/a.cpp"), "void old()\r\n{\r\n\treturn;\r\n}\r\n").unwrap();
    let spread = TRACED.replace(r#"void (\w+)""#, r#"(?s)void (\w+)\(\)\s(.*)""#);
    let offset = spread.find("(?s)").unwrap() + 1;
    let (text, _) = hover(&codeshaper::registry(), &project, &mut Traces::default(), &spread, offset).unwrap();

    assert!(plain(&text).contains("[`a.cpp:1:1-4:2`](<a.cpp#L1,1-L4,2>)\n\
        ```cpp\nvoid old()\n{\n\treturn;\n}\n```\n\
        - `$[0]` = `old` [`1:6-9`](<a.cpp#L1,6-L1,9>)\n\
        - `$[1]` [`2:1-4:2`](<a.cpp#L2,1-L4,2>)\n  ```cpp\n  {\n  \treturn;\n  }\n  ```"), "{text}");
}

#[test]
fn places_are_the_same_in_patch_blocks() {
    let block = format!("patch \"first\" {{\n{TRACED}}}\npatch \"second\" {{\n{TRACED}}}\n");
    let offset = block.rfind(r#"void (\w+)""#).unwrap() + 1;
    let (_directory, project) = crate::preview::operation();
    let (text, _) = hover(&codeshaper::registry(), &project, &mut Traces::default(), &block, offset).unwrap();

    assert_eq!(plain(&text), traced(r#"void (\w+)""#, 1).0);
}

#[test]
fn builtins_are_explained() {
    let patch = INDEXED.replace("gone=\"removed\"", "gone=\"#[case.upper('a')]#[file.folder]#[file.pth]\"");

    let (text, range) = hovered(&patch, "case.upper", 6);
    assert_eq!(range, "case.upper");
    assert_eq!(text, "**case.upper** · built-in function (text)\n\nThe text in capitals.\n\n```kdl\nto \"#[case.upper($[0])]\"\n```");

    let (text, range) = hovered(&patch, "file.folder", 1);
    assert_eq!(range, "file.folder");
    assert!(text.starts_with("**file.folder** · built-in\n\n"), "{text}");

    let (_directory, project) = crate::preview::operation();
    assert!(hover(&codeshaper::registry(), &project, &mut Traces::default(), &patch, patch.find("file.pth").unwrap() + 2).is_none());
}

#[test]
fn arguments_are_shown_in_choices() {
    let patch = INDEXED.replace("gone=\"removed\"", "gone=\"@[0]#[pair('a', 'b')]\"").replace("\"first\" \"second\"", "\"first\" \"@[0]\"");

    for (needle, name) in [("\"@[0]#", "kind"), ("\"first\" \"@[0]", "order")] {
        let shift = needle.rfind("@[").unwrap() + 2;
        let (text, range) = hovered(&patch, needle, shift);

        assert!(text.starts_with(&format!("`@[0]` is argument 1 of `#[{name}]`")), "{text}");
        assert_eq!(range, "@[0]");
    }

    let (text, range) = hovered(&patch, "@[0]#[pair", 7);
    assert!(text.starts_with("Called from 2 place(s)"), "{text}");
    assert_eq!(range, "pair");
}

#[test]
fn arguments_show_the_values_passed() {
    let (text, range) = hovered(INDEXED, "@[1]", 2);
    assert_eq!(range, "@[1]");
    assert_eq!(text, "`@[1]` is argument 2 of `#[pair]`\n\nPassed in 2 of 2 call(s)\n\nSeen as `legacy`, `current`");

    let switch = INDEXED.find("index").unwrap();
    let (text, _) = hovered(INDEXED, &INDEXED[switch..switch + 12], 8);
    assert_eq!(text, "`@[0]` is argument 1 of `#[kind]`\n\nPassed in 2 of 2 call(s)\n\nSeen as `old`, `keep`");

    let more = INDEXED.replace("make \"@[0]_@[1]\"", "make \"@[0]_@[1]\" ").replace("@[1]\"", "@[7]\"");
    assert_eq!(hovered(&more, "@[7]", 2).0, "`@[7]` is argument 8 of `#[pair]`");
}

#[test]
fn replacer_names_show_their_replacements() {
    let named = TRACED.replace("    replacer {", "    replacer \"returns\" {");
    let (text, range) = hovered(&named, "returns", 2);

    assert_eq!(range, "returns");
    assert_eq!(text, hovered(&named, "int $[0]", 1).0);
    assert!(text.starts_with("**2 replacement(s)** across 2 target file(s)\n\n[`a.cpp:1:1-11`]"), "{text}");
    assert_eq!(text, traced("int $[0]", 1).0);
    assert_eq!(hovered(&named, "$[0]()", 2).0, traced("$[0]()", 2).0);
}

fn placed(patch: &str, location: &str, needle: &str, files: &[(&str, &str)], limits: Limits) -> String {
    let (directory, project) = crate::preview::operation();
    for (file, content) in files {
        std::fs::write(directory.path().join("target").join(file), content).unwrap();
    }
    let text = patch.replace("LOCATION", location);
    let offset = text.find(needle).unwrap_or_else(|| panic!("{needle}")) + 1;

    plain(&hover(&codeshaper::registry(), &project, &mut Traces::new(limits), &text, offset).unwrap().0)
}

const PLACED: &str = "enabled true\nfile \"*.cpp\"\nactions {\n    replacer {\n        location LOCATION\n        from \"a\"\n        to \"b\"\n    }\n}\n";
const BODY: &str = "// é𝄞\r\nvoid one() {\r\n\tint 𝄞a;\r\n}\r\n";

#[test]
fn places_cover_several_lines() {
    let text = placed(PLACED, "\"function_definition.body\"", "function_definition", &[("a.cpp", BODY)], Limits::default());

    assert_eq!(text, "cpp: node kind `function_definition` › field `body`\n\n**Places**\n\nIn 2 of 2 target file(s), 2 in all\n\n\
        [`a.cpp:2:12-4:2`](<a.cpp#L2,12-L4,2>)\n```cpp\n{\n\tint 𝄞a;\n}\n```\n\n\
        [`b.cpp:1:13-15`](<b.cpp#L1,13-L1,15>)\n```cpp\n{}\n```");
}

#[test]
fn reference_locations_have_places() {
    let patch = PLACED.replace("from", "reference-location \"function_definition\"\n        from");
    let text = placed(&patch, "\"function_definition.body\"", "function_definition\"", &[], Limits::default());

    assert!(text.contains("**Places**\n\nIn 2 of 2 target file(s), 2 in all\n\n[`a.cpp:1:1-14`]"), "{text}");
}

#[test]
fn more_places_are_counted() {
    let many = ("a.cpp", "void a() {}\nvoid b() {}\nvoid c() {}\nvoid d() {}\n");
    let shown = |samples: usize| placed(PLACED, "\"function_definition\"", "function_definition", &[many], Limits { samples, ..Limits::default() });

    assert!(shown(3).ends_with("void c() {}\n```\n\n+2 more, Find All References lists every place"), "{}", shown(3));
    assert!(shown(1).ends_with("void a() {}\n```\n\n+4 more, Find All References lists every place"), "{}", shown(1));
    assert!(shown(5).ends_with("void keep() {}\n```"), "{}", shown(5));
}

#[test]
fn long_places_are_cut() {
    let limits = Limits { lines: 2, ..Limits::default() };

    assert!(placed(PLACED, "\"function_definition\"", "function_definition", &[("a.cpp", BODY)], limits).contains("```cpp\nvoid one() {\n\tint 𝄞a;\n…\n```"));
}

#[test]
fn locations_may_have_no_place() {
    let text = placed(PLACED, "\"class_specifier\"", "class_specifier", &[], Limits::default());
    assert!(text.ends_with("**Places**\n\nIn 0 of 2 target file(s), 0 in all"), "{text}");

    let text = placed(PLACED, "\"Nope\"", "Nope", &[], Limits::default());
    assert!(text.starts_with("Not a cpp location: ") && !text.contains("Places"), "{text}");
}

#[test]
fn every_word_of_a_query_shows_its_places() {
    let query = "r#\"(function_definition body: (_) @target (#not-eq? @target \"x\"))\"#";
    let places = "\n\n**Places of this location**\n\nIn 2 of 2 target file(s), 2 in all\n\n\
        [`a.cpp:1:12-14`](<a.cpp#L1,12-L1,14>)\n```cpp\n{}\n```\n\n[`b.cpp:1:13-15`](<b.cpp#L1,13-L1,15>)\n```cpp\n{}\n```";
    let shown = |needle: &str| placed(PLACED, query, needle, &[], Limits::default());

    assert_eq!(shown("function_def"), format!("`function_definition`: cpp node kind\n\nMatches 2 place(s) in 2 of 2 target file(s){places}"));
    assert_eq!(shown("body"), format!("`body`: cpp field{places}"));
    assert_eq!(shown("_) @"), format!("Any named node{places}"));
    assert_eq!(shown("not-eq"), format!("`#not-eq?`: {}{places}", shown("not-eq").split(": ").nth(1).unwrap().split("\n\n").next().unwrap()));
    assert_eq!(shown("target"), format!("`@target`: the place this location points at{places}"));
    assert!(shown("\"x\"").ends_with(&places.replace("Places of this location", "Places")), "{}", shown("\"x\""));
}

#[test]
fn broken_queries_show_no_places() {
    let kind = "`function_definition`: cpp node kind\n\nMatches 2 place(s) in 2 of 2 target file(s)";

    for query in ["r#\"(function_definition nope: (_) @target)\"#", "r#\"(function_definition body: (_\"#", "r#\"(function_definition) @other\"#"] {
        assert_eq!(placed(PLACED, query, "function_def", &[], Limits::default()), kind, "{query}");
    }
    assert_eq!(placed(PLACED, "r#\"(nope) @target\"#", "target", &[], Limits::default()), "`@target`: the place this location points at");
    assert_eq!(placed(PLACED, "r#\"(nope) @target\"#", "nope", &[], Limits::default()), "`nope` is not a cpp node kind");
}

#[test]
fn places_belong_to_their_block() {
    let fields = PLACED.replace("LOCATION", "\"function_definition\"");
    let text = format!("patch \"one\" {{\n{fields}}}\npatch \"two\" {{\n{}}}\n", fields.replace("*.cpp", "b.cpp"));
    let (_directory, project) = crate::preview::operation();
    let at = |offset: usize| hover(&codeshaper::registry(), &project, &mut Traces::default(), &text, offset + 2).unwrap().0;

    assert!(at(text.find("function_definition").unwrap()).contains("In 2 of 2 target file(s), 2 in all"));
    assert!(at(text.rfind("function_definition").unwrap()).contains("In 1 of 1 target file(s), 1 in all"));
}

#[test]
fn places_are_remembered_until_files_change() {
    let (directory, project) = crate::preview::operation();
    let text = PLACED.replace("LOCATION", "\"function_definition\"");
    let registry = codeshaper::registry();
    let mut traces = Traces::default();
    let mut shown = |keep_going: bool| {
        traces.guard(move || keep_going);
        hover(&registry, &project, &mut traces, &text, text.find("function_definition").unwrap()).unwrap().0
    };

    assert_eq!(shown(false), "cpp: node kind `function_definition`");
    assert!(shown(true).contains("2 in all"));
    assert!(shown(false).contains("2 in all"));

    std::fs::write(directory.path().join("target/c.cpp"), "void c() {}\n").unwrap();
    assert_eq!(shown(false), "cpp: node kind `function_definition`");
    assert!(shown(true).contains("In 3 of 3 target file(s), 3 in all"));
}
