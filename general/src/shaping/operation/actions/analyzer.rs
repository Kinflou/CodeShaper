// Standard Uses
use std::cell::RefCell;

// Crate Uses
use crate::shaping::operation::actions::Action;
use crate::shaping::operation::expressions::common::Expression;
use crate::shaping::operation::expressions::visitor;
use crate::shaping::operation::expressions::visitor::ExpressionVisitor;

// External Uses


pub struct Analyzer<'a> {
    pub action: &'a Box<dyn Action>,
    pub current_location: Option<String>,

    pub found_actions: Vec<String>,
    pub errors: Vec<Error>,
}

#[allow(unused)]
impl<'a> Analyzer<'a> {
    pub fn with_action_shared_mut(action: &'a mut Box<dyn Action>) -> Self {
        Self {
            action,
            current_location: None,
            found_actions: vec![],
            errors: vec![],
        }
    }

    pub fn analyze(&mut self) {
        let expr = self.action.expression();
        visitor::navigate_expression(self, &expr);
    }
}

#[allow(unused)]
impl<'a> ExpressionVisitor for Analyzer<'a> {
    fn visit_expression(&mut self, expr: &Expression) {
        let Expression::Expr(exp) = expr else {
            unreachable!()
        };

        let Some(action) = self.action.find_inner(exp.0) else {
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
