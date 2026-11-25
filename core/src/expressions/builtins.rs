// Standard Uses
use std::path::{Component, Path};

// Crate Uses

// External Uses


pub const COLLECT: &str = "collect.";
pub const FILE: &str = "file.";


#[derive(Debug, PartialEq, Eq)]
pub struct Builtin {
    pub name: &'static str,
    pub arguments: &'static [&'static str],
    pub text: &'static str,
    pub example: &'static str
}

#[derive(Debug, Default, Clone, Copy)]
pub struct Known<'a> {
    pub file: Option<&'a Path>,
    pub collected: Option<(usize, usize)>
}


const fn value(name: &'static str, text: &'static str, example: &'static str) -> Builtin {
    Builtin { name, arguments: &[], text, example }
}

const fn case(name: &'static str, text: &'static str, example: &'static str) -> Builtin {
    Builtin { name, arguments: &["text"], text, example }
}

pub const BUILTINS: &[Builtin] = &[
    value("file.path", "The path of the file being patched, relative to the target, always with '/'.", "to \"// #[file.path]\""),
    value(
        "file.folder",
        "The folder of the file being patched, relative to the target. It's empty for files at the top, so `#[file.folder]/x` is `/x` there.",
        "to \"#include \\\"#[file.folder]/all.h\\\"\""
    ),
    value("file.name", "The name of the file being patched, with its extension.", "to \"// #[file.name]\""),
    value("file.stem", "The name of the file being patched, without its extension.", "to \"#ifndef #[case.constant(#[file.stem])]_H\""),
    value("file.extension", "The extension of the file being patched, without the dot. It's empty if there's none.", "to \"// #[file.extension]\""),

    value(
        "collect.index",
        "The position of this text among the ones `collect` joins, from 0. Only in the `build` of a builder with `collect true`.",
        "collect true\nbuild \"#[collect.index]: $[0]\""
    ),
    value(
        "collect.count",
        "How many texts `collect` joins, without the places `match` finds nothing in when `build` uses its groups. Only in the `build` of a builder with `collect true`.",
        "collect true\nbuild \"$[0] of #[collect.count]\""
    ),

    case("case.upper", "The text in capitals.", "to \"#[case.upper($[0])]\""),
    case("case.lower", "The text in small letters.", "to \"#[case.lower($[0])]\""),
    case("case.snake", "The text as `get_player`. '_' at the ends is kept.", "to \"#[case.snake($[0])]\""),
    case("case.kebab", "The text as `get-player`. '_' at the ends is kept.", "to \"#[case.kebab($[0])]\""),
    case("case.camel", "The text as `getPlayer`. '_' at the ends is kept.", "to \"#[case.camel($[0])]\""),
    case("case.pascal", "The text as `GetPlayer`. '_' at the ends is kept.", "to \"#[case.pascal($[0])]\""),
    case("case.constant", "The text as `GET_PLAYER`. '_' at the ends is kept.", "to \"#[case.constant($[0])]\""),

    Builtin {
        name: "text.indent", arguments: &["text", "by"],
        text: "The text with every line that isn't empty indented, also the first one.",
        example: "to \"{\\n#[text.indent(#[members], '    ')]\\n}\""
    },
    Builtin {
        name: "text.or", arguments: &["text", "other"],
        text: "The text, or the other one if it's empty, as a group that wasn't found is.",
        example: "to \"#[text.or($[0], 'void')] $[1]()\""
    },
    Builtin {
        name: "text.replace", arguments: &["text", "old", "new"],
        text: "The text with every occurrence of the old part replaced.",
        example: "to \"#[text.replace($[0], '.', '::')]\""
    }
];


pub fn is_builtin(name: &str) -> bool {
    name.contains('.')
}

pub fn builtin(name: &str) -> Option<&'static Builtin> {
    BUILTINS.iter().find(|builtin| builtin.name == name)
}

pub fn closest(name: &str) -> Option<&'static str> {
    BUILTINS.iter()
        .map(|builtin| (distance(name, builtin.name), builtin.name))
        .min()
        .filter(|(distance, _)| *distance <= 2)
        .map(|(_, name)| name)
}

