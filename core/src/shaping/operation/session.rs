// Standard Uses
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

// Crate Uses
use crate::registry::Registry;
use crate::shaping::operation::{Operation, RunFailed};
use crate::shaping::operation::report::{FileReport, Outcome, Report};
use crate::target::{File, Solution};

// External Uses
use miette::{Context, IntoDiagnostic, Result};


#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FileState {
    Untouched,
    Waiting,
    Processing,
    Processed,
    Error
}

#[derive(Debug)]
pub struct Change {
    pub file: PathBuf,
    pub patch: String,
    pub before: String,
    pub result: Result<String, miette::Report>
}


pub struct Session<'a> {
    operation: &'a Operation,
    registry: &'a Registry,
    directory: PathBuf,
    files: Vec<File>,
    steps: Vec<(usize, String)>,
    history: Vec<Change>,
    cursor: usize,
    contents: Vec<Option<String>>,
    started_at: Option<Instant>,
    elapsed: Duration
}

impl<'a> Session<'a> {
    pub(crate) fn new(operation: &'a Operation, registry: &'a Registry, solution: Solution) -> Result<Self> {
        let files: Vec<File> = solution.files().into_iter().cloned().collect();
        let mut steps = vec![];

        for (index, file) in files.iter().enumerate() {
            for name in operation.matching_patches(file)? {
                steps.push((index, name.clone()));
            }
        }

        Ok(Self {
            operation,
            registry,
            directory: solution.directory,
            contents: vec![None; files.len()],
            files,
            steps,
            history: vec![],
            cursor: 0,
            started_at: None,
            elapsed: Duration::ZERO
        })
    }

    pub fn start(&mut self) {
        if self.started_at.is_none() {
            self.started_at = Some(Instant::now());
        }
    }

    pub fn stop(&mut self) {
        if let Some(started_at) = self.started_at.take() {
            self.elapsed += started_at.elapsed();
        }
    }

    pub fn is_running(&self) -> bool {
        self.started_at.is_some()
    }

    pub fn elapsed(&self) -> Duration {
        self.elapsed + self.started_at.map(|started_at| started_at.elapsed()).unwrap_or_default()
    }

    #[allow(clippy::should_implement_trait)]
    pub fn next(&mut self) -> Option<&Change> {
        if self.cursor >= self.steps.len() {
            return None
        }

        if self.cursor == self.history.len() {
            let change = self.apply_step(self.cursor);
            self.history.push(change);
        }

        let index = self.steps[self.cursor].0;
        if let Ok(after) = &self.history[self.cursor].result {
            self.contents[index] = Some(after.clone());
        }

        self.cursor += 1;

        Some(&self.history[self.cursor - 1])
    }

    pub fn previous(&mut self) -> Option<&Change> {
        if self.cursor == 0 {
            return None
        }

        self.cursor -= 1;

        let index = self.steps[self.cursor].0;
        let change = &self.history[self.cursor];
        if change.result.is_ok() {
            self.contents[index] = Some(change.before.clone());
        }

        Some(change)
    }

    pub fn files(&self) -> Vec<(&Path, FileState)> {
        self.files.iter().enumerate()
            .map(|(index, file)| (file.path.as_path(), self.state(index)))
            .collect()
    }

    pub fn state(&self, index: usize) -> FileState {
        let (mut total, mut applied, mut failed) = (0, 0, false);

        for (step, (file, _)) in self.steps.iter().enumerate() {
            if *file != index { continue }

            total += 1;

            if step < self.cursor {
                applied += 1;
                failed |= self.history[step].result.is_err();
            }
        }

        match () {
            _ if total == 0 => FileState::Untouched,
            _ if failed => FileState::Error,
            _ if applied == 0 => FileState::Waiting,
            _ if applied < total => FileState::Processing,
            _ => FileState::Processed
        }
    }

    pub fn content(&self, index: usize) -> Option<&str> {
        self.contents.get(index)?.as_deref()
    }

    pub fn applied(&self) -> &[Change] {
        &self.history[..self.cursor]
    }

    pub fn position(&self) -> usize {
        self.cursor
    }

    pub fn len(&self) -> usize {
        self.steps.len()
    }

    pub fn is_empty(&self) -> bool {
        self.steps.is_empty()
    }

    pub fn finish(mut self) -> Result<Report> {
        while self.next().is_some() {}
        self.stop();

        let mut patched: Vec<Vec<String>> = vec![vec![]; self.files.len()];
        let mut errors = vec![];

        for ((index, _), change) in self.steps.iter().zip(self.history.drain(..)) {
            match change.result {
                Ok(after) if after != change.before => patched[*index].push(change.patch),
                Ok(_) => {}
                Err(error) => errors.push(error)
            }
        }

        if !errors.is_empty() {
            return Err(RunFailed { errors }.into());
        }

        let mut report = Report::default();

        for (index, (file, patches)) in self.files.iter().zip(patched).enumerate() {
            let outcome = if patches.is_empty() {
                Outcome::Unchanged
            } else {
                let content = self.contents[index].as_deref().unwrap_or_default();
                self.operation.write(&self.directory, &file.path, content)?;

                Outcome::Patched(patches)
            };

            report.files.push(FileReport { path: file.path.clone(), outcome });
        }

        Ok(report)
    }

    fn apply_step(&mut self, step: usize) -> Change {
        let (index, patch) = self.steps[step].clone();
        let file = &self.files[index];

        let before = match &self.contents[index] {
            Some(content) => Ok(content.clone()),
            None => {
                let path = self.directory.join(&file.path);

                fs::read_to_string(&path)
                    .into_diagnostic()
                    .wrap_err_with(|| format!("Could not read {}", path.display()))
            }
        };

        let (before, result) = match before {
            Ok(before) => {
                let result = self.operation.apply_patch(self.registry, &patch, file, &before);
                (before, result)
            }
            Err(error) => (String::new(), Err(error))
        };

        if self.contents[index].is_none() && result.is_ok() {
            self.contents[index] = Some(before.clone());
        }

        Change { file: file.path.clone(), patch, before, result }
    }
}
