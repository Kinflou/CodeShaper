// Relative Modules
#[cfg(all(feature = "cpp", feature = "text"))]
mod cpp14;
#[cfg(all(feature = "gdscript", feature = "godot"))]
mod godot;

// Standard Uses
use std::fs;
use std::path::Path;

// Crate Uses

// External Uses


pub fn copy_directory(from: &Path, to: &Path) {
    fs::create_dir_all(to).unwrap();

    for entry in fs::read_dir(from).unwrap() {
        let path = entry.unwrap().path();
        let destination = to.join(path.file_name().unwrap());

        if path.is_dir() {
            copy_directory(&path, &destination);
        } else {
            fs::copy(&path, &destination).unwrap();
        }
    }
}

pub fn read(path: &Path) -> String {
    fs::read_to_string(path).unwrap().replace("\r\n", "\n")
}
