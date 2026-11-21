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
use miette::{miette, Result};


#[derive(Default)]
pub struct TextFile {
    pub name: String,
    pub content: String,

    pub parent: Option<Weak<RefCell<dyn Group>>>,
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
    fn parent(&self) -> &Option<Weak<RefCell<dyn Group>>> { &self.parent }
    // fn controller(&self) -> &Option<ListenerController> { &self.controller }
    fn state(&self) -> &FileState { &self.state }
}


pub struct TextGroup {
    pub name: String,
    pub root: Option<Weak<RefCell<dyn Target>>>,
    pub parent: Option<Weak<RefCell<dyn Group>>>,
    pub groups: Vec<Rc<RefCell<dyn Group>>>,
    pub files: Vec<Rc<RefCell<dyn File>>>
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
    fn root(&self) -> &Option<Weak<RefCell<dyn Target>>> { &self.root }
    fn parent(&self) -> &Option<Weak<RefCell<dyn Group>>> { &self.parent }
    fn groups(&self) -> &Vec<Rc<RefCell<dyn Group>>> { &self.groups }
    fn files(&self) -> &Vec<Rc<RefCell<dyn File>>> { &self.files }

    fn add_root(&mut self, root: Weak<RefCell<dyn Target>>) {
        self.root = Some(root);
    }

    fn add_file(&mut self, file: Rc<RefCell<dyn File>>) {
        self.files.push(file);
    }
}

pub struct TextSolution {
    pub name: String,
    pub groups: Vec<Rc<RefCell<dyn Group>>>,
    pub file_map: Option<FileMap>
}

#[allow(unused)]
impl TextSolution {
    pub fn from_target(path: &Path) -> Result<Rc<RefCell<TextSolution>>> {
        let project = Project::from_path(path)?;

        fn group_recursive(
            group: &project::Group, parent: Option<Weak<RefCell<dyn Group>>>,
            root: &Weak<RefCell<dyn Target>>
        ) -> Rc<RefCell<TextGroup>> {
            let solution_group = TextGroup::new_shared(group.name.to_string());
            solution_group.borrow_mut().parent = parent;
            solution_group.borrow_mut().root = Some(root.clone());

            let as_parent: Weak<RefCell<dyn Group>> = Rc::downgrade(&solution_group) as _;

            for project_group in &group.groups {
                let children = group_recursive(project_group, Some(as_parent.clone()), root);

                solution_group.borrow_mut().groups.push(children as _);
            }

            solution_group
        }

        let root = project.groups.iter()
            .find(|group| group.name == "root")
            .ok_or_else(|| miette!("Target project '{}' has no 'root' group", project.name))?;

        let solution = Rc::new(RefCell::new(Self {
            name: project.name.clone(),
            groups: vec![],
            file_map: None
        }));
        let solution_weak: Weak<RefCell<dyn Target>> = Rc::downgrade(&solution) as _;

        let root_group = group_recursive(root, None, &solution_weak);

        solution.borrow_mut().groups.push(root_group as _);
        solution.borrow_mut().file_map = Some(FileMap::from_target(solution_weak));

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

    pub fn add_group(&mut self, group: Rc<RefCell<dyn Group>>) {
        self.groups.push(group);
    }
}

#[allow(unused)]
impl Target for TextSolution {
    fn name(&self) -> &String { &self.name }
    fn groups(&self) -> &Vec<Rc<RefCell<dyn Group>>> { &self.groups }
    fn operation(&self) -> &Option<Operation> { todo!() }
    fn ast_set(&self) { todo!() }

    fn add_group(&mut self, group: Rc<RefCell<dyn Group>>) {
        self.groups.push(group);
    }
}
