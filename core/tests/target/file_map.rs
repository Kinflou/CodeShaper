// Standard Uses
use std::path::Path;
use std::rc::Rc;
use std::cell::RefCell;

// Crate Uses
use codeshaper_core::target::file_map::FileMap;
use codeshaper_core::target;
use codeshaper_core::target::{File, Group, Target};
use codeshaper_core::target::text::{TextFile, TextGroup, TextSolution};

// External Uses


#[allow(unused)]
#[test]
fn solution_mapping() {
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

    text_group_mut.add_root(Rc::downgrade(&text_solution));
    RefCell::borrow_mut(&text_solution).add_group(Rc::clone(&text_group));

    let text_file: Rc<RefCell<dyn File>> = TextFile::new_shared(
        "test.cpp".to_string(), code
    );

    text_group_mut.add_file(Rc::clone(&text_file));
    drop(text_group_mut);

    let solution_ref = text_solution.borrow();
    let group = &solution_ref.groups()[0];
    let root = group.borrow().root().as_ref().and_then(|root| root.upgrade()).unwrap();

    assert!(Rc::ptr_eq(&root, &text_solution));
    assert_eq!(group.borrow().files()[0].borrow().name(), "test.cpp");

    let file_map = FileMap::from_target(Rc::downgrade(&text_solution));
}

#[test]
fn target_tree_is_owned() {
    let path = Path::new("tests/data/function_name_correction/target/origin/");
    let solution = target::from_type("text", path).unwrap();
    let solution_weak = Rc::downgrade(&solution);

    {
        let solution_ref = solution.borrow();
        let root_group = solution_ref.groups()[0].borrow();

        assert_eq!(root_group.name(), "root");
        assert!(root_group.parent().is_none());

        let root = root_group.root().as_ref().and_then(|root| root.upgrade()).unwrap();
        assert!(Rc::ptr_eq(&root, &solution));
    }

    drop(solution);

    assert!(solution_weak.upgrade().is_none());
}

