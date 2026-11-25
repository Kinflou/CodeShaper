// Standard Uses
use std::ops::Range;
use std::path::Path;

// Crate Uses
use crate::document::{kind, DocumentKind};
use crate::options::{target_kind, TARGET};
use crate::paths::TEXT;
use crate::syntax::Outline;

// External Uses
use lsp_types::{Documentation, MarkupContent, MarkupKind};


#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Explained {
    pub parent: Option<&'static str>,
    pub name: &'static str,
    pub need: Option<&'static str>,
    pub text: &'static str,
    pub example: &'static str
}

impl Explained {
    pub fn markdown(&self) -> String {
        let need = self.need.map(|need| format!(" · {need}")).unwrap_or_default();

        format!("**{}**{need}\n\n{}\n\n```kdl\n{}\n```", self.name, self.text, self.example)
    }
}


const fn top(name: &'static str, need: &'static str, text: &'static str, example: &'static str) -> Explained {
    Explained { parent: None, name, need: Some(need), text, example }
}

const fn within(parent: &'static str, name: &'static str, need: &'static str, text: &'static str, example: &'static str) -> Explained {
    Explained { parent: Some(parent), name, need: Some(need), text, example }
}

const fn action(name: &'static str, text: &'static str, example: &'static str) -> Explained {
    Explained { parent: Some("actions"), name, need: None, text, example }
}

macro_rules! flags {
    ($parent:literal, $field:literal) => {
        within(
            $parent, "flags", "optional, none if left out",
            concat!(
                "Regex flags for `", $field, "`: `i` (case insensitive), `m` ('^' and '$' match at line breaks), ",
                "`s` ('.' also matches '\\n') and `x` (whitespace and '#' comments are ignored)."
            ),
            "flags \"im\""
        )
    };
}


pub const REQUIRED: &str = "required";
const OPTIONAL: &str = "optional";

const INNER: &str = "Actions that belong to this one. Their names are unique in the whole patch, and any of its expressions can call them.";
const FILES: &str = "The target's files, as names or patterns relative to the target. `*` stays in one folder, `**` crosses folders.";
const EXCLUDE: &str = "Names or patterns, relative to the target, to leave out of its files.";

