// Standard Uses
use std::path::Path;

// Crate Uses
use crate::projects::function_name_correction::OPERATION_CONFIGURATION_PATH;

// External Uses
use codeshaper::shaping::operation::configuration::OperationConfiguration;
use codeshaper::shaping::project::Project;


#[test]
fn load_patches() {
    let path = Path::new(OPERATION_CONFIGURATION_PATH);
    let operation_configuration = OperationConfiguration::from_file(path)
        .unwrap();

    let project_path = Path::new(&operation_configuration.project);
    let project = Project::from_path(project_path).unwrap();

    assert_eq!(project.settings.name, "Function Name Correction");
}

