// Standard Uses
use std::path::Path;

// Crate Uses

// External Uses
use codeshaper_core::shaping::operation::configuration::{OperationConfiguration, ResultOptions};
use codeshaper_core::shaping::project::Project;
use codeshaper_core::target::TargetLoader;
use codeshaper_core::target::text::TextLoader;


const EXAMPLE: &str = "../codeshaper-rs/examples/CPP14/Function Name Correction/";


#[test]
fn load_example_configuration() {
    let path = Path::new(EXAMPLE).join("configuration.kdl");
    let configuration = OperationConfiguration::from_file(&path).unwrap();

    let expected_configuration = OperationConfiguration {
        name: "Function Name Correction".to_string(),
        project: "shaping_project/".to_string(),
        target: "target/".to_string(),
        output: "target_result/".to_string(),
        backup: "target_backup/".to_string(),
        result: ResultOptions::BackupAndReplace
    };

    assert_eq!(configuration, expected_configuration);
}

#[test]
fn load_example_project() {
    let path = format!("{EXAMPLE}shaping_project/");
    let project = Project::from_directory(Path::new(&path)).unwrap();

    assert_eq!(project.settings.name, "Function Name Correction");
    assert_eq!(project.settings.target, "text");
    assert_eq!(project.settings.ast_set, "cpp");
}

#[test]
fn load_example_target() {
    let path = format!("{EXAMPLE}target/");
    let target = TextLoader.load(Path::new(&path)).unwrap();

    assert_eq!(target.name, "Function Name Correction");
}
