// Relative Modules
mod completion;
mod diagnostics;
mod groups;
mod hover;
mod navigation;
mod position;
mod preview;
mod protocol;
mod query;
mod symbols;
mod syntax;
mod tokens;
mod trace;

// Standard Uses
use std::fs;
use std::path::PathBuf;

// Crate Uses

// External Uses
use tempfile::TempDir;


pub const SETTINGS: &str = "name \"Test\"\ntarget \"text\"\nast-set \"cpp\"\ndescription \"Test\"\n";

pub const BROKEN: &str = r##"enabled true
alias "cpp"
file "*.cpp"

actions {
    replacers {
        replacer {
            location "FunctionBody"
            from r"old ("
            to "#[missing]"
        }
    }
}
"##;


pub fn project(patches: &[(&str, &str)]) -> (TempDir, PathBuf) {
    let directory = tempfile::tempdir().unwrap();
    let project = directory.path().join("shaping_project");

    fs::create_dir_all(project.join("patches")).unwrap();
    fs::write(project.join("settings.kdl"), SETTINGS).unwrap();

    for (name, content) in patches {
        fs::write(project.join("patches").join(name), content).unwrap();
    }

    (directory, project)
}
