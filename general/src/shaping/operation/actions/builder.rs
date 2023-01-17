// Standard Uses
use std::cell::RefCell;
use std::rc::{Rc, Weak};

// Crate Uses
use crate::ast;
use crate::shaping::patch;
use crate::shaping::operation::expressions::visitor;
use crate::shaping::operation::actions::analyzer::Error;
use crate::shaping::operation::actions::{Action, composer};
use crate::shaping::operation::actions::composer::Composer;
use crate::shaping::operation::expressions::visitor::ExpressionVisitor;

// External Uses
use anyhow::Result;
use regex::Regex;


pub struct Builder<'a> {
    pub builder: patch::builder::Builder,
    pub patch_controller: Option<Rc<patch::controller::Controller>>,
    pub listener_controller: Option<Rc<ast::listener::Controller>>,
    pub composer: Option<Composer<'a>>,

    pub parent: Option<Rc<dyn Action>>,
    pub children: Vec<Rc<RefCell<dyn Action>>>,

    pub input: Option<String>,
    pub locals: Vec<(String, String)>,
    pub built: Option<String>
}

#[allow(unused)]
impl<'a> Builder<'a> {
    pub fn with_shared(patch_builder: patch::builder::Builder) -> Self {
        Self {
            builder: patch_builder,
            patch_controller: None,
            listener_controller: None,
            composer: None,

            parent: None,
            children: vec![],

            input: None,
            locals: vec![],
            built: None
        }
    }

    /*
    pub fn mut_shared_with_shared(patch_builder: patch::builder::Builder) -> Rc<RefCell<Builder>> {
        let mut this = Rc::new(RefCell::new(Self {
            builder: patch_builder,
            patch_controller: Some(Rc::new(Default::default())),
            listener_controller: Some(Rc::new(Default::default())),
            composer: None,
            actions: vec![],

            input: None,
            locals: vec![],
            built: None,
        }));

        let b = RefCell::borrow_mut(&this);

        b.composer = Some(
            Rc::new(RefCell::new(Composer::with_action_shared(this)))
        );

        this

        todo!()
    }
    */

}

#[allow(unused)]
impl<'a> Action for Builder<'a> {
    fn name(&self) -> &str { &self.builder.name }
    fn parent(&self) -> Option<Weak<dyn Action>> {
        Some(Rc::downgrade(&self.parent.clone().unwrap()))
    }
    fn children(&self) -> &Vec<Rc<RefCell<dyn Action>>> { &self.children }
    fn patch_controller(&self) -> Option<Weak<patch::controller::Controller>> {
        Some(Rc::downgrade(&self.patch_controller.as_ref().unwrap()))
    }
    fn listener_controller(&self) -> Option<Weak<ast::listener::Controller>> {
        Some(Rc::downgrade(&self.listener_controller.as_ref().unwrap()))
    }
    fn expression(&self) -> String { self.builder.build.clone() }

    fn process(&mut self) -> Option<String> {
        if self.built.is_some() {
            return self.built.clone()
        }

        let Some(listener) = &self.listener_controller else {
            return None
        };

        let (Some(location), Some(context)) = (
            listener.locations.last(), listener.contents.last()
        ) else {
            return None
        };

        if self.builder.reference_location.is_some() {
            if self.builder.reference_location.as_ref().unwrap().to_lowercase() == location.to_lowercase() {
                return None
            }
        }
        else if self.builder.location.to_lowercase() != location.to_lowercase() {
            return None
        }

        self.process_expression()
    }
}


#[allow(unused)]
impl<'a> Builder<'a> {
    pub fn process_input(&mut self, input: String) -> Result<Option<String>> {
        self.locals = composer::capture_locals(
            &self.builder.r#match, input.clone()
        );

        self.input = Some(input);

        let result = visitor::navigate_expression(
            self.composer.as_mut().unwrap(), &*self.builder.r#match.clone()
        );

        composer::compose_regex(
            self.builder.r#match.clone(), self.input.clone().unwrap(),
            &self.locals, &self.builder.build
        );

        // TODO: Piece the expression nodes together and set self.built
        //       in case if matching occurs

        Ok(self.built.clone())
    }

    pub fn process_expression(&mut self) -> Option<String> {
        let content = &self.input;

        let locals = composer::capture_locals(
            &self.builder.r#match,content.clone().unwrap()
        );

        self.built = Some("".to_string());

        self.built.clone()
    }

    pub fn process_expr(&self, visitor: Rc<dyn ExpressionVisitor>) -> Vec<Error> {
        let mut errors = vec![];

        let possible_rules = self.patch_controller.as_ref().unwrap().possible_rules();

        if !self.builder.location.is_empty() {
            if !possible_rules.contains(&&*self.builder.location) {
                errors.push(Error::LocationNotFound);
            }
        } else if self.builder.reference_location.is_some() {
            if !possible_rules.contains(&&*self.builder.reference_location.clone().unwrap()) {
                errors.push(Error::LocationNotFound);
            }
        } else {
            return errors
        }

        errors
    }

    fn matches(&self, content: &str) {
        let re = Regex::new(&self.builder.r#match).unwrap();
        let captures = Regex::captures(&re, content).unwrap();

        let mut locals = vec![];
        for name in re.capture_names().flatten() {
            locals.push((name, captures.name(name).unwrap().as_str().clone()))
        };
    }
}
