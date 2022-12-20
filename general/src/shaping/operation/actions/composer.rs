// Standard Uses
use std::rc::Rc;
use std::option::Option;
use std::cell::RefCell;

// Crate Uses
use crate::shaping::operation::actions::Action;
use crate::shaping::operation::expressions::common::Expression;
use crate::shaping::operation::expressions::visitor::ExpressionVisitor;

// External Uses
use regex::Regex;


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
            &mut capture_locals(text, self.input.clone().unwrap())
        );
    }
}

pub fn capture_locals(reg_expr: &str, content: String) -> Vec<(String, String)> {
    let re = Regex::new(reg_expr).unwrap();
    let Some(captures) = Regex::captures(&re, &*content) else {
        return vec![]
    };

    let mut locals = vec![];
    for name in re.capture_names().flatten() {
        locals.push((name.to_string(), captures.name(name).unwrap().as_str().to_string()))
    };

    locals
}

// TODO: Better naming for expression argument, since it can be either a simple Regex
//       or also contain Action expressions, being a mix (not that it probably matters)
//       also perhaps the expr type can be &str instead, check that
#[allow(unused)]
pub fn compose_regex(expr: String, input: String, locals: &Vec<(String, String)>, build: &String) -> Option<String> {
    let re = Regex::new(&*expr).unwrap();

    // TODO: Check how to substitute with locals that are custom (not captured by the above Re)

    let built = re.replace(&*input, build);

    Some(built.to_string())
}
