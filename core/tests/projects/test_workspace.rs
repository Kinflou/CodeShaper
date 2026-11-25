// Standard Uses
use std::path::Path;

// Crate Uses

// External Uses
use codeshaper_core::shaping::operation::configuration::{OperationConfiguration, ResultOptions};
use codeshaper_core::shaping::patch::Patch;
use codeshaper_core::shaping::project::Project;


const TEST_WORKSPACE: &str = "tests/data/test_workspace/";


#[test]
fn load_test_workspace_configuration() {
    let path = Path::new(TEST_WORKSPACE).join("config.kdl");
    let configuration = OperationConfiguration::from_file(&path).unwrap();

    let expected_configuration = OperationConfiguration {
        name: "Test Workspace".to_string(),
        project: "shaping_project/".to_string(),
        target: "target/origin/".into(),
        output: "target/expected_result/".to_string(),
        backup: "target/backup/".to_string(),
        result: ResultOptions::BackupAndReplace,
        variables: Default::default()
    };

    assert_eq!(configuration, expected_configuration);
}

#[test]
fn load_test_workspace_project() {
    let path = format!("{TEST_WORKSPACE}shaping_project/");
    let project = Project::from_directory(Path::new(&path)).unwrap();

    assert_eq!(project.settings.name, "Test Shaping Project");
    assert_eq!(project.settings.ast_set.fixed(), Some("cpp"));
    assert_eq!(project.settings.description, "A little test with some cases");
    assert_eq!(project.patches.iter().map(|patch| patch.name.as_str()).collect::<Vec<_>>(), vec!["simple.kdl"]);
}

#[test]
fn load_test_workspace_patch() {
    let path = Path::new(TEST_WORKSPACE).join("shaping_project/patches/simple.kdl");
    let patch = Patch::from_path(&path).unwrap();

    assert!(!patch.enabled);
    assert_eq!(patch.file, "CallerModule.cpp");
}

#[test]
fn missing_projects_are_named() {
    let directory = tempfile::tempdir().unwrap();
    let error = Project::from_directory(directory.path()).err().unwrap().to_string();

    assert!(error.starts_with("Could not find project.kdl at"), "{error}");
}