pub const PATCH: &[Explained] = &[
    top(
        "patch", OPTIONAL,
        "One of several patches in this file, with a name to tell them apart. Without these blocks, the file is one patch.",
        "patch \"rename\" {\n    file \"*.cpp\"\n    actions {\n    }\n}"
    ),
    top(
        "enabled", "optional, `true` if left out",
        "`false` leaves this patch out of runs and of the operation preview. It's still checked, and can still be previewed alone.",
        "enabled false"
    ),
    top(
        "alias", "optional, the project's `ast-set` if left out",
        "The language this patch reads its files in, instead of the one the project's `ast-set` gives them.",
        "alias \"cpp\""
    ),
    top(
        "file", REQUIRED,
        "Which target files this patch changes, as a pattern relative to the target. `*` also crosses folders.",
        "file \"src/*.cpp\""
    ),
    top(
        "actions", REQUIRED,
        "What this patch does. Replacers run in the order written, the order of the other actions means nothing.",
        "actions {\n    replacer {\n        location \"identifier\"\n        from \"old\"\n        to \"new\"\n    }\n}"
    ),

    action(
        "builder",
        "Makes text out of the target's code, for expressions to call as `#[name]`.",
        "builder \"name\" {\n    location \"function_definition.declarator\"\n    match r\"(\\w+)\\(\"\n    build \"$[0]\"\n}"
    ),
    action(
        "replacer",
        "Changes the target's code. Replacers run one after another in the order written, each on the text the earlier ones left. The name is optional.",
        "replacer \"rename\" {\n    location \"identifier\"\n    from r\"^old_(\\w+)\"\n    to \"new_$[0]\"\n}"
    ),
    action(
        "maker",
        "Makes text out of the arguments it's called with, as `#[name]($[0], $[1])`.",
        "maker \"pair\" {\n    make \"@[0] = @[1]\"\n}"
    ),
    action(
        "switch",
        "Picks a text by its key, as `#[name]($[0])`.",
        "switch \"type\" {\n    cases integer=\"int\" boolean=\"bool\"\n    index \"@[0]\"\n}"
    ),
    action(
        "list",
        "Picks a text by its position, as `#[name]($[0])`.",
        "list \"order\" {\n    values \"first\" \"second\"\n    index \"@[0]\"\n}"
    ),

    within(
        "builder", "location", REQUIRED,
        "The code this builder reads when a replacement calls it, as a path or a query: the smallest place around the replacer's, or else the first one inside it. With `collect`, every place in the file, unless there's a `reference-location`.",
        "location \"function_definition\""
    ),
    within(
        "builder", "reference-location", "optional, `location` if left out",
        "The code to read when it isn't the place of `location` itself: the smallest place around it, or else the first one inside it. With `collect`, every one inside it.",
        "location \"class_specifier\"\nreference-location \"class_specifier.name\""
    ),
    within(
        "builder", "match", REQUIRED,
        "A regex for the code this builder reads. Its first match gives `build` the capture groups: `$[0]` is the first one, `$[name]` a named one.",
        "match r\"void (\\w+)\\(\""
    ),
    flags!("builder", "match"),
    within(
        "builder", "build", REQUIRED,
        "The text this builder gives. It can use the groups of `match` as `$[0]` and call other actions as `#[name]`. If it uses a group and `match` finds nothing, it gives nothing: the place is left out of `collect`, otherwise nothing is replaced at the place it's called for.",
        "build \"call_$[0]();\""
    ),
    within(
        "builder", "collect", "optional, `false` if left out",
        "`true` builds from every place in the file and joins the texts, instead of building from one place. With a `reference-location`, from every one inside the place of `location`.",
        "location \"enum_specifier\"\nreference-location \"enum_specifier.enumerator.name\"\ncollect true"
    ),
    within(
        "builder", "separator", "optional, nothing if left out",
        "What goes between the texts `collect` joins. Not used without `collect`.",
        "separator \", \""
    ),
    within("builder", "actions", OPTIONAL, INNER, "actions {\n    maker \"upper\" {\n        make \"@[0]\"\n    }\n}"),

    within(
        "replacer", "location", REQUIRED,
        "Where to replace, as a path or a query. The replacement runs once for each place.",
        "location \"function_definition.body\""
    ),
    within(
        "replacer", "from", REQUIRED,
        "A regex. Every match of it in the text of a place is replaced by `to`.",
        "from r\"^old_(\\w+)\""
    ),
    within(
        "replacer", "to", REQUIRED,
        "What each match of `from` becomes. It can use the groups of `from` as `$[0]` and call other actions as `#[name]`. If a call gives nothing, nothing is replaced at that place.",
        "to \"new_$[0]\""
    ),
    flags!("replacer", "from"),
    within(
        "replacer", "reference-location", "optional, `location` if left out",
        "The text to change when it isn't the place of `location` itself: the smallest place around it, or else the first one inside it.",
        "location \"function_definition.body\"\nreference-location \"function_definition\""
    ),
    within("replacer", "actions", OPTIONAL, INNER, "actions {\n    maker \"upper\" {\n        make \"@[0]\"\n    }\n}"),

    within(
        "maker", "prepare", OPTIONAL,
        "A regex for the first argument. Its groups are `$[0]` in `make`.",
        "prepare r\"(\\w+)=(\\w+)\""
    ),
    flags!("maker", "prepare"),
    within(
        "maker", "make", REQUIRED,
        "The text this maker gives. `@[0]` is the first argument, `$[0]` the first group of `prepare`, and other actions are called as `#[name]`.",
        "make \"set_@[0](@[1])\""
    ),

    within("switch", "cases", REQUIRED, "The texts to pick from, each with its key.", "cases integer=\"int\" boolean=\"bool\""),
    within("switch", "index", REQUIRED, "The key to pick, usually an argument.", "index \"@[0]\""),
    within(
        "switch", "default", OPTIONAL,
        "The text for keys without a case. Without it they give nothing, and nothing is replaced at the place the switch is called for.",
        "default \"void\""
    ),

    within("list", "values", REQUIRED, "The texts to pick from. The first one is at position 0.", "values \"first\" \"second\""),
    within(
        "list", "index", "optional, but without it the list gives nothing",
        "The position to pick, usually an argument.",
        "index \"@[0]\""
    ),
    within(
        "list", "default", OPTIONAL,
        "The text for positions past the end. Without it they give nothing, and nothing is replaced at the place the list is called for.",
        "default \"last\""
    )
];

