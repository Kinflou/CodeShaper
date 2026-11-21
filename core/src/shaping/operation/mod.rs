// Relative Modules
pub mod configuration;
pub mod actions;

// Standard Uses
use std::path::Path;
use std::time::Instant;

// Crate Uses
use crate::shaping::operation::configuration::OperationConfiguration;
use crate::shaping::project::Project;

// External Uses
use miette::Result;


pub struct Operation {
    pub configuration: OperationConfiguration,
    pub started_at: Option<Instant>,
    pub project: Option<Project>,
}


impl Operation {
    pub fn from_configuration(configuration: OperationConfiguration) -> Result<Self> {
        let path = Path::new(configuration.project.as_str());

        let project = Project::from_directory(path)?;

        Ok(Self {
            configuration,
            started_at: None,
            project: Some(project),
        })
    }

    pub fn start(&mut self) {
        self.started_at = Some(Instant::now());
        // self.project.unwrap().file_map.target_file.controller;
    }

    pub fn stop(&self) {
        todo!()
    }

    pub fn next(&self) {
        todo!()
    }
}

