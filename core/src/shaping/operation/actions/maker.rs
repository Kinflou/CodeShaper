// Standard Uses

// Crate Uses
use crate::expressions::visitor;
use crate::shaping::operation::actions::composer::Composer;
use crate::shaping::operation::actions::{Action, composer};
use crate::shaping::patch::controller::PatchController;
use crate::shaping::patch::maker::Maker;
use crate::shaping::patch::pattern::regex_builder;
use crate::shaping::patch::trace::{groups, Found, Sample};

// External Uses
use indextree::NodeId;


pub struct MakerAction {
    maker: Maker,
    self_id: Option<NodeId>,
    parent_id: Option<NodeId>,
    composer: Option<Composer>
}

#[allow(unused)]
impl MakerAction {
    pub fn with_ref(
        maker: Maker, parent_id: Option<NodeId>
    ) -> Self {
        Self {
            maker, self_id: None, parent_id,
            composer: None
        }
    }

    pub fn post_new(&mut self, self_id: NodeId) {
        self.self_id = Some(self_id);
        self.composer = Some(Composer::with(self_id));
    }

    pub fn make(&self, input: &str, controller: &PatchController) -> Option<String> {
        let locals = composer::capture_locals(
            self.maker.prepare.as_deref(), &self.maker.flags, input
        );

        *self.composer.as_ref().unwrap().locals.borrow_mut() = locals;

        visitor::navigate_expression(
            self.composer.as_ref().unwrap(), controller,
            &self.maker.make, &[]
        ).ok()
    }

    pub fn make_with_args(
        &self, args: &[String], controller: &PatchController
    ) -> Option<String> {
        let composer = self.composer.as_ref()?;
        let prepared = self.maker.prepare.as_deref().zip(args.first());

        *composer.locals.borrow_mut() = prepared
            .map(|(prepare, input)| composer::capture_locals(Some(prepare), &self.maker.flags, input))
            .unwrap_or_default();

        let result = visitor::navigate_expression(composer, controller, &self.maker.make, args).ok();

        if let (Some((prepare, input)), true) = (prepared, controller.tracing()) {
            let regex = regex_builder(prepare, &self.maker.flags).build().ok();
            let captures = regex.as_ref().and_then(|regex| regex.captures(input));
            let sample = Sample {
                groups: captures.as_ref().map(groups).unwrap_or_default(),
                input: input.clone(),
                output: result.clone().unwrap_or_default(),
                matched: captures.is_some(),
                ..Sample::default()
            };

            controller.record(*self.self_id(), |trace, limit| {
                trace.input(usize::from(captures.is_some()));
                trace.sample(limit, sample);
                if let Some(captures) = &captures { trace.found(limit, || Found::new(captures, |_| None)); }
            });
        }

        result
    }
}

impl Action for MakerAction {
    fn name(&self) -> &str { &self.maker.name }

    fn self_id(&self) -> &NodeId { self.self_id.as_ref().unwrap() }

    fn parent_id(&self) -> &Option<NodeId> { &self.parent_id }

    // fn locals(&self) -> &LocalsMap { todo!() }

    #[allow(unused)]
    fn process(&self, patch_controller: &PatchController) -> Option<String> {
        todo!()
    }
}
