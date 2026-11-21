// Standard Uses
use std::cell::RefCell;
use std::rc::Rc;

// Crate Uses
use crate::target::{File, Target};

// External Uses


#[allow(unused)]
pub struct FileMap {
    target: Rc<RefCell<dyn Target>>,

    pub target_file: Option<Rc<RefCell<dyn File>>>,
}

impl FileMap {
    pub fn from_target(target: Rc<RefCell<dyn Target>>) -> Self {
        Self {
            target,
            target_file: None,
        }
    }
}

