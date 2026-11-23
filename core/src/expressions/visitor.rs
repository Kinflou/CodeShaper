// Standard Uses

// Crate Uses
use crate::expressions::syntax::{ExpressionAST, parser};
use crate::shaping::patch::controller::PatchController;

// External Uses
use miette::{miette, Result};


#[allow(unused)]
pub trait ExpressionVisitor {
    // fn visit_expression(&self, controller: &PatchController, name: &str, arguments: &[ExpressionAST]);

    fn visit_action_call(
        &self, controller: &PatchController, name: &str, arguments: &[ExpressionAST]
    ) -> Option<String>;

    fn visit_text(
        &self, controller: &PatchController, text: &str
    ) -> Option<String>;

    fn visit_local_call(
        &self, controller: &PatchController, variable: &str
    ) -> Option<String>;

    fn visit_argument_call(
        &self, controller: &PatchController, variable: &str, arguments: &[String]
    ) -> Option<String>;
}

pub fn navigate_expression<'a>(
    visitor: &'a dyn ExpressionVisitor, patch_controller: &PatchController,
    input: &'a str, arguments: &[String]
) -> Result<String> {
    let expr = parser::parse_expression(input)?;

    visit_expression(visitor, patch_controller, &expr, arguments).ok_or(miette!("None"))
}

pub(crate) fn visit_expression(
    visitor: &dyn ExpressionVisitor, controller: &PatchController,
    expr: &ExpressionAST, arguments: &[String]
) -> Option<String> {
    let mut result = String::new();

    match expr {
        ExpressionAST::Action(actions) => {
            for act in actions {
                result += &*visit_expression(visitor, controller, act, arguments)?;
            }
        }
        ExpressionAST::ActionCall { name, arguments: args, .. } => {
            result += &*visitor.visit_action_call(controller, name, args)?;
        }
        ExpressionAST::LocalCall { name: var, .. } => {
            result += &*visitor.visit_local_call(controller, var)?;
        }
        ExpressionAST::ArgumentCall { name: var, .. } => {
            result += &*visitor.visit_argument_call(controller, var, arguments)?;
        }
        ExpressionAST::Text(text) => {
            result += &*visitor.visit_text(controller, text)?;
        }
    }

    Some(result)
}

