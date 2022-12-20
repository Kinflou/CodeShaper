// Standard Uses
use std::rc::Rc;
use std::cell::RefCell;

// Crate Uses
use crate::shaping::operation::actions::Action;
use crate::shaping::operation::expressions::common::Expression;
use crate::shaping::operation::expressions::visitor;
use crate::shaping::operation::expressions::visitor::ExpressionVisitor;

// External Uses


pub struct Analyzer {
    pub action: Rc<RefCell<dyn Action>>,
    pub current_location: Option<String>,

    pub found_actions: Vec<String>,
    pub errors: Vec<Error>,
}

#[allow(unused)]
impl Analyzer {
    pub fn with_action_shared_mut(action: Rc<RefCell<dyn Action>>) -> Self {
        Self {
            action,
            current_location: None,
            found_actions: vec![],
            errors: vec![],
        }
    }

    pub fn analyze(own: &Rc<RefCell<Analyzer>>) {
        let bor = RefCell::borrow(own);
        let expr = &*bor.action.borrow().expression().to_string();

        drop(bor);

        visitor::navigate_expression(
            Rc::clone(own) as _, expr
        );
    }
}

#[allow(unused)]
impl ExpressionVisitor for Analyzer {
    fn visit_expression(&mut self, expr: &Expression) {
        let Expression::Expr(exp) = expr else {
            unreachable!()
        };

        let Some(action) = self.action.borrow().find_inner(exp.0) else {
            self.errors.push(Error::ActionNotFound);
            return;
        };
        let mut action = RefCell::borrow_mut(&action);
        self.found_actions.push(action.name().to_string());

        let Expression::Arguments(args) = &*exp.1 else {
            unreachable!();
        };

        if action.args_count() != args.len() {
            self.errors.push(Error::IncorrectArgs);
            return;
        }

        action.process();
    }
}


#[derive(Debug, PartialEq)]
pub enum Error {
    LocationNotFound,
    ActionNotFound,
    IncorrectArgs,
    IncorrectRegex,
}
