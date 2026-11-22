// Standard Uses
use std::path::Path;

// Crate Uses
use crate::projects::function_name_correction::OPERATION_CONFIGURATION_PATH;

// External Uses
use codeshaper_core::shaping::operation::configuration::{OperationConfiguration, ResultOptions};
use codeshaper_core::shaping::operation::Operation;


#[test]
fn load_configuration() {
    let path = Path::new(OPERATION_CONFIGURATION_PATH);
    let configuration = OperationConfiguration::from_file(path).unwrap();

    let expected_configuration = OperationConfiguration {
        name: "Function Name Correction".to_string(),
        project: "function_name_correction/shaping_project/".to_string(),
        target: "function_name_correction/target/origin/".to_string(),
        output: "function_name_correction/target/expected_result/".to_string(),
        backup: "function_name_correction/target/backup/".to_string(),
        result: ResultOptions::BackupAndReplace
    };

    assert_eq!(configuration, expected_configuration);
}


#[test]
fn load_operation() {
    let operation = Operation::from_file(Path::new(OPERATION_CONFIGURATION_PATH)).unwrap();

    assert_eq!(operation.directory, Path::new("tests/data"));
    assert_eq!(operation.project.settings.name, "Function Name Correction");
    assert_eq!(operation.project.patches.len(), 4);
}


