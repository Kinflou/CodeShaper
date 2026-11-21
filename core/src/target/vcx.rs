// Standard Uses
use std::path::Path;
use std::rc::Rc;
use std::cell::RefCell;

// Crate Uses
use crate::target::{Group, Target, TargetAlias};
use crate::shaping::operation::Operation;

// External Uses


pub struct VCXSolution {}

#[allow(unused)]
impl Target for VCXSolution {
    fn name(&self) -> &String { todo!() }

    fn groups(&self) -> &Vec<Rc<RefCell<dyn Group>>> { todo!() }

    fn operation(&self) -> &Option<Operation> { todo!() }

    fn ast_set(&self) { todo!() }

    fn add_group(&mut self, group: Rc<RefCell<dyn Group>>) {
        todo!()
    }
}

impl TargetAlias for VCXSolution {
    const ALIAS: &'static str = "vcx";
}


#[allow(unused)]
impl VCXSolution {
    pub fn from_target(target: &Path) -> Self {
        todo!()
    }
}

