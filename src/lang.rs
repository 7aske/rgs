#[derive(Clone)]
pub struct Project {
    pub name: String,
    pub path: String,
    pub modified: usize,
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
            modified: 0,
            ahead_behind: (0, 0),
            time: 0,
        }
    }
    pub fn is_clean(&self) -> bool {
        self.modified == 0 && !self.is_ahead_behind()
    }

    pub fn is_ahead_behind(&self) -> bool {
        self.ahead_behind.0 > 0 || self.ahead_behind.1 > 0
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
