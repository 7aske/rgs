use crate::git;
use std::collections::HashMap;

#[derive(Clone, Savefile)]
pub struct Project {
    pub name: String,
    pub grp_name: String,
    pub path: String,
    pub current_branch: String,
    pub branches: Vec<String>,
    #[savefile_ignore]
    pub modified: usize,
    #[savefile_ignore]
    pub time: u64,
    #[savefile_ignore]
    pub ahead_behind: (usize, usize),
    #[savefile_ignore]
    pub remote_ahead_behind: HashMap<String, (usize, usize)>,
    #[savefile_ignore]
    pub fast_forwarded: bool,
}

#[derive(Clone, Savefile)]
pub struct Group {
    pub name: String,
    pub path: String,
    pub projs: Vec<Project>,
    #[savefile_ignore]
    pub not_ok: i32,
}

impl Project {
    pub fn new(name: &str, path: &str, grp_name: &str) -> Self {
        Project {
            name: String::from(name),
            path: String::from(path),
            grp_name: String::from(grp_name),
            current_branch: git::current_branch_from_path(path).unwrap_or_default(),
            branches: git::branches(path),
            modified: 0,
            ahead_behind: (0, 0),
            remote_ahead_behind: HashMap::new(),
            time: 0,
            fast_forwarded: false,
        }
    }

    #[inline]
    pub fn is_clean(&self) -> bool {
        self.modified == 0 && !self.is_ahead_behind()
    }

    #[inline]
    pub fn is_ahead_behind(&self) -> bool {
        self.ahead_behind.0 > 0 || self.ahead_behind.1 > 0
            || self.remote_ahead_behind.iter().any(|x| x.0.ends_with(self.current_branch.clone().as_str()) && (x.1.0 > 0 || x.1.1 > 0))
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
