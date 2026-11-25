// Standard Uses
use std::rc::Rc;

// Crate Uses
use crate::operation::LineBackend;

// External Uses
use codeshaper_core::shaping::patch::Patch;
use codeshaper_core::shaping::patch::controller::PatchController;
use codeshaper_core::shaping::patch::pattern::Regexes;
use codeshaper_core::shaping::patch::trace::Trace;


const ACTIONS: &str = r##"
builder "around" {
    location "Line"
    match r"(?P<first>\w)(\d)?"
    build "<$[first]$[1]>"
}
builder "all" {
    location "Line"
    match r"^(B)"
    flags "i"
    build "$[0]"
    collect true
    separator "+"
}
maker "pair" {
    prepare r"(\w)(\d)"
    make "$[1]$[0]@[0]"
}
maker "plain" {
    make "(@[0])"
}
replacer {
    location "Line"
    from r"(\w+)=(\w+)"
    to "$[0]:#[pair($[0])]#[around]"
}
replacer {
    location "Line"
    from r"^(b\w*)"
    flags "im"
    to "#[plain($[0])]#[all]"
}
"##;

const SOURCES: [&str; 4] = ["a1=b\nBee\n", "", "b=c\n\nd2=e f=g\n", "Bee\nb\n"];

fn patch(actions: &str) -> Rc<Patch> {
    Rc::new(Patch::from_kdl("", &format!("enabled true\nfile \"*\"\nactions {{\n{actions}\n}}\n")).unwrap())
}

fn applied(patch: &Rc<Patch>, regexes: &Rc<Regexes>, source: &str) -> (String, Trace) {
    let mut controller = PatchController::traced(patch.clone(), 9).sharing(regexes.clone());
    let result = controller.apply(&LineBackend, source).unwrap();

    (result, controller.trace.take().unwrap().into_inner())
}


#[test]
fn regexes_are_compiled_once() {
    let regexes = Regexes::default();
    let first = regexes.get(r"(\w+)", "").unwrap();

    assert!(Rc::ptr_eq(&first, &regexes.get(r"(\w+)", "").unwrap()));
    assert!(!Rc::ptr_eq(&first, &regexes.get(r"(\w+)", "i").unwrap()));
    assert!(!Rc::ptr_eq(&first, &regexes.get(r"(\w)", "").unwrap()));
    assert!(regexes.get("(", "").is_none());
    assert!(regexes.get("(", "").is_none());
}

#[test]
fn patterns_and_flags_are_kept_apart() {
    let regexes = Regexes::default();

    assert!(!regexes.get("a", "").unwrap().is_match("A"));
    assert!(regexes.get("a", "i").unwrap().is_match("A"));
    assert!(!regexes.get("ai", "").unwrap().is_match("A"));
    assert!(!regexes.get("a.b", "").unwrap().is_match("a\nb"));
    assert!(regexes.get("a.b", "s").unwrap().is_match("a\nb"));
    assert!(regexes.get("a b", "x").unwrap().is_match("ab"));
    assert!(!regexes.get("^b", "").unwrap().is_match("a\nb"));
    assert!(regexes.get("^b", "m").unwrap().is_match("a\nb"));
}

#[test]
fn controllers_use_the_shared_regexes() {
    let patch = patch(ACTIONS);
    let regexes = Rc::new(Regexes::default());
    let patterns = [(r"(?P<first>\w)(\d)?", ""), ("^(B)", "i"), (r"(\w)(\d)", ""), (r"(\w+)=(\w+)", ""), (r"^(b\w*)", "im")];
    let shared: Vec<_> = patterns.iter().map(|(pattern, flags)| regexes.get(pattern, flags).unwrap()).collect();
    assert!(shared.iter().all(|regex| Rc::strong_count(regex) == 2));

    let mut first = PatchController::with(patch.clone()).sharing(regexes.clone());
    first.apply(&LineBackend, SOURCES[0]).unwrap();
    assert_eq!(shared.iter().map(Rc::strong_count).collect::<Vec<_>>(), vec![3; 5]);

    let mut second = PatchController::with(patch).sharing(regexes);
    second.apply(&LineBackend, SOURCES[0]).unwrap();
    assert_eq!(shared.iter().map(Rc::strong_count).collect::<Vec<_>>(), vec![4; 5]);
}

#[test]
fn unused_regexes_are_not_compiled() {
    let patch = patch(ACTIONS);
    let regexes = Rc::new(Regexes::default());
    let kept = regexes.get(r"(\w+)=(\w+)", "").unwrap();

    let mut controller = PatchController::with(patch).sharing(regexes);
    controller.apply(&LineBackend, "").unwrap();

    assert_eq!(Rc::strong_count(&kept), 2);
}

#[test]
fn sharing_changes_no_result() {
    let patch = patch(ACTIONS);
    let regexes = Rc::new(Regexes::default());

    for source in SOURCES.iter().chain(SOURCES.iter().rev()) {
        assert_eq!(applied(&patch, &regexes, source), applied(&patch, &Rc::default(), source), "{source:?}");
    }

    assert_eq!(applied(&patch, &regexes, SOURCES[0]).0, "a1:1aa1<a1>\n(Bee)B\n");
}

#[test]
fn patterns_that_do_not_compile_do_nothing() {
    let broken = ACTIONS.replace(r"(\w+)=(\w+)", "(").replace(r"(?P<first>\w)(\d)?", "(");
    assert_eq!(applied(&patch(&broken), &Rc::default(), SOURCES[0]).0, "a1=b\n(Bee)B\n");

    let unknown = ACTIONS.replace("flags \"im\"", "flags \"imq\"").replace("flags \"i\"", "flags \"iq\"");
    assert_eq!(applied(&patch(&unknown), &Rc::default(), SOURCES[0]).0, "a1:1aa1<a1>\nBee\n");

    let shared = Rc::new(Regexes::default());
    let known = patch(ACTIONS);
    applied(&known, &shared, SOURCES[0]);
    assert_eq!(applied(&patch(&unknown), &shared, SOURCES[0]).0, "a1:1aa1<a1>\nBee\n");
}
