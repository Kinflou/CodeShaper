// Standard Uses
use std::path::Path;

// Crate Uses
use crate::projects::function_name_correction::OPERATION_CONFIGURATION_PATH;

// External Uses
use codeshaper_core::shaping::operation::configuration::OperationConfiguration;
use codeshaper_core::shaping::project::settings::ProjectSettings;
use codeshaper_core::target::TargetLoader;
use codeshaper_core::target::text::TextLoader;


#[allow(unused)]
#[test]
fn load_patches() {
    let path = Path::new(OPERATION_CONFIGURATION_PATH);
    let operation_configuration = OperationConfiguration::from_file(path)
        .unwrap();

    let project_settings = ProjectSettings::from_path(
        Path::new(&operation_configuration.project)
    ).unwrap();

    let target_path = Path::new(&operation_configuration.target);
    let target_solution = TextLoader.load(target_path).unwrap();

    // println!("{target_solution:#?");

    assert_eq!(project_settings.name, "Function Name Correction");
}

