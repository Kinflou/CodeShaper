// Relative Modules
pub mod vcx;
pub mod text;
pub mod file_map;

// Standard Uses
use std::path::Path;
use std::rc::{Rc, Weak};
use std::cell::RefCell;
use std::fmt::Debug;

// Crate Uses
use crate::target::vcx::VCXSolution;
use crate::ast::listener::Controller;
use crate::shaping::operation::Operation;

// External Uses
use anyhow::{bail, Result};
use crate::target::text::TextSolution;


pub trait File {
    fn name(&self) -> &String;
    fn parent(&self) -> &Option<Box<dyn Group>>;
    fn controller(&self) -> &Option<Controller>;
    fn state(&self) -> &FileState;
}

pub trait Target {
    fn name(&self) -> &String;
    fn groups(&self) -> &Vec<Weak<RefCell<dyn Group>>>;
    fn operation(&self) -> &Option<Operation>;
    fn ast_set(&self);

    fn add_group(&mut self, group: Weak<RefCell<dyn Group>>);
}

pub trait TargetAlias: Target {
    const ALIAS: &'static str;
}

pub trait Group: Debug {
    fn name(&self) -> &String;
    fn root(&self) -> &Option<Rc<RefCell<dyn Target>>>;
    fn parent(&self) -> &Option<Box<dyn Group>>;
    fn groups(&self) -> &Vec<Weak<RefCell<dyn Group>>>;
    fn files(&self) -> &Vec<Weak<RefCell<dyn File>>>;

    fn add_root(&mut self, root: Rc<RefCell<dyn Target>>);
    fn add_file(&mut self, file: Weak<RefCell<dyn File>>);
}


#[doc(hidden)]
pub fn from_type(r#type: &str, path: &Path) -> Result<Rc<RefCell<dyn Target>>> {
    let target: Rc<RefCell<dyn Target>> = match r#type {
        "text" => Rc::new(RefCell::new(TextSolution::from_target(path))),
        "vsx" => Rc::new(RefCell::new(VCXSolution::from_target(path))),
        _ => bail!("The target type '{}' is not supported", r#type)
    };

    Ok(target)
}


#[derive(Debug, Clone)]
#[derive(Default)]
pub enum FileState {
    #[default]
    Untouched,
    Waiting,
    Processing,
    Processed,
    Error
}

