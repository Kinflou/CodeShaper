// Standard Uses
use std::ops::Range;
use std::path::{Path, PathBuf};

// Crate Uses

// External Uses
use codeshaper_core::ast::backend::LanguageBackend;
use codeshaper_core::registry::Registry;
use codeshaper_core::target::{Solution, TargetLoader};
use miette::Result;


struct StubBackend;

impl LanguageBackend for StubBackend {
    fn name(&self) -> &str { "stub" }

    fn locate(&self, _source: &str, _location: &str) -> Result<Vec<Range<usize>>> {
        Ok(vec![])
    }
}

struct StubLoader;

impl TargetLoader for StubLoader {
    fn name(&self) -> &str { "stub" }

    fn load(&self, path: &Path) -> Result<Solution> {
        Ok(Solution { name: "Stub".to_string(), directory: path.to_path_buf(), groups: vec![] })
    }
}

fn registry() -> Registry {
    let mut registry = Registry::default();
    registry.add_backend(Box::new(StubBackend));
    registry.add_loader(Box::new(StubLoader));

    registry
}


#[test]
fn backends_are_named() {
    let registry = registry();

    assert_eq!(registry.backend("stub").unwrap().name(), "stub");
    assert!(registry.backend("other").is_none());
    assert_eq!(registry.languages(), vec!["stub"]);
}

#[test]
fn load_target_by_name() {
    let solution = registry().load_target("stub", Path::new("project/")).unwrap();

    assert_eq!(solution.name, "Stub");
    assert_eq!(solution.directory, PathBuf::from("project/"));
}

#[test]
fn unknown_target_lists_known() {
    let error = registry().load_target("vcx", Path::new("project/")).unwrap_err();

    assert!(error.to_string().contains("'vcx'"));
    assert!(error.to_string().contains("stub"));
}
