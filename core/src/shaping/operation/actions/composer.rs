// Standard Uses
use std::rc::Rc;
use std::cell::RefCell;
use std::option::Option;

// Crate Uses
use crate::shaping::operation::actions::{Action, ActionOf};
use crate::shaping::patch::controller::PatchController;
use crate::expressions::visitor::ExpressionVisitor;
use crate::expressions::syntax::ExpressionAST;
use crate::utils::dual_key_map::DualKeyMap;

// External Uses
use regex::Regex;
use indextree::NodeId;


#[allow(unused)]
pub struct Composer {
    action_id: NodeId,
    patch_controller: Rc<RefCell<PatchController>>,
    built: String
}

#[allow(unused)]
impl Composer {
    pub fn with(action_id: NodeId, patch_controller: Rc<RefCell<PatchController>>) -> Self {
        Self {
            action_id,
            patch_controller,
            built: String::new()
        }
    }
}

#[allow(unused)]
impl ExpressionVisitor for Composer {
    fn visit_expression(&mut self, name: &str, args: &Vec<ExpressionAST>) {
        let controller_mut = self.patch_controller.borrow_mut();

        let Some(action) = controller_mut.find_action(name) else {
            return;
        };
    }

    fn visit_arguments(&mut self, arguments: &Vec<ExpressionAST>) {}

    fn visit_text(&mut self, text: &str) {
        self.built += text;
    }

    fn visit_local_call(&mut self, variable: &str) {
        let mut controller_mut = self.patch_controller.borrow_mut();
        let action: &ActionOf = controller_mut.actions.get(self.action_id).unwrap().get();

        if let Ok(number) = variable.parse::<usize>() {
            if let Some(local) = action.locals().get_by_left(number) {
                todo!()
            }
        } else {
            if let Some(local) = action.locals().get_by_right(variable.to_owned()) {
                todo!()

            }
        }

        panic!("Umh should return a error")
    }

    fn visit_argument_call(&mut self, variable: &str) {
        todo!()
    }

    fn built(&self) -> String {
        self.built.clone()
    }
}

pub fn capture_locals(reg_expr: &str, content: &str) -> DualKeyMap<usize, String, String> {
    let re = Regex::new(reg_expr).unwrap();
    let Some(captures) = re.captures(content) else {
        return DualKeyMap::new()
    };

    let mut locals: DualKeyMap<usize, String, String> = DualKeyMap::new();
    for (idx, mat) in captures.iter().skip(1).enumerate() {
        if let Some(mat) = mat {
            locals.push_with_left(idx, mat.as_str().to_owned())
        }

    };

    // TODO: Add named captures
    // locals.push((name.to_string(), captures.name(name).unwrap().as_str().to_string()))

    locals
}

// TODO: Better naming for expression argument, since it can be either a simple Regex
//       or also contain Action expressions, being a mix (not that it probably matters)
//       also perhaps the expr type can be &str instead, check that
#[allow(unused)]
pub fn compose_regex(
    build: &str, locals: &DualKeyMap<usize, String, String>
) -> Option<String> {
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

