// Standard Uses
use std::collections::HashMap;

// Crate Uses

// External Uses


#[derive(Default, Clone)]
pub struct ListenerController {
    pub location_content: HashMap<String, String>,
    // pub locations: Vec<String>,
    // pub contents: Vec<String>,

    pub state: State,
    pub index: usize,
    pub processing: bool,
}


#[allow(unused)]
impl ListenerController {
    pub fn new() -> Self {
        Self {
            location_content: Default::default(),
            state: Default::default(),
            index: 0,
            processing: false,
        }
    }

    pub fn locations(&self) -> &Vec<String> {
        todo!()
        // &self.locations
    }

    pub fn contents(&self) -> &Vec<String> {
        todo!()
        // &self.contents
    }

    pub fn queue(&mut self, location: String, content: String) {
        if self.processing { return }

        self.processing = true;

        self.location_content.entry(location).or_insert(content);

        // self.locations.push(location);
        // self.contents.push(content);
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

        /*
        for handler in self.handlers.iter_mut() {
            handler.borrow_mut().visit();
        }
        */

        self.post_process()
    }

    pub fn process_with(&mut self, location: &str, content: &str) {
        println!("'{location}': {content}");
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

