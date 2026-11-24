// Standard Uses
use std::fs;
use std::ops::Range;
use std::path::Path;

// Crate Uses
use crate::operation::{registry, workspace};

// External Uses
use codeshaper_core::shaping::operation::Operation;
use codeshaper_core::shaping::patch::Patch;
use codeshaper_core::shaping::patch::origin::Origin;
use codeshaper_core::shaping::patch::trace::{Found, Trace};


const SOURCE: &str = "old one\nkeep é𝄞 two\nold three\n";

fn traced(source: &str, builders: &str, replacers: &[(&str, &str, &str)], limit: usize) -> Trace {
    let directory = workspace("replace", &[]);
    fs::write(directory.path().join("target/a.txt"), source).unwrap();
    fs::write(directory.path().join("target/nested/b.txt"), "").unwrap();

    let replacers: String = replacers.iter()
        .map(|(location, from, to)| format!("replacer {{\nlocation \"{location}\"\nfrom r#\"{from}\"#\nto \"{to}\"\n}}\n"))
        .collect();
    let content = format!(
        "enabled true\nfile \"*.txt\"\nactions {{\nbuilders {{\n{builders}\n}}\nreplacers {{\n{replacers}}}\n}}\n"
    );
    let patch = Patch::from_kdl("", &content).unwrap();
    let operation = Operation::from_file(&directory.path().join("configuration.kdl")).unwrap();
    let trace = operation.trace(&registry(), "", &patch, limit).unwrap();

    assert_eq!(trace.directory, directory.path().join("target"));
    trace
}

fn builder(location: &str, pattern: &str, more: &str) -> String {
    format!("builder \"b\" {{\nlocation \"{location}\"\nmatch r#\"{pattern}\"#\nbuild \"<$[0]>\"\n{more}\n}}\n")
}

fn places(source: &str, trace: &Trace, action: &str) -> Vec<(String, Option<String>, Vec<Option<String>>)> {
    let found = &trace.action(action).unwrap().found;

    found.iter()
        .map(|Found { file, matched, groups }| {
            assert!(file == Path::new("a.txt") || matched.text.is_empty(), "{file:?}");
            let text = |range: &Range<usize>| format!("{}@{}", &source[range.clone()], range.start);
            let groups = groups.iter()
                .map(|group| group.as_ref().and_then(|group| group.place.as_ref()).map(text))
                .collect();

            (matched.text.clone(), matched.place.as_ref().map(text), groups)
        })
        .collect()
}

fn some(text: &str) -> Option<String> {
    Some(text.to_string())
}


#[test]
fn replacers_find_their_matches() {
    let trace = traced(SOURCE, "", &[("Line", r"(?:old (?P<name>\w+))|(two)", "x")], 9);

    assert_eq!(places(SOURCE, &trace, "replacer #1"), vec![
        ("old one".to_string(), some("old one@0"), vec![some("one@4"), None]),
        ("two".to_string(), some("two@20"), vec![None, some("two@20")]),
        ("old three".to_string(), some("old three@24"), vec![some("three@28"), None])
    ]);
}

#[test]
fn builders_find_their_matches() {
    let collecting = builder("Line", r"o(\w+)$", "collect true");
    let trace = traced(SOURCE, &collecting, &[("File", "one", "#[b]")], 9);
    assert_eq!(places(SOURCE, &trace, "b"), vec![("one".to_string(), some("one@4"), vec![some("ne@5")])]);

    let around = builder("Line", r"old (\w+)", "");
    let trace = traced(SOURCE, &around, &[("Line", "three|two", "#[b]")], 9);
    assert_eq!(places(SOURCE, &trace, "b"), vec![("old three".to_string(), some("old three@24"), vec![some("three@28")])]);

    let referring = builder("Line", r"(𝄞)", "reference-location \"File\"");
    let trace = traced(SOURCE, &referring, &[("Line", "one|three", "#[b]")], 9);
    assert_eq!(places(SOURCE, &trace, "b"), vec![("𝄞".to_string(), some("𝄞@15"), vec![some("𝄞@15")])]);
}

#[test]
fn inputs_that_do_not_match_take_no_place() {
    let trace = traced(SOURCE, &builder("Line", r"(three)", "collect true"), &[("File", "one", "#[b]")], 1);
    let action = trace.action("b").unwrap();

    assert_eq!((action.samples.len(), action.samples[0].matched), (1, false));
    assert_eq!(places(SOURCE, &trace, "b"), vec![("three".to_string(), some("three@28"), vec![some("three@28")])]);
}

#[test]
fn places_are_listed_once() {
    let trace = traced(SOURCE, &builder("Line", r"(old)", "collect true"), &[("Line", "one|three", "#[b]"), ("Line", "keep", "#[b]")], 9);

    assert_eq!(trace.action("b").unwrap().matched, 6);
    assert_eq!(places(SOURCE, &trace, "b"), vec![
        ("old".to_string(), some("old@0"), vec![some("old@0")]),
        ("old".to_string(), some("old@24"), vec![some("old@24")])
    ]);
}

