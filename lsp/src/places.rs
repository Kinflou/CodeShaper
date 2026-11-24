// Standard Uses
use std::collections::HashMap;
use std::fs;
use std::ops::Range;
use std::path::{Path, PathBuf};

// Crate Uses
use crate::position::LineIndex;
use crate::trace::{fenced, inline, truncate, Limits};

// External Uses
use codeshaper::core::shaping::patch::trace::Capture;
use url::Url;


pub struct Places<'t> {
    directory: &'t Path,
    limits: &'t Limits,
    sources: HashMap<PathBuf, Option<String>>
}

impl<'t> Places<'t> {
    pub fn new(directory: &'t Path, limits: &'t Limits) -> Self {
        Self { directory, limits, sources: HashMap::new() }
    }

    pub fn file(&mut self, file: &Path, place: Option<&Range<usize>>) -> String {
        self.link(file, place).unwrap_or_else(|| format!("`{}`, changed by an earlier replacer", file.display()))
    }

    pub fn link(&mut self, file: &Path, place: Option<&Range<usize>>) -> Option<String> {
        let (span, url) = self.locate(file, place)?;

        Some(format!("[`{}:{span}`](<{url}>)", file.display()))
    }

    pub fn capture(&mut self, file: &Path, capture: &Capture, language: &str) -> String {
        let place = self.locate(file, capture.place.as_ref())
            .map(|(span, url)| format!(" [`{span}`](<{url}>)"))
            .unwrap_or_default();

        if !capture.text.trim().contains('\n') { return format!(" = {}{place}", self.text(&capture.text)) }

        let snippet = fenced(language, &truncate(&capture.text, self.limits));
        let indented: String = snippet.lines().map(|line| format!("\n  {line}")).collect();

        place + &indented
    }

    pub fn value(&mut self, file: &Path, capture: &Capture) -> String {
        let text = self.text(&capture.text);

        match self.locate(file, capture.place.as_ref()) {
            Some((_, url)) => format!("[{text}](<{url}>)"),
            None => text
        }
    }

    pub fn text(&self, text: &str) -> String {
        let whole = text.lines().count() <= self.limits.lines && text.chars().count() <= self.limits.characters;

        match text {
            "" => "empty".to_string(),
            _ if whole => inline(text),
            _ => inline(&truncate(text, self.limits))
        }
    }

    fn locate(&mut self, file: &Path, place: Option<&Range<usize>>) -> Option<(String, Url)> {
        let path = self.directory.join(file);
        let mut url = Url::from_file_path(&path).ok()?;
        let source = self.sources.entry(path).or_insert_with_key(|path| fs::read_to_string(path).ok()).as_ref()?;

        let range = LineIndex::new(source).range(place?.clone());
        let (line, column) = (range.start.line + 1, range.start.character + 1);
        let (last, end) = (range.end.line + 1, range.end.character + 1);

        url.set_fragment(Some(&format!("L{line},{column}-L{last},{end}")));
        let span = match last == line {
            true => format!("{line}:{column}-{end}"),
            false => format!("{line}:{column}-{last}:{end}")
        };

        Some((span, url))
    }
}
