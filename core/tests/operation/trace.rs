// Standard Uses
use std::path::PathBuf;

// Crate Uses
use crate::operation::{patch, registry, rename, workspace};

// External Uses
use codeshaper_core::shaping::operation::Operation;
use codeshaper_core::shaping::patch::Patch;
use codeshaper_core::shaping::patch::trace::{Sample, Trace};


fn trace(content: &str, limit: usize) -> Trace {
    let directory = workspace("replace", &[]);
    let operation = Operation::from_file(&directory.path().join("configuration.kdl")).unwrap();
    let patch = Patch::from_kdl("traced.kdl", content).unwrap();

    operation.trace(&registry(), "traced.kdl", &patch, limit).unwrap()
}


#[test]
fn replacers_record_each_match() {
    let trace = trace(&rename("lines", "Line"), 3);
    let replacer = trace.action("replacer #1").unwrap();

    assert_eq!(trace.files, 2);
    assert_eq!((replacer.inputs, replacer.matched, replacer.matches, replacer.matched_files), (3, 1, 1, 1));
    assert_eq!(replacer.samples, vec![Sample {
        file: PathBuf::from("a.txt"),
        input: "old one".to_string(),
        matched: true,
        groups: vec![Some("one".to_string())],
        output: "new one".to_string(),
        place: Some(0..7)
    }]);
}

#[test]
fn builders_record_inputs_and_calls() {
    let content = patch("lines", "Line", r"old (\w+)", "new $[0] #[names]").replace("    replacers {", r#"    builders {
        builder "names" {
            location "Line"
            match r"(\w+)"
            build "<$[0]>"
            collect true
            separator ","
        }
    }
    replacers {"#);
    let trace = trace(&content, 1);
    let builder = trace.action("names").unwrap();

    assert_eq!((builder.inputs, builder.matched, builder.calls), (2, 2, 1));
    assert_eq!(builder.samples.iter().map(|sample| sample.output.as_str()).collect::<Vec<_>>(), vec!["<old>"]);
    assert_eq!(builder.results[0].output, "<old>,<keep>");
    assert_eq!(builder.results[0].file, PathBuf::from("a.txt"));
    assert_eq!(trace.action("replacer #1").unwrap().samples[0].output, "new one <old>,<keep>");
}

#[test]
fn nested_actions_are_keyed_by_path() {
    let content = patch("lines", "Line", r"old (\w+)", "new #[inner]").replace("            to \"new #[inner]\"", r#"            to "new #[inner]"
            actions {
                makers {
                    maker "inner" {
                        make "made"
                    }
                }
            }"#);
    let trace = trace(&content, 3);

    assert_eq!(trace.action("replacer #1/inner").unwrap().results[0].output, "made");
}

#[test]
fn prepared_makers_record_their_input() {
    let content = patch("lines", "Line", r"(old \w+)", "#[getter]($[0])").replace("    replacers {", r#"    makers {
        maker "getter" {
            prepare r"(\w+) (\w+)"
            make "get_$[1]"
        }
    }
    replacers {"#);
    let trace = trace(&content, 3);
    let maker = trace.action("getter").unwrap();

    assert_eq!((maker.inputs, maker.matched, maker.calls), (1, 1, 1));
    assert_eq!(maker.samples[0].groups, vec![Some("old".to_string()), Some("one".to_string())]);
    assert_eq!(maker.samples[0].output, "get_one");
}

fn built(builder: &str, from: &str, to: &str) -> (usize, usize, usize, usize) {
    let builder = format!("    builders {{\n        builder \"b\" {{\n{builder}\n        }}\n    }}\n    replacers {{");
    let trace = trace(&patch("lines", "Line", from, to).replace("    replacers {", &builder), 1);
    let found = trace.action("b").cloned().unwrap_or_default();

    (found.inputs, found.matched, found.built, found.calls)
}

#[test]
fn built_texts_are_counted() {
    let collected = |content: &str| format!("location \"Line\"\ncollect true\n{content}");

    assert_eq!(built(&collected("match r\"(\\w+)\"\nbuild \"<$[0]>\""), "old", "#[b]"), (2, 2, 2, 1));
    assert_eq!(built(&collected("match r\"(\\w+)\"\nbuild \"<$[0]>\""), "old|keep", "#[b]"), (5, 5, 5, 3));
    assert_eq!(built(&collected("match r\"old (\\w+)\"\nbuild \"<$[0]>\""), "old", "#[b]"), (2, 1, 1, 1));
    assert_eq!(built(&collected("match r\"old (\\w+)\"\nbuild \"fixed\""), "old", "#[b]"), (2, 1, 2, 1));
    assert_eq!(built(&collected("match r\"(\\w+)\"\nbuild \"$[0]\""), "nothing", "#[b]"), (0, 0, 0, 0));
    assert_eq!(built("location \"Line\"\nmatch r\"missing (\\w+)\"\nbuild \"$[0]\"", "old", "#[b]"), (1, 0, 0, 0));
    assert_eq!(built("location \"Line\"\nmatch r\"keep\"\nbuild \"kept\"", "keep", "#[b] #[b]"), (4, 4, 4, 4));
}

#[test]
fn locations_are_counted_together() {
    let directory = workspace("replace", &[]);
    let operation = Operation::from_file(&directory.path().join("configuration.kdl")).unwrap();
    let patch = Patch::from_kdl("traced.kdl", &rename("lines", "Line")).unwrap();
    let (folder, files) = operation.matching_files(&registry(), "", &patch).unwrap();
    let count = |locations: &[&str]| operation.locate_all(&registry(), &patch, (&folder, &files), locations, &|| true).unwrap();

    let found = count(&["Line", "File", "Missing"]);
    let places: Vec<(usize, usize, usize)> = found.iter().map(|found| (found.files, found.matched_files, found.places)).collect();
    assert_eq!(places, vec![(2, 2, 3), (2, 2, 2), (2, 0, 0)]);

    for (location, found) in ["Line", "File", "Missing"].iter().zip(found) {
        assert_eq!(operation.locate(&registry(), "", &patch, location).unwrap(), found);
    }
    assert_eq!(operation.locate_all(&registry(), &patch, (&folder, &files), &[], &|| panic!("nothing to locate")).unwrap(), vec![]);
    assert!(operation.locate_all(&registry(), &patch, (&folder, &files), &["Line"], &|| false).is_err());
}