#[test]
fn later_replacers_find_places_in_the_file() {
    let replacers = [("Line", "keep", "kept for longer"), ("Line", r"(\S+) (t\w+)$", "x")];
    let trace = traced(SOURCE, "", &replacers, 9);

    assert_eq!(places(SOURCE, &trace, "replacer #2"), vec![
        ("é𝄞 two".to_string(), some("é𝄞 two@13"), vec![some("é𝄞@13"), some("two@20")]),
        ("old three".to_string(), some("old three@24"), vec![some("old@24"), some("three@28")])
    ]);

    let replacers = [("Line", "keep", "kept"), ("Line", r"(\S+) (\S+) (two)", "x")];
    let trace = traced(SOURCE, "", &replacers, 9);

    assert_eq!(places(SOURCE, &trace, "replacer #2"), vec![
        ("kept é𝄞 two".to_string(), None, vec![None, some("é𝄞@13"), some("two@20")])
    ]);
}

#[test]
fn windows_line_endings_are_counted() {
    for source in [SOURCE.replace('\n', "\r\n"), SOURCE.replacen('\n', "\r\n", 1), "a\r\n\r\nold x\nold y\r\n".to_string()] {
        let trace = traced(&source, "", &[("Line", "keep", "k"), ("File", r"(?s)old (\w+)\n", "x")], 9);
        let found = places(&source, &trace, "replacer #2");

        assert_eq!(found.len(), 2, "{source:?}");
        for (text, place, groups) in found {
            let (place, _) = place.as_ref().unwrap().rsplit_once('@').unwrap();
            let (group, _) = groups[0].as_ref().unwrap().rsplit_once('@').unwrap();

            assert_eq!(place.replace("\r\n", "\n"), text);
            assert_eq!(text, format!("old {group}\n"));
        }
    }
}

#[test]
fn empty_matches_have_a_place() {
    let trace = traced("a\n\nb", "", &[("Line", "x?", "")], 9);
    let found: Vec<_> = trace.action("replacer #1").unwrap().found.iter().map(|found| found.matched.place.clone()).collect();

    assert_eq!(found, [0..0, 1..1, 2..2, 3..3, 4..4].map(Some));
}

#[test]
fn origins_follow_random_edits() {
    let mut seed = 7u64;
    let mut random = |limit: usize| {
        seed ^= seed << 13;
        seed ^= seed >> 7;
        seed ^= seed << 17;
        (seed % limit.max(1) as u64) as usize
    };
    let pieces = ["a", "b", "é", "𝄞", "\n", "\r\n", "ab"];
    let mut mapped = 0;

    for _ in 0..400 {
        let source: String = (0..random(30)).map(|_| pieces[random(pieces.len())]).collect();
        let mut text = source.replace("\r\n", "\n");
        let mut origin = Origin::new(&source);
        let mut model: Vec<Option<usize>> = source.bytes().enumerate()
            .filter(|(index, _)| !source[..].as_bytes()[*index..].starts_with(b"\r\n"))
            .map(|(index, _)| Some(index))
            .collect();

        for _ in 0..1 + random(4) {
            let mut edits = vec![];
            let mut position = 0;
            for _ in 0..random(4) {
                let boundaries: Vec<usize> = (position..=text.len()).filter(|index| text.is_char_boundary(*index)).collect();
                let start = boundaries[random(boundaries.len())];
                let after: Vec<usize> = boundaries.into_iter().filter(|index| *index >= start).collect();
                let end = after[random(after.len().min(8))];
                if !edits.is_empty() && start < position { break }

                let new: String = (0..random(5)).map(|_| ["a", "b", "é", "\n"][random(4)]).collect();
                edits.push((start, end, new));
                position = end;
            }

            origin.edit(edits.iter().map(|(start, end, new)| (*start, &text[*start..*end], new.as_str())));
            for (start, end, new) in edits.into_iter().rev() {
                let old = &text.as_bytes()[start..end];
                let prefix = old.iter().zip(new.as_bytes()).take_while(|(old, new)| old == new).count();
                let most = old.len().min(new.len()) - prefix;
                let suffix = old.iter().rev().zip(new.as_bytes().iter().rev()).take(most).take_while(|(old, new)| old == new).count();

                model.splice(start + prefix..end - suffix, vec![None; new.len() - prefix - suffix]);
                text.replace_range(start..end, &new);
            }

            assert_eq!(model.len(), text.len());
            for start in 0..text.len() {
                for end in start + 1..=text.len() {
                    if !text.is_char_boundary(start) || !text.is_char_boundary(end) { continue }

                    let whole = model[start..end].iter().all(Option::is_some)
                        && model[start..end].windows(2).all(|pair| {
                            let between = pair[0].unwrap() + 1..pair[1].unwrap().max(pair[0].unwrap() + 1);
                            pair[0] < pair[1] && source[..].as_bytes()[between.clone()] == b"\r"[..between.len().min(1)]
                                && (between.is_empty() || source.as_bytes()[between.end] == b'\n')
                        });
                    let place = origin.place(start..end);

                    assert_eq!(place.is_some(), whole, "{source:?} {text:?} {start}..{end}");
                    if let Some(place) = place {
                        assert_eq!(source[place].replace("\r\n", "\n"), text[start..end]);
                        mapped += 1;
                    }
                }
            }
        }
    }

    assert!(mapped > 10000, "{mapped}");
}
