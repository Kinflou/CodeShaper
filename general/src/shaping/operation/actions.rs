// Relative Modules
pub mod builder;
pub mod replacer;
pub mod resolver;
pub mod maker;
pub mod analyzer;
pub mod linter;
pub mod composer;

// Standard Uses
use std::rc::{Rc, Weak};
use std::cell::RefCell;

// Create Uses
use crate::ast;
use crate::shaping::patch;

// External Uses


#[allow(unused)]
pub trait Action {
    fn name(&self) -> &str;
    fn parent(&self) -> Option<Weak<dyn Action>>;

    /*
    fn root(&self) -> Option<Weak<dyn Action>> {
        let mut current = Rc::downgrade(&self);
        while let Some(parent) = self.parent() { current = parent; }
        Some(current)
    }
    */

    fn root(&self) -> Option<Weak<dyn Action>> {
        let mut current = self.parent();

        if current.is_none() {
            return None
        }

        while let Some(parent) = self.parent() {
            current = Some(parent);
        }

        current
    }

    fn children(&self) -> &Vec<Rc<RefCell<dyn Action>>>;
    fn patch_controller(&self) -> Option<Weak<patch::controller::Controller>>;
    fn listener_controller(&self) -> Option<Weak<ast::listener::Controller>>;
    fn expression(&self) -> String;
    fn args_count(&self) -> usize { 0 }

    fn process(&mut self) -> Option<String>;

    fn find_from_root_mut(&self, name: &str) -> Option<Rc<RefCell<dyn Action>>> {
        self.root()?.upgrade()?.find_inner(name)
    }

    fn find_inner(&self, name: &str) -> Option<Rc<RefCell<dyn Action>>> {
        let actions = self.children();

        for action in self.children() {
            if action.borrow().name() == name {
                return Some(Rc::clone(action))
            }
        }

        for action in self.children() {
            let act = action.borrow().find_inner(name);

            if act.is_some() { return act }
        }

        None
    }
}

