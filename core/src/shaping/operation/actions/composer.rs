// Standard Uses
use std::cell::RefCell;

// Crate Uses
use crate::shaping::operation::actions::{Action, ActionOf};
use crate::shaping::patch::controller::PatchController;
use crate::shaping::patch::trace::Sample;
use crate::expressions::visitor;
use crate::expressions::visitor::ExpressionVisitor;
use crate::expressions::syntax::ExpressionAST;

// External Uses
use regex::{Captures, Regex};
use indextree::NodeId;


#[derive(Debug, Default, Clone, PartialEq)]
pub struct Locals {
    values: Vec<String>
}

impl Locals {
    pub fn get(&self, number: usize) -> Option<&String> {
        self.values.get(number)
    }
}

impl From<&Captures<'_>> for Locals {
    fn from(captures: &Captures) -> Self {
        let values = captures.iter().skip(1)
            .map(|group| group.map(|group| group.as_str().to_owned()).unwrap_or_default())
            .collect();

        Self { values }
    }
}

#[allow(unused)]
pub struct Composer {
    action_id: NodeId,
    // patch_controller: Rc<RefCell<PatchController>>,
    pub(crate) locals: RefCell<Locals>,
    built: RefCell<String>,
}

#[allow(unused)]
impl Composer {
    pub fn with(action_id: NodeId) -> Self {
        Self {
            action_id,
            locals: Default::default(),
            built: RefCell::new(String::new())
        }
    }
}

#[allow(unused)]
impl ExpressionVisitor for Composer {
    fn visit_action_call(
        &self, controller: &PatchController, name: &str, arguments: &[ExpressionAST]
    ) -> Option<String> {
        let mut args = vec![];
        for arg in arguments {
            args.push(
                visitor::visit_expression(self, controller,
                arg, &[]
                )?
            );
        }

        let Some(action) = controller.find_action(name, controller) else {
            return None
        };

        let result = match action {
            ActionOf::BuilderAction(builder) => builder.process(controller),
            ActionOf::ReplacerAction(_) => None,
            ActionOf::MakerAction(maker) => maker.make_with_args(&args, controller),
            ActionOf::SwitchAction(switch) => switch.resolve_with_args(controller, &args),
            ActionOf::ListAction(list) => list.index_with_args(controller, &args)
        };

        if let (Some(output), true) = (&result, controller.tracing()) {
            let call = Sample { groups: args.into_iter().map(Some).collect(), output: output.clone(), ..Sample::default() };
            controller.record(*action.self_id(), |trace, limit| trace.call(limit, call));
        }

        result
    }

    fn visit_text(&self, controller: &PatchController, text: &str) -> Option<String> {
        Some(text.to_owned())
    }

    fn visit_local_call(
        &self, controller: &PatchController, variable: &str
    ) -> Option<String> {
        let number = variable.parse::<usize>().ok()?;

        self.locals.borrow().get(number).cloned()
    }

    fn visit_argument_call(
        &self, controller: &PatchController, variable: &str, arguments: &[String]
    ) -> Option<String> {
        let index = variable.parse::<usize>().ok()?;

        arguments.get(index).map(|a| a.to_string())
    }
}

pub fn capture_locals(reg_expr: Option<&str>, content: &str) -> Locals {
    // TODO: Add named captures
    // locals.push((name.to_string(), captures.name(name).unwrap().as_str().to_string()))

    reg_expr
        .and_then(|reg_expr| Regex::new(reg_expr).ok())
        .and_then(|regex| regex.captures(content).map(|captures| Locals::from(&captures)))
        .unwrap_or_default()
}

/*
// TODO: Better naming for expression argument, since it can be either a simple Regex
//       or also contain Action expressions, being a mix (not that it probably matters)
//       also perhaps the expr type can be &str instead, check that
#[allow(unused)]
pub fn compose_regex(build: &str, locals: &LocalsMap) -> Option<String> {
    let mut built = String::new();

    regex_automata::util::interpolate::string(
        build,
        |index, dst| {
            if let Some(value) = locals.get_by_left(index - 1) {
                dst.push_str(&format!("\"{}\"", value));
            }
        },
        |name| {
            if let Some(idx) = locals.key_map.get_by_right(&Some(name.to_owned())) {
                return Some(*idx)
            }

            None
        },
        &mut built,
    );

    Some(built)
}
*/

