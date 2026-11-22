// Standard Uses

// Crate Uses
use crate::project;

// External Uses
use codeshaper_lsp::hover::hover;


const PATCH: &str = r##"alias "cpp"
actions {
    makers {
        maker "pair" {
            make "@[0]=@[1]"
        }
    }
    replacers {
        replacer {
            location "FunctionDefinition.body"
            reference-location "Nope"
            to "#[pair](a, b)"
        }
    }
}
"##;

fn at(needle: &str, shift: usize) -> Option<(String, String)> {
    let (_directory, project) = project(&[]);
    let offset = PATCH.find(needle).unwrap() + shift;

    hover(&codeshaper::registry(), &project, PATCH, offset).map(|(text, range)| (text, PATCH[range].to_string()))
}


#[test]
fn actions_show_their_definition() {
    let (text, range) = at("#[pair]", 3).unwrap();

    assert_eq!(text, "```kdl\nmaker \"pair\" {\n            make \"@[0]=@[1]\"\n        }\n```");
    assert_eq!(range, "\"pair\"");
}

#[test]
fn locations_are_described_by_the_backend() {
    let (text, range) = at("FunctionDefinition", 2).unwrap();
    assert_eq!(text, "cpp: node kind `function_definition` › field `body`");
    assert_eq!(range, "\"FunctionDefinition.body\"");

    let (text, _) = at("Nope", 1).unwrap();
    assert!(text.starts_with("Not a cpp location: "), "{text}");
}

#[test]
fn other_values_have_no_hover() {
    assert_eq!(at("@[0]=", 1), None);
}
