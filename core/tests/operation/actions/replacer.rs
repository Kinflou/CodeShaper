// Standard Uses
use std::ops::Range;
use std::rc::Rc;

// Crate Uses

// External Uses
use codeshaper_core::ast::backend::LanguageBackend;
use codeshaper_core::shaping::patch::Patch;
use codeshaper_core::shaping::patch::controller::PatchController;
use miette::Result;


struct WholeBackend;

impl LanguageBackend for WholeBackend {
    fn name(&self) -> &str { "whole" }

    fn locate(&self, source: &str, _location: &str) -> Result<Vec<Range<usize>>> {
        let whole = 0..source.len();

        Ok(vec![whole])
    }
}

fn patch(flags: &str, from: &str) -> Patch {
    let content = format!(r#"enabled true
alias "whole"
file "*"

actions {{
    replacer {{
        location "All"
        from r"{from}"
        to "<$[0]>"
        flags "{flags}"
    }}
}}"#);

    Patch::from_kdl("flags.kdl", &content).unwrap()
}

fn apply(flags: &str, from: &str, source: &str) -> Result<String> {
    PatchController::with(Rc::new(patch(flags, from))).apply(&WholeBackend, source)
}


#[test]
fn flags_shape_the_from_regex() {
    assert_eq!(apply("", "(old)", "OLD old").unwrap(), "OLD <old>");
    assert_eq!(apply("i", "(old)", "OLD old").unwrap(), "<OLD> <old>");
    assert_eq!(apply("m", "^(\\w+)$", "one\ntwo").unwrap(), "<one>\n<two>");
    assert_eq!(apply("s", "(a.b)", "a\nb").unwrap(), "<a\nb>");
    assert_eq!(apply("x", "(o l d)", "old").unwrap(), "<old>");
}

#[test]
fn unknown_flags_are_errors() {
    let replacer = patch("iq", "(a)").actions.replacers.remove(0);

    let error = replacer.from_regex().unwrap_err();
    assert_eq!(error.to_string(), "Unknown regex flag 'q', expected any of 'i', 'm', 's' or 'x'");
}