pub const SETTINGS: &[Explained] = &[
    top("name", REQUIRED, "The project's name.", "name \"Function Names\""),
    top(
        "target", REQUIRED,
        "The kind of target, which finds its files: \"text\" for a folder, \"godot\" for a Godot project. Options go in a block.",
        "target \"text\" {\n    files \"**/*.cpp\"\n}"
    ),
    top(
        "ast-set", REQUIRED,
        "The language of the target's files: one for all of them, \"target\" for the ones the target names, or a block of languages with patterns, where later lines win.",
        "ast-set {\n    cpp \"*.cpp\" \"*.h\"\n    gdscript \"*.gd\"\n}"
    ),
    top("description", REQUIRED, "What the project does, for its readers.", "description \"Renames functions\""),
    top(
        "constant", OPTIONAL,
        "A text of the project, which its patches use as `#[name]`. It can't be set from outside.",
        "constant \"header\" \"Generated, do not edit\""
    ),
    top(
        "variable", OPTIONAL,
        "A text which patches use as `#[name]`, set from outside: with `--set`, by a program that uses CodeShaper, or in the configuration's `variables`. Otherwise it's `default`. A patch that uses it can't run without a value.",
        "variable \"prefix\" default=\"my_\""
    ),
    within(TARGET, "files", "optional, every file if left out", FILES, "target \"text\" {\n    files \"**/*.cpp\" \"main.h\"\n}"),
    within(TARGET, "exclude", OPTIONAL, EXCLUDE, "target \"text\" {\n    exclude \"build/**\"\n}")
];

pub const CONFIGURATION: &[Explained] = &[
    top("name", REQUIRED, "The operation's name.", "name \"Rename the functions\""),
    top(
        "project", REQUIRED,
        "The shaping project's folder, with `project.kdl` and the patches, relative to this file.",
        "project \"shaping_project/\""
    ),
    top(
        "target", REQUIRED,
        "The folder to shape, relative to this file. Options in a block are used instead of the project's.",
        "target \"target/\""
    ),
    top(
        "output", REQUIRED,
        "The folder changed files are written to when `result` is \"create-new\", relative to this file. Not written to otherwise.",
        "output \"target_result/\""
    ),
    top(
        "backup", "optional, needed by \"backup-and-replace\"",
        "The folder the originals are copied to when `result` is \"backup-and-replace\", relative to this file.",
        "backup \"target_backup/\""
    ),
    top(
        "result", REQUIRED,
        "Where results go: \"replace\" changes the target's files, \"backup-and-replace\" copies them to `backup` first, and \"create-new\" writes the changed files to `output` and leaves the target alone.",
        "result \"backup-and-replace\""
    ),
    top(
        "variables", OPTIONAL,
        "Values for the project's variables, used instead of their defaults.",
        "variables prefix=\"x_\""
    ),
    within(TARGET, "files", "optional, the project's if left out", FILES, "target \"target/\" {\n    files \"**/*.cpp\"\n}"),
    within(TARGET, "exclude", "optional, the project's if left out", EXCLUDE, "target \"target/\" {\n    exclude \"build/**\"\n}")
];


pub fn explained<'e>(table: &'e [Explained], parent: Option<&str>, name: &str) -> Option<&'e Explained> {
    table.iter().find(|explained| explained.parent == parent && explained.name == name)
}

pub fn documentation(table: &[Explained], parent: Option<&str>, name: &str) -> Option<Documentation> {
    let value = explained(table, parent, name)?.markdown();

    Some(Documentation::MarkupContent(MarkupContent { kind: MarkupKind::Markdown, value }))
}

pub fn table(path: &Path) -> &'static [Explained] {
    match kind(path) {
        DocumentKind::Patch { .. } => PATCH,
        DocumentKind::Settings => SETTINGS,
        DocumentKind::Configuration => CONFIGURATION,
        DocumentKind::Other => &[]
    }
}

pub fn hover(table: &[Explained], outline: &Outline, offset: usize) -> Option<(String, Range<usize>)> {
    let index = outline.nodes.iter().position(|node| node.name_range.start <= offset && offset <= node.name_range.end)?;
    let node = &outline.nodes[index];
    let is_commented = std::iter::successors(Some(index), |index| outline.nodes[*index].parent).any(|index| outline.nodes[index].commented);
    if is_commented { return None }

    let parent = node.parent.map(|parent| outline.nodes[parent].name.as_str());

    Some((explained(table, parent, &node.name)?.markdown(), node.name_range.clone()))
}

pub fn document(path: &Path, text: &str, offset: usize) -> Option<(String, Range<usize>)> {
    let outline = Outline::parse(text);
    let is_option = outline.nodes.iter().any(|node| {
        node.name_range.start <= offset && offset <= node.name_range.end && node.parent.is_some_and(|parent| outline.is_top(parent, TARGET))
    });
    if is_option && target_kind(path, text).as_deref() != Some(TEXT) { return None }

    hover(table(path), &outline, offset)
}
