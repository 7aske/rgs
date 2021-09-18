use crate::git;
use std::collections::HashMap;
use git2::Error;
use std::borrow::Borrow;

#[derive(Clone, Savefile)]
pub struct Project {
    pub name: String,
    pub grp_name: String,
    pub path: String,
    pub current_branch: String,
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
    }

    #[inline]
    pub fn fetch(&self) -> Result<(), Error> {
        git::fetch(&self.path, &self.current_branch)
    }

    #[inline]
    pub fn fetch_branch(&self, branch: &String) -> Result<(), Error> {
        git::fetch(&self.path, branch)
    }

    #[inline]
    pub fn fetch_all(&self) {
        git::fetch_all(&self.path)
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
