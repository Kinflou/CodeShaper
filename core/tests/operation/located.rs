// Standard Uses
use std::cell::RefCell;
use std::ops::Range;
use std::rc::Rc;

// Crate Uses
use crate::operation::LineBackend;

// External Uses
use codeshaper_core::ast::backend::LanguageBackend;
use codeshaper_core::shaping::patch::Patch;
use codeshaper_core::shaping::patch::controller::PatchController;
use miette::{miette, Result};


#[derive(Default)]
struct CountingBackend {
    together: RefCell<Vec<(String, Vec<String>)>>,
    alone: RefCell<Vec<String>>
}

impl LanguageBackend for CountingBackend {
    fn name(&self) -> &str { "lines" }

    fn locate(&self, source: &str, location: &str) -> Result<Vec<Range<usize>>> {
        self.alone.borrow_mut().push(location.to_string());

        LineBackend.locate(source, location).map_err(|_| miette!("Broken {location}"))
    }

    fn locate_all(&self, source: &str, locations: &[&str]) -> Vec<Result<Vec<Range<usize>>>> {
        let names = locations.iter().map(|location| location.to_string()).collect();
        self.together.borrow_mut().push((source.to_string(), names));

        locations.iter()
            .map(|location| LineBackend.locate(source, location).map_err(|_| miette!("Broken {location}")))
            .collect()
    }
}

fn replacer(location: &str, from: &str, to: &str) -> String {
    format!("replacer {{\nlocation \"{location}\"\nfrom r\"{from}\"\nto \"{to}\"\n}}\n")
}

fn builder(location: &str) -> String {
    format!("builder \"b\" {{\nlocation \"{location}\"\nmatch r\"(.*)\"\nbuild \"<$[0]>\"\ncollect true\nseparator \",\"\n}}\n")
}

fn applied(builders: &str, replacers: &[String], source: &str) -> (Result<String>, Vec<String>, Vec<String>) {
    let content = format!(
        "enabled true\nfile \"*\"\nactions {{\n{builders}{}}}\n", replacers.concat()
    );
    let backend = CountingBackend::default();
    let result = PatchController::with(Rc::new(Patch::from_kdl("", &content).unwrap())).apply(&backend, source);

    (result, backend.together.into_inner().into_iter().map(|(source, _)| source).collect(), backend.alone.into_inner())
}

fn failed(builders: &str, replacers: &[String]) -> String {
    applied(builders, replacers, "a\n").0.unwrap_err().to_string()
}


#[test]
fn unchanged_text_is_located_once() {
    let replacers = [replacer("Line", "z", "y"), replacer("File", "z", "y"), replacer("Line", "a", "a")];
    let (result, located, alone) = applied("", &replacers, "a\nb\n");

    assert_eq!(result.unwrap(), "a\nb\n");
    assert_eq!(located, vec!["a\nb\n"]);
    assert!(alone.is_empty());
}

#[test]
fn changed_text_is_located_again() {
    let replacers = [replacer("Line", "a", "b"), replacer("Line", "z", "y"), replacer("File", "b", "c"), replacer("Line", "c", "d")];
    let (result, located, _) = applied("", &replacers, "a\nb\n");

    assert_eq!(result.unwrap(), "d\nd\n");
    assert_eq!(located, vec!["a\nb\n", "b\nb\n", "c\nc\n"]);
}

#[test]
fn every_location_is_asked_for_together() {
    let content = format!("enabled true\nfile \"*\"\nactions {{\n{}{}}}\n", builder("File"), replacer("Line", "a", "b"));
    let backend = CountingBackend::default();
    PatchController::with(Rc::new(Patch::from_kdl("", &content).unwrap())).apply(&backend, "a\n").unwrap();

    assert_eq!(backend.together.into_inner(), vec![("a\n".to_string(), vec!["File".to_string(), "Line".to_string()])]);
}

#[test]
fn collected_contents_follow_the_text() {
    let replacers = [replacer("Line", "^a$", "x"), replacer("Line", "^b$", "#[b]"), replacer("Line", "^c$", "#[b]")];
    let (result, _, _) = applied(&builder("Line"), &replacers, "a\nb\nc\n");

    assert_eq!(result.unwrap(), "x\n<x>,<b>,<c>\n<x>,<<x>,<b>,<c>>,<c>\n");
}

#[test]
fn broken_locations_fail_in_the_same_order() {
    let fine = replacer("Line", "a", "b");

    assert_eq!(failed("", &[replacer("Zzz", "a", "b"), replacer("Aaa", "a", "b")]), "Broken Zzz");
    assert_eq!(failed("", &[fine.clone(), replacer("Zzz", "a", "b"), replacer("Aaa", "a", "b")]), "Broken Aaa");
    assert_eq!(failed(&builder("Aaa"), &[fine, replacer("Zzz", "a", "b")]), "Broken Aaa");
    assert_eq!(failed("", &[replacer("", "a", "b"), replacer("Aaa", "a", "b")]), "Broken ");
}

#[test]
fn patches_without_replacers_locate_nothing() {
    let (result, located, alone) = applied(&builder("Aaa"), &[], "a\n");

    assert_eq!(result.unwrap(), "a\n");
    assert!(located.is_empty() && alone.is_empty());
}
