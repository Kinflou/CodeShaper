// Standard Uses

// Crate Uses
use crate::shaping::operation::actions::Action;
use crate::expressions::visitor;
use crate::expressions::visitor::ExpressionVisitor;

// External Uses


pub struct Linter {
    pub action: Box<dyn Action>,
    // pub suggestions: Vec<Suggestion>,
    pub warnings: Vec<Warning>
}


#[allow(unused)]
impl Linter {
    pub fn with_action(action: Box<dyn Action>) -> Self {
        Self {
            action,
            warnings: vec![],
        }
    }

    pub fn lint(&mut self) {
        let expr = self.action.expression().to_string();

        // TODO: Uncomment below
        // visitor::navigate_expression(self, &expr);
    }
}

impl ExpressionVisitor for Linter {}


#[derive(Debug, PartialEq)]
pub enum Warning {
    UnusedAction
}

/*
#[derive(Debug)]
pub enum Suggestion {
}
*/
