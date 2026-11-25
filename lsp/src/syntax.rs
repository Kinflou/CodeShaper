// Standard Uses
use std::ops::Range;

// Crate Uses

// External Uses
use codeshaper::core::shaping::lines::unix;
use codeshaper::core::shaping::patch::replacer::Replacer;


pub const ACTIONS: [&str; 4] = ["builder", "maker", "switch", "list"];
pub const ACTION_NODES: [&str; 5] = ["builder", "replacer", "maker", "switch", "list"];


#[derive(Debug, Clone, PartialEq)]
pub struct Literal {
    pub value: String,
    pub range: Range<usize>,
    pub content: Range<usize>,
    pub quoted: bool
}

#[derive(Debug, Clone, PartialEq)]
pub struct Node {
    pub name: String,
    pub name_range: Range<usize>,
    pub parent: Option<usize>,
    pub arguments: Vec<Literal>,
    pub properties: Vec<(String, Literal)>,
    pub children: Option<Range<usize>>,
    pub commented: bool
}

#[derive(Debug, Clone, PartialEq)]
pub enum Context {
    Name { parent: Option<usize>, prefix: Range<usize> },
    Value { node: usize, literal: Literal, before: String },
    None
}

#[derive(Debug, Clone, PartialEq)]
pub struct Entry {
    pub node: usize,
    pub content: Range<usize>,
    pub quoted: bool
}

#[derive(Debug, Default)]
pub struct Outline {
    pub nodes: Vec<Node>
}

impl Outline {
    pub fn parse(text: &str) -> Self {
        Scanner { text, bytes: text.as_bytes(), at: 0, nodes: vec![], stack: vec![], current: None, commented: false }.run()
    }

    pub fn top(&self, name: &str) -> Option<&Node> {
        self.nodes.iter().find(|node| node.parent.is_none() && !node.commented && node.name == name)
    }

    pub fn argument(&self, name: &str) -> Option<&str> {
        self.top(name)?.arguments.first().map(|literal| literal.value.as_str())
    }

    pub fn actions(&self) -> impl Iterator<Item = (&Node, &Literal)> {
        self.nodes.iter()
            .filter(|node| ACTIONS.contains(&node.name.as_str()))
            .filter_map(|node| node.arguments.first().map(|name| (node, name)))
    }

    pub fn literals(&self) -> impl Iterator<Item = (usize, &Literal)> {
        self.nodes.iter().enumerate().flat_map(|(index, node)| {
            node.arguments.iter()
                .chain(node.properties.iter().map(|(_, literal)| literal))
                .map(move |literal| (index, literal))
        })
    }

    pub fn literal_at(&self, offset: usize) -> Option<(usize, &Literal)> {
        self.literals().find(|(_, literal)| literal.range.start <= offset && offset <= literal.range.end)
    }

    pub fn is_top(&self, node: usize, name: &str) -> bool {
        self.nodes[node].parent.is_none() && self.nodes[node].name == name
    }

    pub fn field(&self, action: usize, name: &str) -> Option<&Literal> {
        self.nodes.iter()
            .find(|node| node.parent == Some(action) && node.name == name)
            .and_then(|node| node.arguments.first())
    }

    pub fn action_path(&self, index: usize) -> Option<String> {
        let node = &self.nodes[index];
        let segment = match node.name.as_str() {
            "replacer" => {
                let earlier = self.nodes[..index].iter().filter(|other| other.parent == node.parent && other.name == "replacer").count();
                let name = node.arguments.first().filter(|name| name.quoted).map(|name| name.value.as_str());

                Replacer::label(name, earlier)
            }
            _ => node.arguments.first()?.value.clone()
        };

        match self.owner(index) {
            Some(owner) => Some(format!("{}/{segment}", self.action_path(owner)?)),
            None => Some(segment)
        }
    }

    pub fn owner(&self, index: usize) -> Option<usize> {
        self.nodes[index].parent.and_then(|actions| self.nodes[actions].parent)
    }

    pub fn enclosing(&self, offset: usize) -> Option<usize> {
        self.nodes.iter().enumerate()
            .filter(|(_, node)| node.children.as_ref().is_some_and(|children| children.start <= offset && offset <= children.end))
            .max_by_key(|(_, node)| node.children.as_ref().map(|children| children.start))
            .map(|(index, _)| index)
    }

