// Standard Uses
use std::cell::RefCell;

// Crate Uses
use crate::shaping::patch::builder::Builder;
use crate::shaping::patch::controller::PatchController;
use crate::shaping::operation::actions::{Action, composer};
use crate::shaping::operation::actions::composer::Composer;
use crate::shaping::patch::pattern::regex_builder;
use crate::shaping::patch::trace::{groups, Sample};
use crate::expressions::visitor;

// External Uses
use indextree::NodeId;


#[allow(unused)]
pub struct BuilderAction {
    builder: Builder,
    self_id: Option<NodeId>,
    parent_id : Option<NodeId>,
    composer: Option<Composer>,
    pub current_inner_builder: RefCell<Option<String>>,
    // input: RefCell<Option<String>>,
    // locals: RefCell<LocalsMap>,
}

#[allow(unused)]
impl BuilderAction {
    pub fn with_ref(
        builder: Builder, parent_id: Option<NodeId>
    ) -> Self {
        Self {
            builder, self_id: None, parent_id,
            composer: None,
            current_inner_builder: Default::default(),
        }
    }

    pub fn post_new(&mut self, self_id: NodeId) {
        self.self_id = Some(self_id);
        self.composer = Some(Composer::with(self_id));
    }
}

#[allow(unused)]
impl Action for BuilderAction {
    fn name(&self) -> &str { &self.builder.name }

    fn self_id(&self) -> &NodeId { self.self_id.as_ref().unwrap() }

    fn parent_id(&self) -> &Option<NodeId> { &self.parent_id }

    fn process(&self, patch_controller: &PatchController) -> Option<String> {
        let set_current = |b: &Self| {
            if let Some(root_id) = b.root_id(patch_controller) {
                let Some(root) = patch_controller.actions.get(root_id) else {
                    return None
                };

                let root = root.get().builder().unwrap();

                *root.current_inner_builder.borrow_mut() = Some(root.builder.name.clone());
                return None
            };

            *b.current_inner_builder.borrow_mut() = None;
            None
        };

        /*
        let Some(last) = patch_controller.listener_controller.locations.last() else {
            return set_current(self)
        };

        if self.built.is_some() {
            return self.built.clone()
        }

        let (Some(location), Some(context)) = (
            patch_controller.listener_controller.locations.last(),
            patch_controller.listener_controller.contents.last()
        ) else { return set_current(&self) };
        */

        if self.builder.collect {
            let location = self.builder.reference_location.as_deref()
                .unwrap_or(&self.builder.location);

            let Some(contents) = patch_controller.listener_controller.location_contents
                .get(location) else
            {
                return set_current(self)
            };

            let built: Vec<String> = contents.iter()
                .filter_map(|content| self.process_input(patch_controller, content.clone()))
                .collect();

            return Some(built.join(&self.builder.separator))
        }

        if let Some(ref_location) = &self.builder.reference_location {
            let Some(context) = patch_controller.listener_controller.location_content
                .get(ref_location) else
            {
                return set_current(&self)
            };

            return self.process_input(patch_controller, context.clone())
        }

        let Some(context) = patch_controller.listener_controller.location_content
            .get(&*self.builder.location) else
        {
            return set_current(&self)
        };

        self.process_input(patch_controller, context.clone())
    }
}


#[allow(unused)]
impl BuilderAction {
    pub fn process_input(
        &self, controller: &PatchController, input: String
    ) -> Option<String> {
        let locals = composer::capture_locals(
            Some(&self.builder.r#match), &self.builder.flags, &input
        );

        *self.composer.as_ref().unwrap().locals.borrow_mut() = locals;

        let result = visitor::navigate_expression(
            self.composer.as_ref().unwrap(), controller,
            &self.builder.build, &[]
        ).ok();

        if controller.tracing() {
            let regex = regex_builder(&self.builder.r#match, &self.builder.flags).build().ok();
            let matched = regex.as_ref().is_some_and(|regex| regex.is_match(&input));
            let sample = Sample {
                groups: regex.map(|regex| groups(&regex, &input)).unwrap_or_default(),
                input,
                output: result.clone().unwrap_or_default(),
                matched,
                ..Sample::default()
            };

            controller.record(*self.self_id(), |trace, limit| {
                trace.input(usize::from(matched));
                trace.built += usize::from(result.is_some());
                trace.sample(limit, sample);
            });
        }

        result
    }

    /*
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
    */
}
