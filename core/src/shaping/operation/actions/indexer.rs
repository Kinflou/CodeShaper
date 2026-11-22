// Standard Uses
use std::option::Option;

// Crate Uses
use crate::expressions::visitor;
use crate::shaping::patch::resolver::{List, Switch};
use crate::shaping::operation::actions::Action;
use crate::shaping::operation::actions::composer::Composer;
use crate::shaping::patch::controller::PatchController;

// External Uses
use indextree::NodeId;



#[allow(unused)]
pub struct ListAction{
    list: List,
    self_id: Option<NodeId>,
    parent_id: Option<NodeId>,
    composer: Option<Composer>
}

#[allow(unused)]
impl ListAction {
    pub fn with_ref(list: List, parent_id: Option<NodeId>) -> Self {
        Self { list, self_id: None, parent_id, composer: None }
    }

    pub fn post_new(&mut self, self_id: NodeId) {
        self.self_id = Some(self_id);
        self.composer = Some(Composer::with(self_id));
    }

    pub fn index(&self, controller: &PatchController, index: usize) -> Option<String> {
        if self.list.values.is_empty() { return None }

        if index <= self.list.values.len() {
            return self.list.values.get(index).cloned()
        }

        if let Some(default) = &self.list.default {
            let Some(default_index) = self.resolve_index_expr(
                controller, &self.list.index.clone().unwrap(), &[]
            ) else { return None };

            return self.index(controller, default_index)
        }

        None
    }

    pub fn index_with_args(&self, controller: &PatchController, args: &[String]) -> Option<String> {
        if self.list.values.is_empty() { return None }
        if self.list.index.is_none() { return None }

        if let Some(index) = &self.list.index {
            let Some(index) = self.resolve_index_expr(
                controller,&self.list.index.clone().unwrap(), args
            ) else { return None };

            return self.index(controller, index)
        }

        if let Some(default) = &self.list.default {
            let Some(default_index) = self.resolve_index_expr(
                controller, &self.list.index.clone().unwrap(), args
            ) else { return None };

            return self.index(controller, default_index)
        }

        None
    }

    fn resolve_index_expr(
        &self, controller: &PatchController, expression: &str, arguments: &[String]
    ) -> Option<usize> {
        let result = visitor::navigate_expression(
            self.composer.as_ref().unwrap(), controller,
            expression, arguments
        ).ok();

        result.and_then(|r| r.parse::<usize>().ok())
    }
}

impl Action for ListAction {
    fn name(&self) -> &str { &self.list.name }

    fn self_id(&self) -> &NodeId {
        todo!()
    }

    fn parent_id(&self) -> &Option<NodeId> {
        todo!()
    }

    // fn locals(&self) -> &LocalsMap { todo!() }

    #[allow(unused)]
    fn process(&self, patch_controller: &PatchController) -> Option<String> {
        todo!()
    }
}

#[allow(unused)]
pub struct SwitchAction {
    switch: Switch,
    self_id: Option<NodeId>,
    parent_id: Option<NodeId>,
    composer: Option<Composer>
}

#[allow(unused)]
impl SwitchAction {
    pub fn with_ref(switch: Switch, parent_id: Option<NodeId>) -> Self {
        Self { switch, self_id: None, parent_id, composer: None }
    }

    pub fn post_new(&mut self, self_id: NodeId) {
        self.self_id = Some(self_id);
        self.composer = Some(Composer::with(self_id));
    }

    pub fn resolve(&self, controller: &PatchController, argument: &str) -> Option<String> {
        if self.switch.cases.is_empty() { return None }
        if self.switch.index.is_empty() { return None }

        if self.switch.cases.contains_key(argument) {
            return self.switch.cases.get(argument).cloned()
        }

        if let Some(default) = &self.switch.default {
            let Some(default_index) = self.resolve_index_expr(
                controller, &self.switch.index.clone(), &[]
            ) else { return None };

            return self.resolve(controller, default_index.as_str())
        }

        None
    }

    pub fn resolve_with_args(&self, controller: &PatchController, args: &[String]) -> Option<String> {
        let key = self.resolve_index_expr(controller, &self.switch.index.clone(), args)?;

        self.resolve(controller, &key)
    }

    pub fn resolve_isolated(&self, argument: &str) -> Option<String> {
        if self.switch.cases.is_empty() { return None }
        if self.switch.index.is_empty() { return None }

        if self.switch.cases.contains_key(argument) {
            return self.switch.cases.get(argument).cloned()
        }

        self.switch.default.clone()
    }

    fn resolve_index_expr(
        &self, controller: &PatchController, expression: &str, arguments: &[String]
    ) -> Option<String> {
        let result = visitor::navigate_expression(
            self.composer.as_ref().unwrap(), controller,
            expression, arguments
        ).ok();

        result
    }
}


impl Action for SwitchAction {
    fn name(&self) -> &str { &self.switch.name }

    fn self_id(&self) -> &NodeId {
        todo!()
    }

    fn parent_id(&self) -> &Option<NodeId> {
        todo!()
    }

    // fn locals(&self) -> &LocalsMap { todo!() }

    #[allow(unused)]
    fn process(&self, patch_controller: &PatchController) -> Option<String> {
        todo!()
    }
}

