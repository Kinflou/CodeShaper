// Standard Uses

// Crate Uses
use crate::shaping::operation::actions::Action;
use crate::shaping::operation::expressions::Expression;
use crate::shaping::patch::controller::PatchController;
use crate::shaping::patch::Builder as PatchBuilder;
use crate::ast::listener::Controller;

// External Uses


pub struct Builder {
    pub name: String,
    pub builder: PatchBuilder,
    pub controller: PatchController,
    pub built: bool,
    pub location: Option<String>,
    pub result: String
}

#[allow(unused)]
impl Action for Builder {
    fn name(&self) -> &str { &*self.name }
    fn actions(&self) -> Vec<Box<dyn Action>> { todo!() }
    fn controller(&self) -> &PatchController { &self.controller }
    fn expression(&self) -> &str { &*self.builder.build }
    fn result(&self) -> &str { &*self.result }
    fn expressions(&self) -> Vec<Box<dyn Expression>> { todo!() }

    fn process(&mut self, controller: &Controller) {
        if self.built {
            return
        }

        if self.location.is_some() {
            self.process_expression();
            return
        }

        let (Some(location), Some(context)) = (
            (controller.locations.last(), controller.contents.last())
        ) else { return };

        if !self.builder.reference_location.is_empty() {
            if self.builder.reference_location.to_lowercase() == location.to_lowercase() {
                return
            }
        }
        else if self.builder.location.to_lowercase() != location.to_lowercase() {
            return
        }

        self.process_expression();
    }

    fn process_expression(&mut self) {
        if !self.expression().is_empty() {
            // self.expressions =
        }

        self.built = build_groups("", "", "");
    }
}


#[allow(unused)]
impl Builder {
    fn matches(self, content: &str) -> &str {
        todo!()
    }
}

#[allow(unused)]
fn build_groups(r#match: &str, build: &str, content: &str) -> bool {
    todo!()
}

