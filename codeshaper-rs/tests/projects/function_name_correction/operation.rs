// Standard Uses
use std::path::Path;

// Crate Uses
use crate::projects::function_name_correction::OPERATION_CONFIGURATION_PATH;

// External Uses
use codeshaper::shaping::operation::configuration::{OperationConfiguration, ResultOptions};
use codeshaper::shaping::operation::Operation;


#[test]
fn load_configuration() {
    let path = Path::new(OPERATION_CONFIGURATION_PATH);
    let configuration = OperationConfiguration::from_file(path).unwrap();

    let expected_configuration = OperationConfiguration {
        name: "Function Name Correction".to_string(),
        project: "tests/data/function_name_correction/shaping_project/".to_string(),
        target: "tests/data/function_name_correction/target/origin/".to_string(),
        output: "tests/data/function_name_correction/target/expected_result/".to_string(),
        backup: "tests/data/function_name_correction/target/backup/".to_string(),
        result: ResultOptions::BackupAndReplace
    };

    assert_eq!(configuration, expected_configuration);
}


#[test]
fn load_operation() {
    let path = Path::new(OPERATION_CONFIGURATION_PATH);
    let configuration = OperationConfiguration::from_file(path).unwrap();

    let mut operation = Operation::from_configuration(configuration)
        .unwrap();

    operation.start()
}


