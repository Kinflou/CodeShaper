// Standard Uses
use std::collections::{BTreeSet, HashMap};
use std::path::{Path, PathBuf};

// Crate Uses

// External Uses
use regex::Regex;


#[derive(Debug, Default, Clone, PartialEq)]
pub struct Sample {
    pub file: PathBuf,
    pub input: String,
    pub matched: bool,
    pub groups: Vec<Option<String>>,
    pub output: String
}

#[derive(Debug, Default, Clone, PartialEq)]
pub struct ActionTrace {
    pub inputs: usize,
    pub matched: usize,
    pub matches: usize,
    pub matched_files: usize,
    pub samples: Vec<Sample>,
    pub calls: usize,
    pub passed: Vec<usize>,
    pub results: Vec<Sample>,
    pub keys: BTreeSet<String>
}

#[derive(Debug, Default, Clone, PartialEq)]
pub struct Trace {
    pub limit: usize,
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
        self.matched += other.matched;
        self.matches += other.matches;
        self.matched_files += usize::from(other.matches > 0);
        self.calls += other.calls;
        self.pass(&other.passed);
        self.keys.extend(other.keys);

        for sample in other.samples {
            self.sample(limit, Sample { file: file.to_path_buf(), ..sample });
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


pub fn groups(regex: &Regex, input: &str) -> Vec<Option<String>> {
    regex.captures(input)
        .map(|captures| captures.iter().skip(1).map(|group| group.map(|group| group.as_str().to_string())).collect())
        .unwrap_or_default()
}
