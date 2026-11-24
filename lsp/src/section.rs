// Standard Uses
use std::borrow::Cow;
use std::ops::Range;

// Crate Uses
use crate::syntax::Outline;

// External Uses
use codeshaper::core::shaping::patch::file::BLOCK;


#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Section {
    pub name: Option<String>,
    pub named: Range<usize>,
    pub whole: Range<usize>,
    pub content: Range<usize>
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct View<'t> {
    pub section: Option<Section>,
    pub text: Cow<'t, str>
}

impl View<'_> {
    pub fn name(&self) -> Option<&str> {
        self.section.as_ref()?.name.as_deref()
    }
}


pub fn sections(text: &str) -> Vec<Section> {
    if !text.contains(BLOCK) { return vec![] }

    Outline::parse(text).nodes.into_iter()
        .filter(|node| node.parent.is_none() && node.name == BLOCK && !node.commented)
        .map(|node| {
            let name = node.arguments.first();
            let named = name.map(|name| name.range.clone()).unwrap_or(node.name_range.clone());
            let end = node.children.as_ref().map(|children| (children.end + 1).min(text.len())).unwrap_or(named.end);

            Section {
                name: name.filter(|name| name.quoted).map(|name| name.value.clone()),
                whole: node.name_range.start..end,
                content: node.children.unwrap_or(named.end..named.end),
                named
            }
        })
        .collect()
}

pub fn views(text: &str) -> Vec<View<'_>> {
    let sections = sections(text);
    if sections.is_empty() { return vec![View { section: None, text: Cow::Borrowed(text) }] }

    sections.into_iter()
        .map(|section| View { text: Cow::Owned(only(text, &section.content)), section: Some(section) })
        .collect()
}

pub fn view_at(text: &str, offset: usize) -> View<'_> {
    let section = sections(text).into_iter()
        .find(|section| section.content.start <= offset && offset <= section.content.end);

    match section {
        Some(section) => View { text: Cow::Owned(only(text, &section.content)), section: Some(section) },
        None => View { section: None, text: Cow::Borrowed(text) }
    }
}

pub fn only(text: &str, content: &Range<usize>) -> String {
    [blank(&text[..content.start]).as_str(), &text[content.clone()], blank(&text[content.end..]).as_str()].concat()
}

fn blank(text: &str) -> String {
    text.chars()
        .map(|character| match (character, character.len_utf8()) {
            ('\n', _) => "\n",
            ('\r', _) => "\r",
            (_, 1) => " ",
            (_, 2) => "\u{a0}",
            (_, 3) => "\u{3000}",
            _ => "\u{a0}\u{a0}"
        })
        .collect()
}
