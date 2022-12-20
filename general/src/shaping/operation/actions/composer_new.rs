// Standard Uses
use std::rc::Rc;
use std::option::Option;
use std::cell::RefCell;

// Crate Uses
use crate::shaping::operation::actions::{Action, composer};
use crate::shaping::operation::expressions::common::Expression;
use crate::shaping::operation::expressions::visitor::ExpressionVisitor;

// External Uses


#[allow(unused)]
pub struct Composer {
    pub action: Rc<dyn Action>,
    pub input: Option<String>,
    pub locals: Vec<(String, String)>
}

#[allow(unused)]
impl Composer {
    pub fn with_action_shared(action: Rc<dyn Action>) -> Self{
        Self {
            action,
            input: None,
            locals: vec![],
        }
    }
}

#[allow(unused)]
impl ExpressionVisitor for Composer {
    fn visit_expression(&mut self, expr: &Expression) {
        let Expression::Expr((name, args)) = expr else {
            unreachable!()
        };

        let Some(action) = self.action.find_inner(name) else {
            return;
        };
        let mut action = RefCell::borrow_mut(&action);
    }

    fn visit_text(&mut self, text: &Expression) {
        let Expression::Text(text) = text else {
            unreachable!()
        };

        self.locals.append(
            &mut composer::capture_locals(text, self.input.clone().unwrap())
        );
    }
}