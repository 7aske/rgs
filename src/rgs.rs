use crate::lang::{Group, Project};
use std::{fs, thread, io};
use glob::Pattern;
use std::path::Path;
use std::sync::mpsc;
use mpsc::Sender;
use crate::git::{git_is_clean, git_is_inside_work_tree, git_fetch, git_ahead_behind};
use std::fs::{File};
use std::io::BufRead;
use crate::print::{OutputType, SummaryType, print_groups, print_progress};
use std::sync::mpsc::channel;
use std::time::Instant;

pub struct Rgs {
    code: String,
    codeignore: Vec<Pattern>,
    fetch: bool,
    out_types: Vec<OutputType>,
    summary_type: SummaryType,
    groups: Vec<Group>,
    count: i32,
    depth: i32,
}

impl Rgs {
    pub fn new(code: String, no_codeignore: bool, fetch: bool, out_types: Vec<OutputType>, summary_type: SummaryType, depth: i32) -> Self {
        let mut codeignore = vec![];
        if !no_codeignore {
            match File::open(Path::new(&code).join(".codeignore")) {
                Ok(file) => {
                    codeignore = io::BufReader::new(file)
                        .lines()
                        .filter_map(|line| line.ok())
                        .filter(|line| !line.starts_with("#"))
                        .map(|line| Pattern::new(line.as_str()).unwrap())
                        .collect()
                }
                Err(_) => {}
            };
        }
        let langs = vec![];

        Rgs {
            code,
            out_types,
            summary_type,
            codeignore,
            fetch,
            depth,
            groups: langs,
            count: 0,
        }
    }

    pub fn run(&mut self) {
        match self.list_dir(String::from(&self.code).as_str(), self.depth) {
            Ok(_) => {
                if self.fetch {
                    self.fetch_projs();
                }
                self.update_projs();
                self.groups.sort_by(|a, b| a.name.cmp(&b.name));
                print_groups(&self.groups, &self.summary_type, &self.out_types)
            }

            Err(err) => { eprintln!("rgs: error: {}", err.to_string()) }
        }
    }

    pub fn fetch_projs(&mut self) {
        let (tx, rx) = channel();
        let (tx_progress, rx_progress) = channel();
        let mut handles = vec![];

        for i in 0..self.groups.len() {
            for j in 0..self.groups[i].projs.len() {
                let path = String::from(&self.groups[i].projs[j].path);
                let tx = Sender::clone(&tx);
                let tx_progress = Sender::clone(&tx_progress);
                let handle = thread::spawn(move || {
                    let now = Instant::now();
                    git_fetch(&path);
                    tx_progress.send((String::from(&path), true));
                    tx.send((i, j, now.elapsed().as_millis())).unwrap()
                });
                handles.push(handle);
            }
        }

        let mut rx_progress = rx_progress.iter();
        let mut count = self.count;
        while count > 0 {
            count -= 1;
            print_progress(self.count, count);
            rx_progress.next();
            print!("{esc}c", esc = 27 as char);
        }

        for handle in handles {
            handle.join().unwrap();
        }

        drop(tx);
        for (i, j, time) in rx {
            let proj = &mut self.groups[i].projs[j];
            proj.time += time;
        }
    }

    pub fn update_projs(&mut self) {
        let (tx, rx) = channel();

        let mut handles = vec![];

        for i in 0..self.groups.len() {
            for j in 0..self.groups[i].projs.len() {
                let path = String::from(&self.groups[i].projs[j].path);
                let tx = Sender::clone(&tx);
                let handle = thread::spawn(move || {
                    let now = Instant::now();
                    let modified = git_is_clean(&path);
                    let ahead_behind = git_ahead_behind(&path).unwrap_or((0, 0));
                    tx.send((i, j, modified, ahead_behind, now.elapsed().as_millis())).unwrap();
                });
                handles.push(handle);
            }
        }

        for handle in handles {
            handle.join().unwrap();
        }
        drop(tx);
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
                    if fs::read_link(path.clone()).is_ok() && !self.out_types.contains(&OutputType::All) {
                        continue;
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

                    lang.add_project(Project::new(dir_name, path_str));
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
}