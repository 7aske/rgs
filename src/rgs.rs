use crate::lang::{Group, Project};
use std::{fs, io};
use glob::Pattern;
use std::path::{Path};
use std::sync::mpsc;
use mpsc::Sender;
use crate::git::{git_is_clean, git_is_inside_work_tree, git_fetch, git_ahead_behind};
use std::fs::{File};
use std::io::{BufRead};
use crate::print::{OutputType, SummaryType, print_groups, SortType};
use std::sync::mpsc::channel;
use std::time::{Instant, SystemTime, Duration};
use threadpool::ThreadPool;

extern crate savefile;

use savefile::prelude::*;
use std::ops::Sub;

pub struct Rgs {
    code: String,
    codeignore: Vec<Pattern>,
    out_types: Vec<OutputType>,
    sort: SortType,
    summary_type: SummaryType,
    pub groups: Vec<Group>,
    fetch: bool,
    count: i32,
    depth: i32,
    pool: ThreadPool,
}

impl Rgs {
    pub fn new(code: &String) -> Rgs {
        Rgs {
            code: String::from(code),
            codeignore: vec![],
            count: 0,
            depth: 2,
            fetch: false,
            groups: vec![],
            out_types: vec![],
            sort: SortType::Dir,
            summary_type: SummaryType::Default,
            pool: ThreadPool::new(num_cpus::get()),
        }
    }

    pub fn run(&mut self) {
        self.load_repos();
        if self.fetch {
            self.fetch_projs();
        }
        if !self.is_showing_only_all_dirs() {
            self.update_projs();
        }
        self.print();
    }

    fn is_showing_only_all_dirs(&self) -> bool {
        self.out_types.contains(&OutputType::Dir) && self.out_types.contains(&OutputType::All)
    }

    pub fn load_repos(&mut self) {
        let cache = Path::new(&self.code).join(".codecache");
        if self.is_showing_only_all_dirs() && cache.exists() {
            if let Ok(meta) = cache.metadata() {
                if let Ok(mod_time) = meta.modified() {
                    if mod_time > SystemTime::now().sub(Duration::from_secs(1800)) {
                        self.groups = load_file(cache.to_str().unwrap(), 0).unwrap();
                        return;
                    }
                }
            }
        }

        match self.list_dir(String::from(&self.code).as_str(), self.depth) {
            Ok(_) => {}
            Err(err) => { eprintln!("rgs: error: {}", err.to_string()) }
        }

        self.groups.sort_by(|a, b| a.name.cmp(&b.name));
        save_file(cache.to_str().unwrap(), 0, &self.groups).unwrap();
    }

    pub fn print(&mut self) {
        print_groups(&self.groups, &self.summary_type, &self.out_types, &self.sort)
    }

    pub fn fetch_projs(&mut self) {
        let (tx, rx) = channel();

        for i in 0..self.groups.len() {
            for j in 0..self.groups[i].projs.len() {
                let path = String::from(&self.groups[i].projs[j].path);
                let tx = Sender::clone(&tx);
                self.pool.execute(move || {
                    let now = Instant::now();
                    git_fetch(&path);
                    tx.send((i, j, now.elapsed().as_millis() as u64)).unwrap()
                });
            }
        }

        drop(tx);

        for (i, j, time) in rx {
            let proj = &mut self.groups[i].projs[j];
            proj.time += time;
        }

        self.pool.join();
    }

    pub fn update_projs(&mut self) {
        let (tx, rx) = channel();

        for i in 0..self.groups.len() {
            for j in 0..self.groups[i].projs.len() {
                let path = String::from(&self.groups[i].projs[j].path);
                let tx = Sender::clone(&tx);
                self.pool.execute(move || {
                    let now = Instant::now();
                    let modified = git_is_clean(&path);
                    let ahead_behind = git_ahead_behind(&path).unwrap_or((0, 0));
                    tx.send((i, j, modified, ahead_behind, now.elapsed().as_millis() as u64)).unwrap();
                });
            }
        }

        drop(tx);
        self.pool.join();

        for (i, j, modified, ahead_behind, time) in rx {
            let proj = &mut self.groups[i].projs[j];
            proj.modified = modified;
            proj.ahead_behind = ahead_behind;
            proj.time += time;
        }
    }


    pub fn list_dir(&mut self, path: &str, depth: i32) -> io::Result<()> {
        if depth == 0 { return Ok(()); }

        for entry in fs::read_dir(path)? {
            let path = entry?.path();
            let path_str = path.to_str().unwrap();
            let replaced = path_str.replace(&self.code, "");
            let path_root = replaced.as_str();

            if self.codeignore.iter().any(|g| g.matches(path_root)) {
                continue;
            }

            if path.is_dir() {
                let dir_name = path.file_name().unwrap().to_str().unwrap();
                let par_name = path.parent().unwrap().to_str().unwrap();

                if git_is_inside_work_tree(&path_str) {

                    // check if it is a link
                    match fs::read_link(path.clone()) {
                        Ok(res) => {
                            // if its not an absolute link treat it as an alias
                            if !res.is_absolute() {
                                continue;
                            }

                            // if it is an absolute link within code directory treat it as an alias
                            if res.starts_with(self.code.as_str()) {
                                continue;
                            }
                        }
                        Err(_) => {}
                    }

                    self.count += 1;

                    // last or new
                    let mut lang = self.groups.pop().unwrap_or(Group::new(dir_name, path_str));

                    // if its a top-level repository (eg. uni)
                    if self.code.as_str() == par_name {
                        self.groups.push(lang);
                        lang = Group::new(dir_name, path_str);
                    } else {
                        let code = Path::new(&self.code);
                        let root = code.join(Path::new(&lang.name));
                        let root = root.to_str().unwrap();
                        if root != par_name {
                            let code_len = code.to_str().unwrap().len() + 1;
                            let lang_name = &par_name[code_len..];
                            self.groups.push(lang);
                            lang = Group::new(&lang_name, path_str);
                        }
                    }

                    lang.add_project(Project::new(dir_name, path_str, &lang.name.as_str()));
                    self.groups.push(lang);
                } else {
                    if self.code == par_name {
                        self.groups.push(Group::new(dir_name, path_str));
                    }
                    self.list_dir(path_str, depth - 1)?;
                }
            }
        };
        Ok(())
    }

    pub fn sort(mut self, sort: SortType) -> Self {
        self.sort = sort;
        self
    }

    pub fn fetch(mut self, fetch: bool) -> Self {
        self.fetch = fetch;
        self
    }

    pub fn out_types(mut self, out_types: Vec<OutputType>) -> Self {
        self.out_types = out_types;
        self
    }

    pub fn summary(mut self, summary: SummaryType) -> Self {
        self.summary_type = summary;
        self
    }

    pub fn depth(mut self, depth: i32) -> Self {
        self.depth = depth;
        self
    }


    pub fn codeignore(mut self, codeignore: bool) -> Self {
        if !codeignore {
            return self;
        }

        self.codeignore = match File::open(Path::new(&self.code).join(".codeignore")) {
            Ok(file) => {
                io::BufReader::new(file)
                    .lines()
                    .filter_map(|line| line.ok())
                    .filter(|line| !line.starts_with("#"))
                    .map(|line| Pattern::new(line.as_str()).unwrap())
                    .collect()
            }
            Err(_) => { vec![] }
        };

        self
    }
}