// Relative Modules
pub mod project;

// Standard Uses
use std::cell::RefCell;
use std::fmt::{Debug, Formatter};
use std::path::Path;
use std::rc::{Rc, Weak};

// Crate Uses
use crate::ast::listener::ListenerController;
use crate::shaping::operation::Operation;
use crate::target::{File, FileState, Group, Target};
use crate::target::text::project::Project;
use crate::target::file_map::FileMap;

// External Uses
use miette::Result;


#[derive(Default)]
pub struct TextFile {
    pub name: String,
    pub content: String,

    pub parent: Option<Box<dyn Group>>,
    pub controller: Option<ListenerController>,
    pub state: FileState
}

impl TextFile {
    pub fn new_shared(name: String, content: String) -> Rc<RefCell<Self>> {
        Rc::new(RefCell::new(
            Self {
                name, content, parent: None,
                controller: None, state: Default::default()
            }
        ))
    }
}

impl File for TextFile {
    fn name(&self) -> &String { &self.name }
    fn parent(&self) -> &Option<Box<dyn Group>> { &self.parent }
    // fn controller(&self) -> &Option<ListenerController> { &self.controller }
    fn state(&self) -> &FileState { &self.state }
}


pub struct TextGroup {
    pub name: String,
    pub root: Option<Rc<RefCell<dyn Target>>>,
    pub parent: Option<Box<dyn Group>>,
    pub groups: Vec<Weak<RefCell<dyn Group>>>,
    pub files: Vec<Weak<RefCell<dyn File>>>
}

impl TextGroup {
    pub fn new_shared(name: String) -> Rc<RefCell<Self>> {
        Rc::new(RefCell::new(
            Self {
                name,
                root: None,
                parent: None,
                groups: vec![],
                files: vec![]
            }
        ))
    }
}

impl Debug for TextGroup {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TextGroup")
            .field("groups", &self.groups)
            .finish()
    }
}

impl Group for TextGroup {
    fn name(&self) -> &String { &self.name }
    fn root(&self) -> &Option<Rc<RefCell<dyn Target>>> { &self.root }
    fn parent(&self) -> &Option<Box<dyn Group>> { &self.parent }
    fn groups(&self) -> &Vec<Weak<RefCell<dyn Group>>> { &self.groups }
    fn files(&self) -> &Vec<Weak<RefCell<dyn File>>> { &self.files }

    fn add_root(&mut self, root: Rc<RefCell<dyn Target>>) {
        self.root = Some(root);
    }

    fn add_file(&mut self, file: Weak<RefCell<dyn File>>) {
        self.files.push(file);
    }
}

pub struct TextSolution {
    pub name: String,
    pub groups: Vec<Weak<RefCell<dyn Group>>>,
    pub file_map: Option<FileMap>
}

#[allow(unused)]
impl TextSolution {
    pub fn from_target(path: &Path) -> Result<Rc<RefCell<TextSolution>>> {
        let project = Project::from_path(path)?;

        fn group_recursive(name: &String, group: &project::Group, text_group: Rc<RefCell<TextGroup>>)
                           -> Rc<RefCell<TextGroup>> {
            let mut solution_group = TextGroup::new_shared(name.to_string());

            for (name, project_group) in &group.groups {
                let children = group_recursive(
                    name, project_group, Rc::clone(&solution_group)
                );

                RefCell::borrow_mut(&solution_group).groups.push(
                    Rc::downgrade(&children) as _
                );
            }

            solution_group
        }

        let root_group = TextGroup::new_shared("root".to_string());
        let root_group = group_recursive(
            &"root".to_string(), &project.groups["root"], root_group
        );

        let mut solution = Rc::new(RefCell::new(Self {
            name: project.name,
            groups: vec![Rc::downgrade(&root_group) as _],
            file_map: None
        }));

        solution.borrow_mut().file_map = Option::from(FileMap::from_target(
            Rc::clone(&solution) as _)
        );

        Ok(solution)
    }

    pub fn new_shared(name: String) -> Rc<RefCell<TextSolution>> {
        Rc::new(RefCell::new(
            Self {
                name,
                groups: vec![],
                file_map: None
            }
        ))
    }

    pub fn add_group(&mut self, group: Weak<RefCell<dyn Group>>) {
        self.groups.push(group);
    }
}

#[allow(unused)]
impl Target for TextSolution {
    fn name(&self) -> &String { &self.name }
    fn groups(&self) -> &Vec<Weak<RefCell<dyn Group>>> { &self.groups }
    fn operation(&self) -> &Option<Operation> { todo!() }
    fn ast_set(&self) { todo!() }

    fn add_group(&mut self, group: Weak<RefCell<dyn Group>>) {
        self.groups.push(group);
    }
}
