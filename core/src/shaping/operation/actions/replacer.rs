// Standard Uses

// Crate Uses
use crate::shaping::patch::replacer::Replacer;
use crate::shaping::patch::controller::PatchController;
use crate::shaping::operation::actions::Action;
use crate::shaping::operation::actions::composer::{Composer, Locals};
use crate::shaping::patch::trace::Sample;
use crate::expressions::visitor;

// External Uses
use indextree::NodeId;


pub struct ReplacerAction {
    replacer: Replacer,
    self_id: Option<NodeId>,
    parent_id: Option<NodeId>,
    composer: Option<Composer>,
}

impl ReplacerAction {
    pub fn with_ref(replacer: Replacer, parent_id: Option<NodeId>) -> Self {
        Self { replacer, self_id: None, parent_id, composer: None }
    }

    pub fn post_new(&mut self, self_id: NodeId) {
        self.self_id = Some(self_id);
        self.composer = Some(Composer::with(self_id));
    }

    pub fn replacer(&self) -> &Replacer { &self.replacer }

    pub fn replace_input(&self, controller: &PatchController, input: &str) -> Option<String> {
        let from = self.replacer.from_regex().ok()?;
        let composer = self.composer.as_ref()?;

        let mut result = String::with_capacity(input.len());
        let mut last = 0;
        let mut matches = 0;

        for captures in from.captures_iter(input) {
            let found = captures.get(0)?;

            *composer.locals.borrow_mut() = Locals::from(&captures);

            let replacement = visitor::navigate_expression(
                composer, controller, &self.replacer.to, &[]
            ).ok()?;

            matches += 1;
            if controller.tracing() {
                let sample = Sample {
                    input: found.as_str().to_string(),
                    groups: captures.iter().skip(1).map(|group| group.map(|group| group.as_str().to_string())).collect(),
                    output: replacement.clone(),
                    ..Sample::default()
                };
                controller.record(*self.self_id(), |trace, limit| trace.sample(limit, sample));
            }

            result.push_str(&input[last..found.start()]);
            result.push_str(&replacement);
            last = found.end();
        }

        result.push_str(&input[last..]);
        controller.record(*self.self_id(), |trace, _| trace.input(matches));

        Some(result)
    }
}

impl Action for ReplacerAction {
    fn name(&self) -> &str { "" }

    fn self_id(&self) -> &NodeId { self.self_id.as_ref().unwrap() }

    fn parent_id(&self) -> &Option<NodeId> { &self.parent_id }

    fn process(&self, patch_controller: &PatchController) -> Option<String> {
        let input = patch_controller.listener_controller.location_content
            .get(self.replacer.target_location())?;

        self.replace_input(patch_controller, input)
    }
}
