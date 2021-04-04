#[derive(Clone)]
pub struct Project {
    pub name: String,
    pub path: String,
    pub clean: bool,
    pub time: u128,
    pub ahead_behind: (usize, usize),
}

#[derive(Clone)]
pub struct Group {
    pub name: String,
    pub path: String,
    pub projs: Vec<Project>,
    pub not_ok: i32,
}

impl Project {
    pub fn new(name: &str, path: &str) -> Self {
        Project {
            name: String::from(name),
            path: String::from(path),
            clean: true,
            ahead_behind: (0,0),
            time: 0,
        }
    }
}


impl Group {
    pub fn new(name: &str, path: &str) -> Self {
        Group {
            name: String::from(name),
            path: String::from(path),
            projs: vec![],
            not_ok: 0,
        }
    }
    pub fn add_project(&mut self, proj: Project) {
        self.projs.push(proj)
    }
}
