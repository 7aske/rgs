#[derive(Clone)]
pub struct Project {
    pub name: String,
    pub path: String,
    pub is_ok: bool,
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
            is_ok: true,
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
