// Standard Uses
use std::cell::RefCell;
use std::rc::{Rc, Weak};

// Crate Uses
use crate::ast;
use crate::shaping::patch;
use crate::shaping::patch::controller::Controller;
use crate::shaping::operation::expressions::visitor::ExpressionVisitor;
use crate::shaping::operation::actions::{Action, composer};

// External Uses


pub struct Maker {
    pub maker: patch::maker::Maker,
    pub actions: Vec<Rc<RefCell<dyn Action>>>
}


#[allow(unused)]
impl Maker {
    pub fn with(patch_maker: patch::maker::Maker) -> Self {
        Self {
            maker: patch_maker,
            actions: vec![]
        }
    }

    pub fn make(&mut self, composer: Rc<dyn ExpressionVisitor>, input: String) -> Option<String> {
        let locals  = if self.maker.prepare.is_some() {
            composer::capture_locals(
                &*self.maker.prepare.clone().unwrap(), input.clone()
            )
        } else {
            vec![]
        };

        let result = composer::compose_regex(
            self.maker.make.clone(), input.clone(), &locals, &"".to_string()
        );

        result
    }
}

impl Action for Maker {
    fn name(&self) -> &str { &self.maker.name }

    fn parent(&self) -> Option<Weak<dyn Action>> {
        todo!()
    }
    
    fn children(&self) -> &Vec<Rc<RefCell<dyn Action>>> { &self.actions }

    fn patch_controller(&self) -> Option<Weak<Controller>> { todo!() }

    fn listener_controller(&self) -> Option<Weak<ast::listener::Controller>> { todo!() }

    fn expression(&self) -> &str {
        todo!()
    }

    fn process(&mut self) -> Option<String> {
        todo!()
    }
}
