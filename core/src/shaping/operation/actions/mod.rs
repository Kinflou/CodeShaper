// Relative Modules
pub mod builder;
pub mod replacer;
pub mod resolver;
pub mod maker;
pub mod composer;
pub mod error;

// Standard Uses

// Crate Uses
use crate::shaping::operation::actions::builder::BuilderAction;
use crate::shaping::operation::actions::maker::MakerAction;
use crate::shaping::operation::actions::replacer::ReplacerAction;
use crate::shaping::operation::actions::resolver::ResolverAction;
use crate::shaping::patch::controller::PatchController;
use crate::utils::dual_key_map::DualKeyMap;

// External Uses
use indextree::{Node, NodeId};
use enum_dispatch::enum_dispatch;
use eyre::{bail, Result};


#[allow(unused)]
#[enum_dispatch]
pub trait Action {
    fn name(&self) -> &str;
    fn patch_controller(&self) -> &PatchController;

    fn parent_id(&self) -> &Option<NodeId>;
    fn parent(&self) -> Option<&Node<ActionOf>> {
        if let Some(parent_id) = self.parent_id() {
            return self.patch_controller().actions.get(*parent_id)
        }

        None
    }

    fn root(&self) -> Option<&ActionOf> {
        let Some(parent) = self.parent() else {
            return None
        };

        let mut root = parent.get();
        while let Some(parent) = root.parent() {
            root = parent.get()
        }

        Some(root)
    }

    fn children(&self) -> &Vec<&ActionOf> {
        todo!()
    }
    fn children_mut(&self) -> &Vec<&mut ActionOf> {
        todo!()
    }

    // fn expression(&self) -> String;

    fn args_count(&self) -> usize { 0 }

    fn locals(&self) -> &DualKeyMap<usize, String, String>;

    fn process(&mut self) -> Option<String>;

    fn find_inner(&self, name: &str) -> Option<&ActionOf> {
        let actions = self.children();

        for action in self.children() {
            if action.name() == name {
                return Some(action)
            }
        }

        for action in self.children() {
            if let Some(act) = action.find_inner(name) {
                return Some(act)
            }
        }

        None
    }

    fn find_inner_mut(&self, name: &str) -> Option<&mut ActionOf> {
        /*
        let actions = self.children();

        for action in self.children_mut() {
            if action.name() == name {
                return Some(action)
            }
        }

        for action in self.children_mut() {
            if let Some(act) = action.find_inner_mut(name) {
                return Some(act)
            }
        }

        None
        */

        todo!()
    }
}

#[enum_dispatch(Action)]
pub enum ActionOf {
    BuilderAction(BuilderAction),
    ReplacerAction(ReplacerAction),
    MakerAction(MakerAction),
    ResolverAction(ResolverAction),
}

impl ActionOf {
    pub fn maker(&self) -> Result<&MakerAction> {
        match self {
            Self::MakerAction(m) => Ok(m),
            _ => bail!("Bad")
        }
    }
    pub fn maker_mut(&mut self) -> Result<&mut MakerAction> {
        match self {
            Self::MakerAction(m) => Ok(m),
            _ => bail!("Bad")
        }
    }
}
