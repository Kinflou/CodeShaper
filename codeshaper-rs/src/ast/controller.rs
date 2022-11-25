// Standard Uses

// Crate Uses

// External Uses
use antlr_rust::rule_context::RuleContext;
// use crate::ast::set::ASTSet;


/// Abstract Syntax Tree (AST) Visitor Controller
/// Visits nodes one by one and stores the information about them
/// such as text content, location name and the node context object
#[derive(Debug, Clone, Default)]
// #[derive(Builder)]
pub struct VisitorController<'a, T, Ctx> {
    pub ast_set: Box<dyn ASTSet<'a, T, Ctx>>,

    // #[builder(default)]
    pub state: State,

    pub contexts: Vec<Box<dyn RuleContext<'a, TF=T, Ctx=Ctx>>>,
    pub locations: Vec<String>,
    pub contents: Vec<String>,

    // #[builder(default)]
    index: u64,

    // #[builder(default)]
    processing: bool
}

impl<T, Ctx> VisitorController<'static, T, Ctx> where T: Clone, Ctx: Clone {

    pub fn new(ast_set: Box<dyn ASTSet<'static, T, Ctx>>) -> VisitorController<'static, T, Ctx> {
        /*
        let controller = VisitorControllerBuilder::default()
            .ast_set(ast_set)
            .build().unwrap();

        controller
        */

        Self {
            ast_set,
            state: Default::default(),
            contexts: vec![],
            locations: vec![],
            contents: vec![],
            index: 0,
            processing: false
        }
    }

    pub fn request_state_change(&mut self, _state: State) {

    }

    pub fn process(&mut self) {
        if self.processing {
            return
        }

        self.processing = true;

        while self.index < self.contents.len() as u64 {

            if self.state != State::Visit {
                self.processing = false;
                return
            }

            // let context = self.contexts[self.index]
            let location = &self.locations;

            let next_context = self.visitor.visit_children();

            if next_context.is_err() {
                self.finish_process()
            }

            let next_content = self.ast_set.get_context_text(next_context);

            let location_name = todo!();

            self.contexts.push(next_context);
            self.locations.push(location_name);
            // self.contents.push();

            self.index += 1;

            self.post_process()
        }
    }

    pub fn post_process(&self) {

    }

    pub fn finish_process(&self) {

    }
}


#[derive(Debug, Clone, Eq, PartialEq)]
#[derive(Default)]
pub enum State {
    #[default]
    Ready,
    Visit,
    Paused,
    Stopped,
    Finished
}

