// Standard Uses
use std::rc::Rc;

// Crate Uses
use crate::ast::listener::ListenerController;
use crate::shaping::patch::{Actions, Patch};
use crate::shaping::operation::actions::{Action, ActionOf};
use crate::shaping::operation::actions::builder::BuilderAction;
use crate::shaping::operation::actions::maker::MakerAction;
use crate::shaping::operation::actions::indexer::{ListAction, SwitchAction};

// External Uses
use indextree::{Arena, NodeId};


#[allow(unused)]
pub struct PatchController {
    pub patch: Rc<Patch>,
    pub actions: Arena<ActionOf>,
    pub listener_controller: ListenerController,
}

#[allow(unused)]
impl PatchController {
    pub fn with(patch: Rc<Patch>) -> Self {
        let actions = Self::make_actions(patch.clone());

        Self { patch, actions, listener_controller: Default::default() }
    }

    pub fn with_listener(patch: Rc<Patch>, listener_controller: ListenerController) -> Self {
        let actions = Self::make_actions(patch.clone());

        Self { patch, actions, listener_controller }
    }

    fn make_actions(patch: Rc<Patch>) -> Arena<ActionOf> {
        let mut arena = Arena::<ActionOf>::new();
        Self::make_inner_actions(&mut arena, &patch.actions, None);
        arena
    }

    fn make_inner_actions(
        arena: &mut Arena<ActionOf>, actions: &Actions, parent: Option<&NodeId>
    ) {
        if let Some(builders) = &actions.builders {
            for builder in builders.iter() {
                let action = ActionOf::BuilderAction(
                    BuilderAction::with_ref(builder.clone(), parent.cloned())
                );
                let id = arena.new_node(action);
                arena.get_mut(id).unwrap().get_mut().builder_mut().unwrap().post_new(id);

                if let Some(parent) = parent { parent.append(id, arena); }

                Self::make_inner_actions(arena, &builder.actions, Some(&id));
            }
        }

        if let Some(makers) = &actions.makers {
            for maker in makers.iter() {
                let action = ActionOf::MakerAction(
                    MakerAction::with_ref(maker.clone(), parent.cloned())
                );
                let id = arena.new_node(action);
                arena.get_mut(id).unwrap().get_mut().maker_mut().unwrap().post_new(id);

                if let Some(parent) = parent { parent.append(id, arena); }
            }
        }

        if let Some(lists) = &actions.lists {
            for list in lists.iter() {
                let action = ActionOf::ListAction(
                    ListAction::with_ref(list.clone(), parent.cloned())
                );
                let id = arena.new_node(action);
                arena.get_mut(id).unwrap().get_mut().list_mut().unwrap().post_new(id);

                if let Some(parent) = parent { parent.append(id, arena); }
            }
        }

        if let Some(switchers) = &actions.switchers {
            for switch in switchers.iter() {
                let action = ActionOf::SwitchAction(
                    SwitchAction::with_ref(switch.clone(), parent.cloned())
                );
                let id = arena.new_node(action);
                arena.get_mut(id).unwrap().get_mut().switch_mut().unwrap().post_new(id);

                if let Some(parent) = parent { parent.append(id, arena); }
            }
        }
    }

    pub fn visit(&mut self) {
        for action in self.actions.iter() {
            let action = action.get();

            match action {
                ActionOf::BuilderAction(b) => {
                    let result = b.process(&self);
                }
                ActionOf::ReplacerAction(r) => {
                    let result = r.process(&self);
                }
                _ => {}
            };
        }
    }

    pub fn find_action<'t>(
        &'t self, name: &str, controller: &'t PatchController
    ) -> Option<&ActionOf> {
        for action in self.actions.iter() {
            let action: &ActionOf = action.get();

            if action.name() == name { return Some(action) }

            if let Some(name) = action.find_inner(name, controller) {
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
