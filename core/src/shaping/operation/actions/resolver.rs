// Standard Uses
use std::option::Option;
use indextree::NodeId;
use crate::shaping::operation::actions::Action;

// Crate Uses
use crate::shaping::patch;
use crate::shaping::patch::controller::PatchController;
use crate::utils::dual_key_map::DualKeyMap;

// External Uses


#[allow(unused)]
pub struct ResolverAction {
    resolver: patch::resolver::Resolver
}

#[allow(unused)]
impl ResolverAction {
    pub fn with(resolver: patch::resolver::Resolver) -> Self {
        Self { resolver }
    }

    pub fn resolve(&self, argument: &str) -> Option<String> {
        let mode = Mode::from_str(&self.resolver.mode)?;

        match mode {
            Mode::Switch => {
                if self.resolver.cases.contains_key(argument) {
                    return Option::from(self.resolver.cases[argument].clone())
                }

                if self.resolver.default.is_empty() {
                    return None
                }

                Option::from(self.resolver.default.clone())
            }
            Mode::List => {
                if self.resolver.list.is_empty() {
                    return None
                }

                if self.resolver.index.is_empty() {
                    return None
                }

                let index = Self::resolve_index_expr(&self.resolver.index);

                if index <= self.resolver.list.len() {
                    return Option::from(self.resolver.list[index].clone())
                }

                if !self.resolver.default.is_empty() {
                    return Option::from(self.resolver.default.clone())
                }

                None
            }
        }
    }

    fn resolve_index_expr(expression: &str) -> usize {
        todo!()
    }
}

impl Action for ResolverAction {
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

enum Mode {
    Switch,
    List
}

impl Mode {
    pub fn from_str(mode: &str) -> Option<Self> {
        Option::from(match mode {
            "switch" => Self::Switch,
            "list" => Self::List,
            _ => { return None }
        })
    }
}