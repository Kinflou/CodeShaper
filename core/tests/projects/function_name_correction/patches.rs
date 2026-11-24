// Standard Uses
use std::path::Path;

// Crate Uses
use crate::projects::function_name_correction::PROJECT_SETTINGS_PATH;

// External Uses
use codeshaper_core::shaping::project::Project;


#[test]
fn load_patches() {
    let project = Project::from_directory(Path::new(PROJECT_SETTINGS_PATH)).unwrap();
    let names: Vec<&str> = project.patches.iter().map(|patch| patch.name.as_str()).collect();

    assert_eq!(names, vec![
        "function_creation.kdl",
        "function_name_correction.kdl",
        "function_name_correction_cpp.kdl",
        "function_return_creation.kdl"
    ]);
    assert!(project.patch("function_creation.kdl").unwrap().patch.enabled);
    assert!(!project.patch("function_name_correction.kdl").unwrap().patch.enabled);
}
