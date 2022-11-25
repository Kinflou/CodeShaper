// Standard Uses
use std::path::Path;

// Crate Uses
use crate::projects::function_name_correction::PROJECT_SETTINGS_PATH;

// External Uses
use codeshaper::shaping::project::settings::ProjectSettings;
use codeshaper::shaping::project::Project;


#[test]
fn load_project_settings() {
    let settings_path = Path::new(PROJECT_SETTINGS_PATH);

    let settings = ProjectSettings::from_path(
        settings_path
    ).unwrap();

    let expected_settings = ProjectSettings {
        name: "Function Name Correction".to_string(),
        target: "text".to_string(),
        ast_set: "cpp14".to_string(),
        description: "  \
    Function Name Correction  \
    ========================    \
        \
    This example shows how it can change any found void functions in a specific file called Module.cpp  \
    that starts with '__' (two underscores) that we want to change to _ only instead as an example  \
    ".to_string()
    };

    assert_eq!(settings, expected_settings);
}


#[test]
fn load_project() {
    let settings_path = Path::new(PROJECT_SETTINGS_PATH);

    let project = Project::from_path(settings_path).unwrap();

    assert_eq!(project.settings.name, "Function Name Correction")
}