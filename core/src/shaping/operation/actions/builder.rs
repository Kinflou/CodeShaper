// Standard Uses
use std::rc::Rc;

// Crate Uses
use crate::shaping::patch;
use crate::shaping::patch::controller::PatchController;
use crate::shaping::operation::actions::{Action, ActionOf, composer};
use crate::shaping::operation::actions::composer::Composer;
use crate::shaping::operation::actions::error::Error;
use crate::expressions::visitor;
use crate::expressions::visitor::ExpressionVisitor;
use crate::utils::dual_key_map::DualKeyMap;

// External Uses
use eyre::Result;
use indextree::NodeId;
use regex::Regex;


pub struct BuilderAction {
    pub builder: patch::builder::Builder,
    parent_id : Option<NodeId>,
    composer: Option<Composer>,
    input: Option<String>,
    locals: DualKeyMap<usize, String, String>,
    // built: Option<String>
}

#[allow(unused)]
impl BuilderAction {
    pub fn with(builder: patch::builder::Builder) -> Self {
        Self {
            builder,
            parent_id: None,
            composer: None,
            locals: DualKeyMap::new(),
            input: None
        }

        // Some(Composer::new())
    }

    pub fn with_ref(builder: &patch::builder::Builder) -> Self {
        todo!()
    }
}

#[allow(unused)]
impl Action for BuilderAction {
    fn name(&self) -> &str { &self.builder.name }

    fn patch_controller(&self) -> &PatchController { todo!() }

    fn parent_id(&self) -> &Option<NodeId> { &self.parent_id }

    fn children(&self) -> &Vec<&ActionOf> {
        todo!() // &self.children
    }
    fn children_mut(&self) -> &Vec<&mut ActionOf> {
        todo!() // &self.children
    }

    // fn expression(&self) -> String { self.builder.build.clone() }

    fn locals(&self) -> &DualKeyMap<usize, String, String> { &self.locals }

    fn process(&mut self) -> Option<String> {
        /*
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
        */
        todo!()
    }
}


#[allow(unused)]
impl BuilderAction {
    pub fn process_input(&mut self, input: String) -> Result<Option<String>> {
        self.locals = composer::capture_locals(&self.builder.r#match, &input);
        self.input = Some(input);

        let result = composer::compose_regex(&self.builder.build, &self.locals);

        let res = visitor::navigate_expression(
            self.composer.as_mut().unwrap(), &*self.builder.build
        )?;

        Ok(result)
    }

    pub fn process_expression(&mut self) -> Option<String> {
        let Some(content) = &self.input else {
            panic!("TODO: Umh maybe do something here")
        };

        let locals = composer::capture_locals(
            &self.builder.r#match, &content
        );

        // self.built = Some("".to_string());
        // self.built.clone()
        Some("".to_owned())
    }

    pub fn process_expr(&self, visitor: Rc<dyn ExpressionVisitor>) -> Vec<Error> {
        let mut errors = vec![];

        /*
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
        */

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