    pub fn entry(&self, text: &str, offset: usize) -> Option<Entry> {
        let literal = self.literals()
            .find(|(_, literal)| literal.quoted && literal.content.start <= offset && offset <= literal.content.end)
            .map(|(node, literal)| Entry { node, content: literal.content.clone(), quoted: true });

        let entry = literal.or_else(|| {
            let (node, named) = self.nodes.iter()
                .filter_map(|node| Some((node.parent?, node)))
                .find(|(_, node)| node.name_range.start <= offset && offset <= node.name_range.end)?;
            let name = &text[named.name_range.clone()];
            let quoted = name.starts_with('"');
            let closed = quoted && name.len() > 1 && name.ends_with('"');
            let content = named.name_range.start + usize::from(quoted)..named.name_range.end - usize::from(closed);

            (content.start <= offset).then_some(Entry { node, content, quoted })
        });

        match entry {
            Some(entry) => {
                let line = text[entry.content.clone()].find(['\n', '\r']).map(|length| entry.content.start + length);
                let content = entry.content.start..line.unwrap_or(entry.content.end);

                (offset <= content.end).then_some(Entry { content, ..entry })
            }
            None => match self.context(text, offset) {
                Context::Name { parent: Some(node), prefix } => Some(Entry { node, content: prefix, quoted: false }),
                _ => None
            }
        }
    }

    pub fn context(&self, text: &str, offset: usize) -> Context {
        let found = self.literals()
            .find(|(_, literal)| literal.quoted && literal.content.start <= offset && offset <= literal.content.end);

        if let Some((node, literal)) = found {
            let before = text.get(literal.content.start..offset).unwrap_or_default().to_string();
            return Context::Value { node, literal: literal.clone(), before }
        }

        let start = text[..offset].trim_end_matches(is_word).len();
        let line = text[..start].trim_end_matches(|character: char| character.is_whitespace() && character != '\n');

        if line.is_empty() || line.ends_with(['\n', '{', ';']) {
            return Context::Name { parent: self.enclosing(offset), prefix: start..offset }
        }

        Context::None
    }
}


pub fn is_name(character: char) -> bool {
    character.is_ascii_alphanumeric() || matches!(character, '_' | '-' | '!')
}

pub fn is_call(character: char) -> bool {
    is_name(character) || character == '.'
}

fn is_word(character: char) -> bool {
    character.is_alphanumeric() || character == '-' || character == '_'
}

fn ends_word(byte: u8) -> bool {
    byte.is_ascii_whitespace() || b"{}();\"=\\".contains(&byte)
}


struct Scanner<'t> {
    text: &'t str,
    bytes: &'t [u8],
    at: usize,
    nodes: Vec<Node>,
    stack: Vec<Option<usize>>,
    current: Option<usize>,
    commented: bool
}

