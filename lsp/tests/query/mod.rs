// Standard Uses

// Crate Uses
use crate::preview::operation;
use crate::project;

// External Uses
use codeshaper_lsp::completion::completion;
use codeshaper_lsp::hover::hover;
use codeshaper_lsp::query::{captures, part_at, Part};
use codeshaper_lsp::trace::Traces;
use lsp_types::CompletionTextEdit;


const QUERY: &str = r##"(function_definition declarator: (_) @fn (#match? @fn "(a") ; (b
    !body) @target"##;

fn part(needle: &str, shift: usize) -> Option<(Part, &'static str)> {
    part_at(QUERY, QUERY.find(needle).unwrap() + shift).map(|(part, word)| (part, &QUERY[word]))
}

fn patch(query: &str) -> String {
    format!("alias \"cpp\"\nfile \"*.cpp\"\nactions {{\n    replacers {{\n        replacer {{\n            location r#\"{query}\"#\n        }}\n    }}\n}}\n")
}

fn completed(query: &str) -> Vec<(String, String)> {
    let (_directory, project) = project(&[]);
    let text = patch(&query.replace('|', ""));
    let offset = text.find("r#\"").unwrap() + 3 + query.find('|').unwrap();

    completion(&codeshaper::registry(), &project, &text, offset).into_iter()
        .map(|item| {
            let Some(CompletionTextEdit::Edit(edit)) = item.text_edit else { panic!("expected an edit") };
            (item.label, edit.new_text)
        })
        .collect()
}

fn hovered(query: &str, needle: &str) -> (String, String) {
    let (_directory, project) = operation();
    let text = patch(query);
    let offset = text.find(needle).unwrap() + 1;
    let (found, range) = hover(&codeshaper::registry(), &project, &mut Traces::default(), &text, offset).unwrap();

    (found, text[range].to_string())
}


#[test]
fn parts_follow_their_surroundings() {
    assert_eq!(part("function_definition", 3), Some((Part::Kind, "function_definition")));
    assert_eq!(part("declarator", 0), Some((Part::Field, "declarator")));
    assert_eq!(part("match?", 6), Some((Part::Predicate, "match?")));
    assert_eq!(part("@target", 1), Some((Part::Capture, "target")));
    assert_eq!(part("!body", 2), Some((Part::Field, "!body")));
    assert_eq!(part("\"(a", 2), None);
    assert_eq!(part("; (b", 4), None);
}

#[test]
fn captures_are_listed_once() {
    assert_eq!(captures(QUERY).into_iter().collect::<Vec<_>>(), vec!["fn", "target"]);
}

#[test]
fn each_part_has_its_own_completion() {
    assert!(completed("(func|").contains(&("function_definition".to_string(), "function_definition".to_string())));
    assert!(completed("(function_definition dec|").contains(&("declarator".to_string(), "declarator: ".to_string())));
    assert!(completed("(function_definition dec|: (_))").contains(&("declarator".to_string(), "declarator".to_string())));
    assert!(completed("((identifier) @a (#|").iter().any(|(label, _)| label == "any-of?"));
    assert_eq!(completed("((identifier) @a (#eq? @|").iter().map(|(label, _)| label.as_str()).collect::<Vec<_>>(), vec!["a", "target"]);
    assert_eq!(completed("((identifier) @a (#eq? @a \"(|"), vec![]);
}

#[test]
fn each_part_has_its_own_hover() {
    let query = "((function_definition declarator: (nope) @x) @target (#eq? @x \"a\"))";

    let (text, range) = hovered(query, "function_definition");
    assert_eq!(range, "function_definition");
    assert_eq!(text, "`function_definition`: cpp node kind\n\nMatches 2 place(s) in 2 of 2 target file(s)");

    assert_eq!(hovered(query, "declarator").0, "`declarator`: cpp field");
    assert_eq!(hovered(query, "nope").0, "`nope` is not a cpp node kind");
    assert_eq!(hovered(query, "eq?").0, "`#eq?`: the capture's text equals a string or another capture");
    assert!(hovered("(function_definition) @target", "target").0.ends_with("Matches 2 place(s) in 2 of 2 target file(s)"));
}
