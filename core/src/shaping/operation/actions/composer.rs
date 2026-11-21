// Standard Uses
use std::cell::RefCell;

// Crate Uses
use crate::shaping::operation::actions::{Action, ActionOf, LocalsMap};
use crate::shaping::patch::controller::PatchController;
use crate::expressions::visitor;
use crate::expressions::visitor::ExpressionVisitor;
use crate::expressions::syntax::ExpressionAST;
use crate::utils::dual_key_map::DualKeyMap;

// External Uses
use regex::Regex;
use indextree::NodeId;


#[allow(unused)]
pub struct Composer {
    action_id: NodeId,
    // patch_controller: Rc<RefCell<PatchController>>,
    pub(crate) locals: RefCell<LocalsMap>,
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

        match action {
            ActionOf::BuilderAction(builder) => {
                return builder.process(controller);
            }
            ActionOf::ReplacerAction(replacer) => {

            }
            ActionOf::MakerAction(maker) => {
                return maker.make_with_args(&args, controller)
            }
            ActionOf::SwitchAction(switch) => {

            }
            ActionOf::ListAction(list) => {}
        }

        None
    }

    fn visit_text(&self, controller: &PatchController, text: &str) -> Option<String> {
        Some(text.to_owned())
    }

    fn visit_local_call(
        &self, controller: &PatchController, variable: &str
    ) -> Option<String> {
        let locals = self.locals.borrow();

        let action: &ActionOf = controller.actions.get(self.action_id).unwrap().get();

        if let Ok(number) = variable.parse::<usize>() {
            let Some(local) = locals.get_by_left(number) else {
                panic!("Local not found")
            };

            return Some(local.clone())
        }

        if let Some(local) = self.locals.borrow().get_by_right(variable.to_owned()) {
            return None
        }

        panic!("Local '{}' not found", variable);
    }

    fn visit_argument_call(
        &self, controller: &PatchController, variable: &str, arguments: &[String]
    ) -> Option<String> {
        let index = variable.parse::<usize>().ok()?;

        arguments.get(index).map(|a| a.to_string())
    }
}

pub fn capture_locals(reg_expr: Option<&str>, content: &str) -> LocalsMap {
    let Some(reg_expr) = reg_expr else { return LocalsMap::default() };

    let re = Regex::new(reg_expr).unwrap();
    let Some(captures) = re.captures(content) else {
        return DualKeyMap::default()
    };

    let mut locals = LocalsMap::default();
    for (idx, mat) in captures.iter().skip(1).enumerate() {
        if let Some(mat) = mat {
            locals.push_with_left(idx, mat.as_str().to_owned())
        }
    };

    // TODO: Add named captures
    // locals.push((name.to_string(), captures.name(name).unwrap().as_str().to_string()))

    locals
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

