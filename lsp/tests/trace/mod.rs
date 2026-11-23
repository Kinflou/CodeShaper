// Standard Uses
use std::sync::Arc;

// Crate Uses
use crate::preview::{operation, RENAME};

// External Uses
use codeshaper_lsp::trace::{diff, inline, truncate, Limits, Traces};


const SMALL: Limits = Limits { samples: 1, lines: 2, characters: 10 };


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
