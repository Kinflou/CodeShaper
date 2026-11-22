// Relative Modules
pub mod builder;
pub mod replacer;
pub mod indexer;
pub mod maker;
pub mod composer;

// Standard Uses

// Crate Uses
use crate::shaping::patch::controller::PatchController;
use crate::shaping::operation::actions::builder::BuilderAction;
use crate::shaping::operation::actions::maker::MakerAction;
use crate::shaping::operation::actions::replacer::ReplacerAction;
use crate::shaping::operation::actions::indexer::{ListAction, SwitchAction};
use crate::utils::dual_key_map::DualKeyMap;

// External Uses
use indextree::{Children, Node, NodeId};
use enum_dispatch::enum_dispatch;
use miette::{bail, Result};


pub(crate) type LocalsMap = DualKeyMap<usize, String, String>;

#[allow(unused)]
#[enum_dispatch]
pub trait Action {
    fn name(&self) -> &str;
    fn self_id(&self) -> &NodeId;

    fn parent_id(&self) -> &Option<NodeId>;
    fn parent<'t>(&'t self, controller: &'t PatchController) -> Option<&Node<ActionOf>> {
        if let Some(parent_id) = self.parent_id() {
            return controller.actions.get(*parent_id)
        }

        None
    }

    fn root_id(&self, controller: &PatchController) -> Option<NodeId> {
        let mut root = Some(self.self_id().clone());

        while let Some(node) = controller.actions.get(root.unwrap()) {
            if node.parent().is_none() { return root }
            root = node.parent();
        }

        root
    }
    /*
    fn root<'t>(&'t self, controller: &'t PatchController) -> Option<&ActionOf> {
        let Some(parent) = self.parent(controller) else {
            return None
        };

        let mut root = parent.get();
        while let Some(parent) = root.parent(controller) {
            root = parent.get()
        }

        Some(root)
    }
    */

    fn children<'t>(&'t self, controller: &'t PatchController) -> Children<'_, ActionOf> {
        self.self_id().children(&controller.actions)
    }

    // fn expression(&self) -> String;

    fn args_count(&self) -> usize { 0 }

    // fn locals(&self) -> &LocalsMap;

    fn process(&self, patch_controller: &PatchController) -> Option<String>;

    fn find_inner<'t>(&'t self, name: &str, controller: &'t PatchController) -> Option<&ActionOf> {
        let actions = self.children(controller);

        for id in actions.into_iter() {
            let action = controller.actions.get(id).unwrap().get();

            if action.name() == name { return Some(action) }
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

    fn find_inner_id(&self, name: &str, controller: &PatchController) -> Option<NodeId> {
        let actions = self.children(controller);

        for id in self.children(controller) {
            let action = controller.actions.get(id).unwrap().get();
            if action.name() == name {
                return Some(id)
            }
        }

        None
    }

}

#[enum_dispatch(Action)]
pub enum ActionOf {
    BuilderAction(BuilderAction),
    ReplacerAction(ReplacerAction),
    MakerAction(MakerAction),
    SwitchAction(SwitchAction),
    ListAction(ListAction),
}

impl ActionOf {
    pub fn maker(&self) -> Result<&MakerAction> {
        match self {
            Self::MakerAction(m) => Ok(m),
            _ => bail!("Not a maker")
        }
    }
    pub fn maker_mut(&mut self) -> Result<&mut MakerAction> {
        match self {
            Self::MakerAction(m) => Ok(m),
            _ => bail!("Not a maker")
        }
    }

    pub fn builder(&self) -> Result<&BuilderAction> {
        match self {
            Self::BuilderAction(b) => Ok(b),
            _ => bail!("Not a builder")
        }
    }

    pub fn builder_mut(&mut self) -> Result<&mut BuilderAction> {
        match self {
            Self::BuilderAction(b) => Ok(b),
            _ => bail!("Not a builder")
        }
    }

    pub fn replacer(&self) -> Result<&ReplacerAction> {
        match self {
            Self::ReplacerAction(r) => Ok(r),
            _ => bail!("Not a replacer")
        }
    }

    pub fn replacer_mut(&mut self) -> Result<&mut ReplacerAction> {
        match self {
            Self::ReplacerAction(r) => Ok(r),
            _ => bail!("Not a replacer")
        }
    }

    pub fn list(&self) -> Result<&ListAction> {
        match self {
            Self::ListAction(l) => Ok(l),
            _ => bail!("Not a list")
        }
    }

    pub fn list_mut(&mut self) -> Result<&mut ListAction> {
        match self {
            Self::ListAction(l) => Ok(l),
            _ => bail!("Not a list")
        }
    }

    pub fn switch(&self) -> Result<&SwitchAction> {
        match self {
            Self::SwitchAction(s) => Ok(s),
            _ => bail!("Not a switch")
        }
    }

    pub fn switch_mut(&mut self) -> Result<&mut SwitchAction> {
        match self {
            Self::SwitchAction(s) => Ok(s),
            _ => bail!("Not a switch")
        }
    }
}
