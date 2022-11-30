// Standard Uses
use std::{fs, io};
use std::io::Write;
use std::process::Command;

// Crate Uses

// External Uses
use anyhow::Result;


pub const ANTLR4_PATH: &str = "build/antlr4-4.8-2-SNAPSHOT-complete.jar";

#[allow(unused)]
fn main() {
    println!("ANTLR4 Builder Script");
    println!("Current Directory: {:?}", std::env::current_dir().unwrap());

    let grammars = vec![
        (
            "src/contents/ast/cpp14/grammars/",
            "CPP14Lexer.g4",
            "src/contents/ast/cpp14/generated/"
        ),
        (
            "src/contents/ast/cpp14/grammars/",
            "CPP14Parser.g4",
            "src/contents/ast/cpp14/generated/"
        ),
    ];

    println!("Generating ANTLR4 grammars code");

    for grammar in grammars.iter() {
        println!("Generating {:#?} to {}", grammar.1, grammar.2);
        generate_grammar(grammar.0, grammar.1, grammar.2).unwrap();
    }

    println!("cargo:rerun-if-changed=build.rs");
}

#[allow(unused)]
fn generate_grammar(path: &str, grammar: &str, output: &str) -> Result<()> {
    // let mut command = Command::new("java");
    // command.arg("-jar").arg(ANTLR4_PATH)

    let mut command = Command::new("antlr4");
    command
        .arg("-Dlanguage=Rust")
        .arg("-visitor").arg("-listener")
        .arg("-o").arg(&output)
        .arg("-lib").arg(path).arg(format!("{}{}", path, grammar));

    let output = command.output().expect("Antlr4 tool failed to start");

    io::stdout().write_all(&output.stdout)?;
    io::stderr().write_all(&output.stderr)?;

    Ok(())
}

