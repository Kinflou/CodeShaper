// Standard Uses

// Crate Uses
use crate::shaping::patch;
use crate::shaping::operation::actions::{Action, composer};
use crate::shaping::patch::controller::PatchController;
use crate::utils::dual_key_map::DualKeyMap;

// External Uses
use indextree::NodeId;


pub struct MakerAction {
    pub maker: patch::maker::Maker,
    pub parent_id: Option<NodeId>
}


#[allow(unused)]
impl MakerAction {
    pub fn with(maker: patch::maker::Maker, parent_id: Option<NodeId>) -> Self {
        Self { maker, parent_id }
    }

    pub fn make(&self, input: &str) -> Option<String> {
        let locals  = if self.maker.prepare.is_some() {
            composer::capture_locals(
                &*self.maker.prepare.clone().unwrap(), &input
            )
        } else {
            DualKeyMap::new()
        };

        let result = composer::compose_regex(&self.maker.make, &locals);

        result
    }
}

impl Action for MakerAction {
    fn name(&self) -> &str { &self.maker.name }

    fn patch_controller(&self) -> &PatchController {
        todo!()
    }

    fn parent_id(&self) -> &Option<NodeId> { &self.parent_id }

    fn locals(&self) -> &DualKeyMap<usize, String, String> { todo!() }

    fn process(&mut self) -> Option<String> {
        todo!()
    }
}
