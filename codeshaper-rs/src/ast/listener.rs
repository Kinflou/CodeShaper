// Standard Uses

// Crate Uses
use crate::shaping::operation::actions::Action;

// External Uses


#[derive(Default)]
// #[derive(Debug, Clone)]
pub struct Controller {
    pub locations: Vec<String>,
    pub contents: Vec<String>,

    pub state: State,
    pub index: usize,
    pub processing: bool,

    pub handlers: Vec<Box<dyn Action>>,
}


impl Controller {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn locations(&self) -> Vec<String> {
        todo!()
    }

    pub fn contents(&self) -> Vec<String> {
        todo!()
    }

    pub fn queue(&mut self, location: String, content: String) {
        if self.processing { return }

        self.processing = true;

        self.locations.push(location);
        self.contents.push(content);
    }

    pub fn stop(&mut self) {
        self.processing = false;
        self.state = State::Stopped;
    }

    #[allow(unused)]
    fn process(&mut self) {
        if self.state != State::Visit {
            self.processing = false;
            return
        }

        self.index += 1;

        /*
        for handler in &mut self.handlers {
            handler.process(self);
        }
        */

        self.post_process()
    }

    #[allow(unused)]
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