impl Scanner<'_> {
    fn run(mut self) -> Outline {
        while self.at < self.bytes.len() {
            let next = self.bytes.get(self.at + 1).copied();

            match (self.bytes[self.at], next) {
                (b'\n' | b';', _) => { self.current = None; self.at += 1; }
                (b'/', Some(b'/')) => self.skip_line(),
                (b'/', Some(b'*')) => self.skip_block(),
                (b'/', Some(b'-')) => { self.commented = self.current.is_none(); self.at += 2; }
                (b'\\', _) => self.continue_line(),
                (b'(', _) => self.skip_type(),
                (b'{', _) => {
                    if let Some(node) = self.current {
                        self.nodes[node].children = Some(self.at + 1..self.bytes.len());
                    }
                    self.stack.push(self.current);
                    self.current = None;
                    self.at += 1;
                }
                (b'}', _) => {
                    if let Some(Some(node)) = self.stack.pop() {
                        if let Some(children) = &mut self.nodes[node].children {
                            children.end = self.at;
                        }
                    }
                    self.current = None;
                    self.at += 1;
                }
                (byte, _) if byte.is_ascii_whitespace() => self.at += 1,
                (byte, _) if !byte.is_ascii() && self.space().is_some() => self.at += self.space().unwrap_or(1),
                _ => self.item()
            }
        }

        Outline { nodes: self.nodes }
    }

    fn space(&self) -> Option<usize> {
        self.text[self.at..].chars().next().filter(|character| character.is_whitespace()).map(char::len_utf8)
    }

    fn item(&mut self) {
        let Some(literal) = self.literal() else {
            self.at += 1;
            return
        };

        if self.bytes.get(self.at) == Some(&b'=') && self.current.is_some() {
            self.at += 1;
            if let Some(value) = self.literal() {
                let node = self.current.unwrap();
                self.nodes[node].properties.push((literal.value, value));
            }
            return
        }

        match self.current {
            Some(node) => self.nodes[node].arguments.push(literal),
            None => {
                self.nodes.push(Node {
                    name: literal.value,
                    name_range: literal.range,
                    parent: self.stack.last().copied().flatten(),
                    arguments: vec![],
                    properties: vec![],
                    children: None,
                    commented: std::mem::take(&mut self.commented)
                });
                self.current = Some(self.nodes.len() - 1);
            }
        }
    }

    fn literal(&mut self) -> Option<Literal> {
        let start = self.at;

        match self.bytes.get(start) {
            Some(b'"') => Some(self.quoted(start)),
            Some(b'r') if self.raw_hashes(start + 1).is_some() => Some(self.raw(start)),
            Some(byte) if !ends_word(*byte) => {
                while self.at < self.bytes.len() && !ends_word(self.bytes[self.at]) {
                    self.at += 1;
                }
                let range = start..self.at;
                Some(Literal { value: self.text[range.clone()].to_string(), content: range.clone(), range, quoted: false })
            }
            _ => None
        }
    }

    fn quoted(&mut self, start: usize) -> Literal {
        let mut value = String::new();
        self.at = start + 1;

        while self.at < self.bytes.len() {
            match self.bytes[self.at] {
                b'"' => {
                    self.at += 1;
                    return Literal { value, range: start..self.at, content: start + 1..self.at - 1, quoted: true }
                }
                b'\\' => {
                    let escaped = self.text[self.at + 1..].chars().next();
                    value.push(match escaped {
                        Some('n') => '\n',
                        Some('t') => '\t',
                        Some('r') => '\r',
                        Some(other) => other,
                        None => '\\'
                    });
                    self.at += 1 + escaped.map(char::len_utf8).unwrap_or(0);
                }
                b'\r' if self.bytes.get(self.at + 1) == Some(&b'\n') => self.at += 1,
                _ => {
                    let character = self.text[self.at..].chars().next().unwrap();
                    value.push(character);
                    self.at += character.len_utf8();
                }
            }
        }

        Literal { value, range: start..self.at, content: start + 1..self.at, quoted: true }
    }

    fn raw_hashes(&self, from: usize) -> Option<usize> {
        let hashes = self.bytes[from..].iter().take_while(|byte| **byte == b'#').count();

        (self.bytes.get(from + hashes) == Some(&b'"')).then_some(hashes)
    }

    fn raw(&mut self, start: usize) -> Literal {
        let hashes = self.raw_hashes(start + 1).unwrap_or(0);
        let content_start = start + 2 + hashes;
        let closing = format!("\"{}", "#".repeat(hashes));

        let (content_end, end) = match self.text[content_start..].find(&closing) {
            Some(found) => (content_start + found, content_start + found + closing.len()),
            None => (self.bytes.len(), self.bytes.len())
        };
        self.at = end;

        Literal {
            value: unix(&self.text[content_start..content_end]).into_owned(),
            range: start..end,
            content: content_start..content_end,
            quoted: true
        }
    }

    fn skip_line(&mut self) {
        self.at = self.text[self.at..].find('\n').map(|found| self.at + found).unwrap_or(self.bytes.len());
    }

    fn skip_block(&mut self) {
        let mut depth = 0;

        while self.at < self.bytes.len() {
            match (self.bytes[self.at], self.bytes.get(self.at + 1)) {
                (b'/', Some(b'*')) => { depth += 1; self.at += 2; }
                (b'*', Some(b'/')) => {
                    depth -= 1;
                    self.at += 2;
                    if depth == 0 { return }
                }
                _ => self.at += 1
            }
        }
    }

    fn continue_line(&mut self) {
        self.at += 1;
        while self.at < self.bytes.len() && matches!(self.bytes[self.at], b' ' | b'\t' | b'\r') {
            self.at += 1;
        }
        if self.bytes.get(self.at) == Some(&b'\n') {
            self.at += 1;
        }
    }

    fn skip_type(&mut self) {
        self.at = self.text[self.at..].find(')').map(|found| self.at + found + 1).unwrap_or(self.bytes.len());
    }
}
