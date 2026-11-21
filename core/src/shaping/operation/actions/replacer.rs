// Standard Uses

// Crate Uses
use crate::shaping::operation::actions::Action;

// External Uses
use indextree::NodeId;
use crate::shaping::patch::controller::PatchController;


pub struct ReplacerAction {}

impl Action for ReplacerAction {
    fn name(&self) -> &str {
        todo!()
    }

    fn self_id(&self) -> &NodeId { todo!() }

    fn parent_id(&self) -> &Option<NodeId> {
        todo!()
    }

    // fn locals(&self) -> &LocalsMap { todo!() }

    #[allow(unused)]
    fn process(&self, patch_controller: &PatchController) -> Option<String> {
        todo!()
    }
}
