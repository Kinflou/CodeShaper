// Relative Modules
mod actions;


// Standard Uses
use std::rc::Rc;
use std::cell::RefCell;

// Crate Uses

// External Uses
use codeshaper_general::target::{File, Group, Target};
use codeshaper_general::target::text::{TextFile, TextGroup, TextSolution};


#[test]
fn make_some() {
    let code = "\
    int main() {
        return 0;
    }
    ".to_string();

    let text_solution: Rc<RefCell<dyn Target>> = TextSolution::new_shared(
        "Test Solution".to_string()
    );

    let text_group: Rc<RefCell<dyn Group>> = TextGroup::new_shared("Root".to_string());
    let mut text_group_mut = RefCell::borrow_mut(&text_group);

    text_group_mut.add_root(Rc::clone(&text_solution));
    RefCell::borrow_mut(&text_solution).add_group(Rc::downgrade(&text_group));

    let text_file: Rc<RefCell<dyn File>> = TextFile::new_shared(
        "test.cpp".to_string(), code
    );

    text_group_mut.add_file(Rc::downgrade(&text_file));

    println!("{:#?}", text_group_mut);

    // let controller = Controller::new();
}
