// Standard Uses
use std::rc::Rc;
use std::cell::RefCell;

// Crate Uses
use crate::shaping::patch;

// External Uses


#[derive(Default, Clone)]
pub struct Controller {
    pub locations: Vec<String>,
    pub contents: Vec<String>,

    pub state: State,
    pub index: usize,
    pub processing: bool,

    pub handlers: Vec<Rc<RefCell<patch::controller::Controller>>>,
}


#[allow(unused)]
impl Controller {
    pub fn locations(&self) -> &Vec<String> {
        &self.locations
    }

    pub fn contents(&self) -> &Vec<String> {
        &self.contents
    }

    pub fn queue(&mut self, location: String, content: String) {
        if self.processing { return }

        self.processing = true;

        self.locations.push(location);
        self.contents.push(content);
    }

    pub fn visit(&mut self) {
        self.state = State::Visit;
    }

    pub fn stop(&mut self) {
        self.processing = false;
        self.state = State::Stopped;
    }

    pub fn process(&mut self) {
        if self.state != State::Visit {
            self.processing = false;
            return
        }

        self.index += 1;

        for handler in self.handlers.iter_mut() {
            handler.borrow_mut().visit();
        }

        self.post_process()
    }

    fn post_process(&mut self) {
        todo!()
    }
}


#[derive(Default, PartialEq)]
#[derive(Debug, Clone)]
pub enum State {
    #[default]
    Ready,
    Visit,
    Paused,
    Stopped,
    Finished
}

