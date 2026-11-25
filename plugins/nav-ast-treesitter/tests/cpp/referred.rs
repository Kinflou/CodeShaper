// Standard Uses
use std::rc::Rc;

// Crate Uses

// External Uses
use codeshaper_core::shaping::patch::Patch;
use codeshaper_core::shaping::patch::controller::PatchController;
use codeshaper_nav_ast_treesitter::TreeSitterBackend;


const COLLECT: &str = "collect true\nseparator \", \"";

const CLASSES: &str = "class A\n{\n\tvoid f()\n\t{\n\t\t// x\n\t}\n};\n\nclass B\n{\n\tvoid g()\n\t{\n\t\t// x\n\t}\n};\n";

const ENUMS: &str = "enum Color\n{\n\tRed = 1, // the first\n\tGreen,\n\tBlue = Red + 4\n}; // x\n\nenum Size { Small , Large };\n\nvoid f()\n{\n\t// x\n}\n";

fn builder(location: &str, reference: &str, more: &str) -> String {
    format!(
        "builder \"b\" {{\nlocation r#\"{location}\"#\nreference-location r#\"{reference}\"#\nmatch r\"(\\w+)\"\nbuild \"$[0]\"\n{more}\n}}\n"
    )
}

fn replacer(location: &str, from: &str, to: &str) -> String {
    format!("replacer {{\nlocation \"{location}\"\nfrom r\"{from}\"\nto \"{to}\"\n}}\n")
}

fn applied(actions: &str, source: &str) -> String {
    let patch = Patch::from_kdl("", &format!("enabled true\nfile \"*\"\nactions {{\n{actions}}}\n")).unwrap();
    let patch = Rc::new(patch);
    let backend = TreeSitterBackend::cpp();
    let result = PatchController::with(patch.clone()).apply(&backend, source).unwrap();

    assert_eq!(PatchController::traced(patch.clone(), 9).apply(&backend, source).unwrap(), result);
    assert_eq!(PatchController::with(patch).apply(&backend, &source.replace('\n', "\r\n")).unwrap(), result.replace('\n', "\r\n"));

    result
}


#[test]
fn builders_go_out_then_in() {
    let names = |location: &str, reference: &str| applied(&(builder(location, reference, "") + &replacer("function_definition.body", "x", "#[b]")), CLASSES);

    assert_eq!(names("class_specifier", "class_specifier.name"), CLASSES.replacen('x', "A", 1).replacen('x', "B", 1));
    assert_eq!(names("class_specifier", "(class_specifier name: (_) @target)"), CLASSES.replacen('x', "A", 1).replacen('x', "B", 1));
    assert_eq!(names("function_definition", "function_definition.declarator.declarator"), CLASSES.replacen('x', "f", 1).replacen('x', "g", 1));
}

#[test]
fn builders_go_in_then_out() {
    let names = |location: &str| applied(&(builder(location, "class_specifier", "") + &replacer("function_definition.body", "x", "#[b]")), CLASSES);

    assert_eq!(names("comment"), CLASSES.replace('x', "class"));
    assert_eq!(names("function_definition"), CLASSES.replace('x', "class"));
}

#[test]
fn locations_found_nowhere_near_give_nothing() {
    for more in ["", COLLECT] {
        let called = |location: &str| applied(&(builder(location, "identifier", more) + &replacer("function_definition.body", "x", "#[b]")), ENUMS);

        assert_eq!(called("class_specifier"), ENUMS);
        assert_eq!(called("enum_specifier"), ENUMS);
        assert_eq!(called(r#"(enum_specifier name: (type_identifier) @n (#eq? @n "Color")) @target"#), ENUMS);
    }
}

#[test]
fn collecting_stays_inside_the_location() {
    let names = builder("enum_specifier", "enum_specifier.enumerator.name", COLLECT) + &replacer("enum_specifier", r"\}$", "} /* #[b] */");

    assert_eq!(applied(&names, ENUMS), ENUMS.replace("};", "} /* Small, Large */;").replacen("Small, Large", "Red, Green, Blue", 1));
    assert_eq!(applied(&names, "enum E {};\n"), "enum E {} /*  */;\n");

    let whole = builder("enum_specifier", "enum_specifier", COLLECT) + &replacer("enum_specifier", r"\}$", "} /* #[b] */");
    assert_eq!(applied(&whole, ENUMS), ENUMS.replace("};", "} /* enum */;"));

    let around = builder("enum_specifier.body", "enum_specifier", COLLECT) + &replacer("enum_specifier", r"\}$", "} /* #[b] */");
    assert_eq!(applied(&around, ENUMS), ENUMS.replace("};", "} /*  */;"));
}

#[test]
fn collecting_without_a_reference_takes_the_file() {
    let names = builder("enum_specifier.enumerator.name", "", COLLECT).replace("reference-location r#\"\"#\n", "") + &replacer("enum_specifier", r"\}$", "} /* #[b] */");

    assert_eq!(applied(&names, ENUMS), ENUMS.replace("};", "} /* Red, Green, Blue, Small, Large */;"));
}

#[test]
fn nested_places_are_collected_with_their_outer_place() {
    let source = "struct A\n{\n\tint a;\n\tstruct B { int b; } c;\n};\n";
    let names = |location: &str| builder(location, "field_identifier", COLLECT) + &replacer("struct_specifier", r"\}$", "/* #[b] */ }");

    assert_eq!(applied(&names("struct_specifier"), source), source.replace("};\n", "/* a, b, c */ };\n"));
}

#[test]
fn the_place_around_comes_before_the_one_inside() {
    let source = "void f()\n{\n\tstruct A { void g() { x; } };\n}\n";
    let called = |location: &str, reference: &str| applied(&(builder(location, reference, "") + &replacer("expression_statement", "x", "#[b]")), source);

    assert_eq!(called("function_definition", "field_identifier"), source.replace('x', "g"));
    assert_eq!(called("struct_specifier", "function_definition.declarator"), source.replace('x', "g"));
    assert_eq!(called("struct_specifier", "function_definition.body"), source.replace('x', "struct"));
}

#[test]
fn later_replacers_read_the_changed_text() {
    let names = replacer("enum_specifier.enumerator.name", "^", "e") + &builder("enum_specifier", "enum_specifier.enumerator.name", COLLECT)
        + &replacer("enum_specifier", r"\}$", "} /* #[b] */");
    let renamed = "enum A { ea, eb } /* ea, eb */;\nenum B { ec } /* ec */;\n";

    assert_eq!(applied(&names, "enum A { a, b };\nenum B { c };\n"), renamed);
}

#[test]
fn builders_call_referring_builders() {
    let outer = "builder \"outer\" {\nlocation \"class_specifier\"\nmatch \"x\"\nbuild \"<#[b]>\"\n}\n";
    let names = builder("class_specifier", "class_specifier.name", "") + outer + &replacer("function_definition.body", "x", "#[outer]");

    assert_eq!(applied(&names, CLASSES), CLASSES.replacen('x', "<A>", 1).replacen('x', "<B>", 1));
}
