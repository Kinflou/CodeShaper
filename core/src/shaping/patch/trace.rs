// Standard Uses
use std::collections::{BTreeSet, HashMap};
use std::ops::Range;
use std::path::{Path, PathBuf};

// Crate Uses

// External Uses
use regex::{Captures, Match};


#[derive(Debug, Default, Clone, PartialEq)]
pub struct Sample {
    pub file: PathBuf,
    pub input: String,
    pub matched: bool,
    pub groups: Vec<Option<String>>,
    pub output: String,
    pub place: Option<Range<usize>>
}

#[derive(Debug, Default, Clone, PartialEq)]
pub struct Found {
    pub file: PathBuf,
    pub matched: Capture,
    pub groups: Vec<Option<Capture>>
}

#[derive(Debug, Default, Clone, PartialEq)]
pub struct Capture {
    pub text: String,
    pub place: Option<Range<usize>>
}

#[derive(Debug, Default, Clone, PartialEq)]
pub struct ActionTrace {
    pub inputs: usize,
    pub unreferred: usize,
    pub matched: usize,
    pub matches: usize,
    pub built: usize,
    pub matched_files: usize,
    pub samples: Vec<Sample>,
    pub found: Vec<Found>,
    pub calls: usize,
    pub passed: Vec<usize>,
    pub results: Vec<Sample>,
    pub keys: BTreeSet<String>
}

#[derive(Debug, Default, Clone, PartialEq)]
pub struct Trace {
    pub limit: usize,
    pub directory: PathBuf,
    pub files: usize,
    pub actions: HashMap<String, ActionTrace>
}


impl ActionTrace {
    pub fn input(&mut self, matches: usize) {
        self.inputs += 1;
        self.matches += matches;
        self.matched += usize::from(matches > 0);
    }

    pub fn sample(&mut self, limit: usize, sample: Sample) {
        if self.samples.len() < limit { self.samples.push(sample) }
    }

    pub fn found(&mut self, limit: usize, found: impl FnOnce() -> Found) {
        if self.found.len() >= limit { return }

        let found = found();
        if !self.found.contains(&found) { self.found.push(found) }
    }

    pub fn call(&mut self, limit: usize, result: Sample) {
        self.calls += 1;
        self.pass(&vec![1; result.groups.len()]);
        if self.results.len() < limit { self.results.push(result) }
    }

    fn pass(&mut self, counts: &[usize]) {
        self.passed.resize(self.passed.len().max(counts.len()), 0);

        for (total, count) in self.passed.iter_mut().zip(counts) {
            *total += count;
        }
    }

    fn merge(&mut self, other: ActionTrace, file: &Path, limit: usize) {
        self.inputs += other.inputs;
        self.unreferred += other.unreferred;
        self.matched += other.matched;
        self.matches += other.matches;
        self.built += other.built;
        self.matched_files += usize::from(other.matches > 0);
        self.calls += other.calls;
        self.pass(&other.passed);
        self.keys.extend(other.keys);

        for sample in other.samples {
            self.sample(limit, Sample { file: file.to_path_buf(), ..sample });
        }
        for found in other.found {
            if self.found.len() < limit { self.found.push(Found { file: file.to_path_buf(), ..found }) }
        }
        for result in other.results {
            if self.results.len() < limit { self.results.push(Sample { file: file.to_path_buf(), ..result }) }
        }
    }
}

impl Trace {
    pub fn new(limit: usize) -> Self {
        Self { limit, ..Self::default() }
    }

    pub fn action(&self, path: &str) -> Option<&ActionTrace> {
        self.actions.get(path)
    }

    pub fn merge(&mut self, other: Trace, file: &Path) {
        self.files += 1;

        for (path, action) in other.actions {
            self.actions.entry(path).or_default().merge(action, file, self.limit);
        }
    }
}


impl Found {
    pub fn new(captures: &Captures, place: impl Fn(Range<usize>) -> Option<Range<usize>>) -> Self {
        let capture = |group: Match| Capture { text: group.as_str().to_string(), place: place(group.range()) };
        let mut groups: Vec<Option<Capture>> = captures.iter().map(|group| group.map(capture)).collect();
        let matched = groups.remove(0).unwrap_or_default();

        Self { file: PathBuf::new(), matched, groups }
    }
}


pub fn groups(captures: &Captures) -> Vec<Option<String>> {
    captures.iter().skip(1).map(|group| group.map(|group| group.as_str().to_string())).collect()
}
