// Relative Modules
pub(crate) mod composition;

// Standard Uses
use std::collections::HashMap;

// Crate Uses
use crate::shaping::operation::actions::Action;
use crate::shaping::patch::controller::PatchController;

// External Uses
use anyhow::{bail, Result};


pub trait Expression {

}


#[allow(unused)]
fn search_type(controller: PatchController, expression: &str, parent: &Box<dyn Action>)
    -> Result<Vec<Box<dyn Action>>> {

    let mut expressions = vec!();

    if composition::has_expression(expression) { bail!("") }

    for (name, r#_type) in get_expressions_and_names(controller, &expression, parent) {
        let action = parent.controller().find_action(&name);

        expressions.push(action);
    }

    Ok(expressions)
}


#[allow(unused)]
fn get_expressions_and_names(controller: PatchController, expression: &str, _parent: &Box<dyn Action>)
    -> HashMap<String, Box<dyn Action>> {
    let mut actions = HashMap::new();

    for name in composition::get_expression_names(&expression) {
        let action = controller.find_action(&name);

        actions.insert(name, action);
    }

    actions
}

