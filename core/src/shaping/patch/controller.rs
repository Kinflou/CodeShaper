// Standard Uses
use std::rc::Rc;

// Crate Uses
use crate::shaping::patch::Patch;
use crate::shaping::operation::actions::{Action, ActionOf};
use crate::shaping::operation::actions::builder::BuilderAction;
use crate::ast::listener::ListenerController;

// External Uses
use indextree::Arena;


#[allow(unused)]
#[derive(Default)]
pub struct PatchController {
    pub patch: Rc<Patch>,
    pub actions: Arena<ActionOf>,
    pub listener_controller: ListenerController,
}

#[allow(unused)]
impl PatchController {
    pub fn with(patch: Patch) -> Self {
        let patch = Rc::new(patch);
        let actions = Arena::<ActionOf>::new();
        // let composer = composer::Composer::with(hello_maker);

        Self {
            patch, actions,
            listener_controller: Default::default(),
        }
    }

    pub fn with_shared(patch: Rc<Patch>) -> Self {
        let actions = Self::make_actions(&patch);
        // let composer = composer::Composer::with(hello_maker);

        Self {
            patch, actions,
            listener_controller: Default::default(),
        }
    }

    fn make_actions(patch: &Rc<Patch>) -> Arena::<ActionOf> {
        let actions = Arena::<ActionOf>::new();

        for builder in patch.actions.builders.as_ref().unwrap() {
            let action = ActionOf::from(BuilderAction::with_ref(builder));
        }

        actions
    }
    fn make_inner_action(arena: Arena::<ActionOf>) {

    }

    pub fn visit(&mut self) {
        self.actions.iter_mut().map(|action|
            action.get_mut().process()
        );
    }

    pub fn find_action(&self, name: &str) -> Option<&ActionOf> {
        for action in self.actions.iter() {
            let action = action.get();

            if action.name() == name { return Some(action) }

            if let Some(name) = action.find_inner(name) {
                return Some(name)
            }
        }

        None
    }

    pub fn find_action_mut(&mut self, name: &str) -> Option<&mut ActionOf> {
        for action in self.actions.iter_mut() {
            let action = action.get_mut();

            if action.name() == name { return Some(action) }
        }

        None
    }

    pub fn possible_rules(&self) -> Vec<&'static str> {
        vec!["*"]
    }
}
