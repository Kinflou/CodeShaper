// Standard Uses
use std::path::Path;

// Crate Uses
use crate::projects::function_name_correction::PROJECT_SETTINGS_PATH;

// External Uses
use codeshaper_core::shaping::project::ast_set::AstSet;
use codeshaper_core::shaping::project::settings::ProjectSettings;
use codeshaper_core::shaping::project::Project;


#[test]
fn load_project_settings() {
    let settings_path = Path::new(PROJECT_SETTINGS_PATH);

    let settings = ProjectSettings::from_path(
        settings_path
    ).unwrap();

    let expected_settings = ProjectSettings {
        name: "Function Name Correction".to_string(),
        target: "text".into(),
        ast_set: AstSet::Language("cpp".to_string()),
        description: "Function Name Correction\n\
            ========================\n\
            \n\
            This example shows how it can change any found void functions in a specific file called Module.cpp\n\
            that starts with '__' (two underscores) that we want to change to _ only instead as an example"
            .to_string(),
        constants: vec![],
        variables: vec![]
    };

    assert_eq!(settings, expected_settings);
}


#[allow(unused)]
#[test]
fn load_project() {
    let settings_path = Path::new(PROJECT_SETTINGS_PATH);
    let project_settings = ProjectSettings::from_path(settings_path).unwrap();

    let project = Project::from_directory(settings_path).unwrap();

    assert_eq!(project.settings.name, "Function Name Correction")
}