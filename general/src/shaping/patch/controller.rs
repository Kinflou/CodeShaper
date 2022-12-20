// Standard Uses
use std::rc::Rc;
use std::cell::RefCell;

// Crate Uses
use crate::ast;
use crate::shaping::operation::actions::Action;
use crate::shaping::patch::Patch;

// External Uses


#[allow(unused)]
#[derive(Default)]
pub struct Controller {
    pub listener_controller: Rc<RefCell<ast::listener::Controller>>,
    pub patch: Rc<Patch>,
    pub actions: Vec<Rc<RefCell<dyn Action>>>
}

#[allow(unused)]
impl Controller {
    pub fn from_patch(patch: Patch) -> Self {
        todo!()
    }

    pub fn visit(&mut self) {
        self.actions.iter_mut().map(|action|
            RefCell::borrow_mut(action).process()
        );
    }

    pub fn find_action(&self, name: &str) -> Option<Rc<RefCell<dyn Action>>> {
        for action in &self.actions {
            let action_bm = action.borrow_mut();

            if action_bm.name() == name {
                return Some(Rc::clone(&action))
            }

            let Some(action) = action_bm.find_inner(name) else {
                continue
            };

            if action_bm.find_inner(&name).is_some() {
                return action_bm.find_inner(&name)
            }
        }

        None
    }

    pub fn possible_rules(&self) -> Vec<&'static str> {
        vec!["*"]
    }
}
