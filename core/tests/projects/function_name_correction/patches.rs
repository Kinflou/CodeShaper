// Standard Uses
use std::path::Path;

// Crate Uses
use crate::projects::function_name_correction::PROJECT_SETTINGS_PATH;

// External Uses
use codeshaper_core::shaping::project::Project;


#[test]
fn load_patches() {
    let project = Project::from_directory(Path::new(PROJECT_SETTINGS_PATH)).unwrap();
    let names: Vec<&str> = project.patches.keys().map(String::as_str).collect();

    assert_eq!(names, vec![
        "function_creation.kdl",
        "function_name_correction.kdl",
        "function_name_correction_cpp.kdl",
        "function_return_creation.kdl"
    ]);
    assert!(project.patches["function_creation.kdl"].enabled);
    assert!(!project.patches["function_name_correction.kdl"].enabled);
}