pub fn call(name: &str, arguments: &[String], known: Known) -> Option<String> {
    if builtin(name)?.arguments.len() != arguments.len() { return None }
    let text = arguments.first().map(String::as_str).unwrap_or_default();
    let part = |part: Option<&std::ffi::OsStr>| part.map(|part| part.to_string_lossy().to_string()).unwrap_or_default();

    Some(match name {
        "file.path" => slashed(known.file?),
        "file.folder" => slashed(known.file?.parent().unwrap_or(Path::new(""))),
        "file.name" => part(known.file?.file_name()),
        "file.stem" => part(known.file?.file_stem()),
        "file.extension" => part(known.file?.extension()),

        "collect.index" => known.collected?.0.to_string(),
        "collect.count" => known.collected?.1.to_string(),

        "case.upper" => text.to_uppercase(),
        "case.lower" => text.to_lowercase(),
        "case.snake" => joined(text, "_", |_, word| word.to_lowercase()),
        "case.kebab" => joined(text, "-", |_, word| word.to_lowercase()),
        "case.camel" => joined(text, "", |index, word| if index == 0 { word.to_lowercase() } else { capitalised(word) }),
        "case.pascal" => joined(text, "", |_, word| capitalised(word)),
        "case.constant" => joined(text, "_", |_, word| word.to_uppercase()),

        "text.indent" => indented(text, &arguments[1]),
        "text.or" => if text.is_empty() { arguments[1].clone() } else { text.to_string() },
        "text.replace" => if arguments[1].is_empty() { text.to_string() } else { text.replace(&arguments[1], &arguments[2]) },
        _ => return None
    })
}


fn slashed(path: &Path) -> String {
    let parts: Vec<String> = path.components()
        .filter(|part| *part != Component::CurDir)
        .map(|part| match part {
            Component::RootDir => String::new(),
            part => part.as_os_str().to_string_lossy().to_string()
        })
        .collect();

    parts.join("/")
}

fn indented(text: &str, by: &str) -> String {
    text.split_inclusive('\n')
        .map(|line| if line.trim_end_matches(['\r', '\n']).is_empty() { line.to_string() } else { format!("{by}{line}") })
        .collect()
}

fn joined(text: &str, separator: &str, shape: impl Fn(usize, &str) -> String) -> String {
    let inner = text.trim_matches('_');
    if inner.is_empty() { return text.to_string() }

    let start = text.len() - text.trim_start_matches('_').len();
    let words: Vec<String> = words(inner).into_iter().enumerate().map(|(index, word)| shape(index, word)).collect();

    format!("{}{}{}", &text[..start], words.join(separator), &text[start + inner.len()..])
}

fn words(text: &str) -> Vec<&str> {
    let mut words = vec![];

    for run in text.split(|character: char| !character.is_alphanumeric()).filter(|run| !run.is_empty()) {
        let characters: Vec<(usize, char)> = run.char_indices().collect();
        let mut start = 0;

        for (index, (at, character)) in characters.iter().enumerate().skip(1) {
            let after_small = characters[index - 1].1.is_lowercase();
            let before_small = characters.get(index + 1).is_some_and(|(_, next)| next.is_lowercase());

            if character.is_uppercase() && (after_small || before_small) {
                words.push(&run[start..*at]);
                start = *at;
            }
        }

        words.push(&run[start..]);
    }

    words
}

fn capitalised(word: &str) -> String {
    let mut characters = word.chars();
    let first = characters.next().map(|first| first.to_uppercase().to_string()).unwrap_or_default();

    first + &characters.as_str().to_lowercase()
}

fn distance(one: &str, other: &str) -> usize {
    let other: Vec<char> = other.chars().collect();
    let mut row: Vec<usize> = (0..=other.len()).collect();

    for (index, character) in one.chars().enumerate() {
        let mut above = std::mem::replace(&mut row[0], index + 1);

        for (at, known) in other.iter().enumerate() {
            let replaced = above + usize::from(character != *known);
            above = row[at + 1];
            row[at + 1] = replaced.min(above + 1).min(row[at] + 1);
        }
    }

    row[other.len()]
}
