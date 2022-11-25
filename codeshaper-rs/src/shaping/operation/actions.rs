// Relative Modules
pub mod builder;

// Standard Uses

// Create Uses
use crate::shaping::operation::expressions::Expression;
use crate::shaping::patch::controller::PatchController;
use crate::ast::listener::Controller;

// External Uses


pub trait Action {
    fn name(&self) -> &str;
    fn actions(&self) -> Vec<Box<dyn Action>>;
    fn controller(&self) -> &PatchController;
    fn expression(&self) -> &str;
    fn result(&self) -> &str;

    fn expressions(&self) -> Vec<Box<dyn Expression>>;

    fn process(&mut self, controller: &Controller);

    fn process_expression(&mut self);
}

