// Standard Uses

// Crate Uses
use crate::shaping::operation::actions::Action;
use crate::shaping::operation::expressions::Expression;
use crate::shaping::patch::controller::PatchController;
use crate::shaping::patch::Builder as PatchBuilder;
use crate::ast::listener::Controller;

// External Uses


pub struct Builder {
    name: String,
    builder: PatchBuilder,
    controller: PatchController,
    built: bool,
    location: Option<String>,
    result: String
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

        let locations = controller.locations();

        let location = locations.last().unwrap();
        let content = controller.contents().last().unwrap();

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
    fn matches(self, _content: &str) -> &str {
        todo!()
    }
}


fn build_groups(r#_match: &str, _build: &str, _content: &str) -> bool {
    todo!()
}

