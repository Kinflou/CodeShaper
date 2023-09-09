// Standard Uses

// Crate Uses
use crate::shaping::operation::actions::Action;
use crate::shaping::patch::controller::PatchController;
use crate::utils::dual_key_map::DualKeyMap;

// External Uses
use indextree::NodeId;


pub struct ReplacerAction {}

impl Action for ReplacerAction {
    fn name(&self) -> &str {
        todo!()
    }

    fn patch_controller(&self) -> &PatchController {
        todo!()
    }

    fn parent_id(&self) -> &Option<NodeId> {
        todo!()
    }

    fn locals(&self) -> &DualKeyMap<usize, String, String> {
        todo!()
    }

    fn process(&mut self) -> Option<String> {
        todo!()
    }
}
