// Standard Uses
use std::{env, fs, io};
use std::io::Write;
use std::process::Command;

// Crate Uses

// External Uses
use anyhow::Result;


pub const ANTLR4_PATH_LOCAL: &str = "antlr4-4.8-2-SNAPSHOT-complete.jar";


#[allow(unused)]
fn main() {
    println!("ANTLR4 Builder Script");
    println!("Current Directory: {:?}", std::env::current_dir().unwrap());

    let grammars = vec![
        (
            "CPP14",
            "src/contents/ast/cpp14/grammars/",
            vec!["CPP14Lexer.g4", "CPP14Parser.g4"],
            "src/contents/ast/cpp14/generated/"
        ),
    ];


    println!("Generating ANTLR4 grammars code");

    for grammar in grammars.iter() {
        println!("Generating {:#?} to {}", grammar.0, grammar.3);
        generate_grammar_move(
            grammar.0, &grammar.1, &grammar.2, grammar.3
        ).unwrap();
    }

    println!("cargo:rerun-if-changed=build.rs");
}


#[allow(unused)]
fn generate_grammar_move(name: &str, path: &str, grammars: &Vec<&str>, target: &str) -> Result<()> {

    let generated_path = format!("{}/", name);
    let generated_path_all = format!("build/{}", generated_path);

    for grammar in grammars {
        let grammar_path = format!("{}/{}", path, grammar);
        let grammar_local_path = format!("{}/build/{}", env::current_dir()?.to_str().unwrap(), grammar);

        fs::copy(&grammar_path, &grammar_local_path);
        println!("Copying grammar {}", &grammar_path);
    }

    let mut command = Command::new("java");
    command
        .current_dir("build")
        .arg("-jar")
        .arg(ANTLR4_PATH_LOCAL)
        .arg("-Dlanguage=Rust")
        .arg("-visitor").arg("-listener")
        .arg("-o").arg(&generated_path)
        .arg("-lib")
        .arg(".").args(grammars);

    let output = command.output().expect("Antlr4-rust tool failed to start");

    io::stdout().write_all(&output.stdout)?;
    io::stderr().write_all(&output.stderr)?;


    println!("Moving generated from {} to {}", &generated_path_all, &target);
    fs::create_dir(&target);
    fs::rename(&generated_path_all, &target);

    for grammar in grammars {
        let grammar_local_path = format!("{}/build/{}", env::current_dir()?.to_str().unwrap(), grammar);
        fs::remove_file(grammar_local_path);
    }

    // fs::remove_dir_all(format!("build/{}", generated_path));

    Ok(())
}

