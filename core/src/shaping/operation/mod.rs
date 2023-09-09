// Relative Modules
pub mod configuration;
pub mod actions;

// Standard Uses
use std::path::Path;

// Crate Uses
use crate::shaping::operation::configuration::OperationConfiguration;
use crate::target::text::project::Project;

// External Uses
use eyre::{Result};
use stopwatch::Stopwatch;


pub struct Operation {
    pub configuration: OperationConfiguration,
    pub stop_watch: Stopwatch,
    pub project: Option<Project>,
}


impl Operation {
    pub fn from_configuration(configuration: OperationConfiguration) -> Result<Self> {
        let path = Path::new(configuration.project.as_str());

        let project = Project::from_path(path)?;

        Ok(Self {
            configuration,
            stop_watch: Default::default(),
            project: Some(project),
        })
    }

    pub fn start(&mut self) {
        // self.project.unwrap().file_map.target_file.controller;
    }

    pub fn stop(&self) {
        todo!()
    }

    pub fn next(&self) {
        todo!()
    }
}

