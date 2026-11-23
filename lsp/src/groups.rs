// Standard Uses
use std::iter;
use std::ops::Range;

// Crate Uses
use crate::schema::{regex_field, template_field};
use crate::syntax::{is_name, Literal, Outline};

// External Uses
use codeshaper::core::shaping::patch::pattern::{self, Group};
use codeshaper::core::shaping::patch::source::literal_offsets;


#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Link {
    pub group: Range<usize>,
    pub uses: Vec<Range<usize>>
}

impl Link {
    pub fn ranges(&self) -> Vec<Range<usize>> {
        iter::once(self.group.clone()).chain(self.uses.clone()).collect()
    }
}


pub fn groups(outline: &Outline, action: usize) -> Vec<Group> {
    let Some(literal) = regex_field(&outline.nodes[action].name).and_then(|field| outline.field(action, field)) else {
        return vec![]
    };
    let flags = outline.field(action, "flags").map(|flags| flags.value.as_str()).unwrap_or_default();

    pattern::groups(&literal.value, flags).unwrap_or_default()
}

pub fn group_number(groups: &[Group], variable: &str) -> Option<usize> {
    match variable.parse::<usize>() {
        Ok(number) => Some(number).filter(|number| *number < groups.len()),
        Err(_) => groups.iter().position(|group| group.name.as_deref() == Some(variable))
    }
}

pub fn locals(text: &str, literal: &Literal) -> Vec<(String, Range<usize>)> {
    let content = &text[literal.content.clone()];

    content.match_indices("$[")
        .filter_map(|(index, _)| {
            let start = literal.content.start + index;
            let name = &text[start + 2..literal.content.end];
            let length = name.find(|character: char| !is_name(character)).unwrap_or(name.len());
            let end = start + 2 + length + usize::from(name[length..].starts_with(']'));

            (length > 0).then(|| (name[..length].to_string(), start..end))
        })
        .collect()
}

pub fn links(outline: &Outline, text: &str, action: usize) -> Option<Vec<Link>> {
    let kind = outline.nodes[action].name.as_str();
    let regex = outline.field(action, regex_field(kind)?)?;
    let template = template_field(kind)
        .and_then(|field| outline.field(action, field))
        .map(|literal| locals(text, literal))
        .unwrap_or_default();

    let groups = groups(outline, action);
    let offsets = literal_offsets(&text[regex.range.clone()]);

    groups.iter().enumerate()
        .map(|(number, group)| {
            let start = regex.range.start + offsets.get(group.range.start)?;
            let end = regex.range.start + offsets.get(group.range.end)?;
            let uses = template.iter()
                .filter(|(variable, _)| group_number(&groups, variable) == Some(number))
                .map(|(_, range)| range.clone())
                .collect();

            Some(Link { group: start..end, uses })
        })
        .collect()
}

pub fn link_at(outline: &Outline, text: &str, offset: usize) -> Option<Link> {
    let (node, literal) = outline.literal_at(offset)?;
    let action = outline.nodes[node].parent?;
    let kind = outline.nodes[action].name.as_str();
    let links = links(outline, text, action)?;

    if regex_field(kind) == Some(outline.nodes[node].name.as_str()) {
        return links.into_iter()
            .filter(|link| link.group.start <= offset && offset <= link.group.end)
            .min_by_key(|link| link.group.len())
    }

    let (variable, _) = locals(text, literal).into_iter().find(|(_, range)| range.start <= offset && offset <= range.end)?;
    let number = group_number(&groups(outline, action), &variable)?;

    links.into_iter().nth(number)
}
