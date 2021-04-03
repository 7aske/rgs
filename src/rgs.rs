use crate::lang::{Group, Project};
use std::{fs, thread, io};
use glob::Pattern;
use std::path::{Path, Iter};
use std::thread::JoinHandle;
use std::sync::{mpsc, Arc, Barrier};
use mpsc::Sender;
use crate::git::{git_is_clean, git_is_inside_work_tree, git_fetch, git_is_clean_remote};
use std::fs::File;
use std::io::BufRead;
use crate::print::{OutputType, SummaryType, print_groups};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::mpsc::channel;

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
                self.update_projs();
                self.groups.sort_by(|a, b| a.name.cmp(&b.name));
                print_groups(&self.groups, &self.summary_type, &self.out_types)
            }

            Err(err) => { eprintln!("rgs: error: {}", err.to_string()) }
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
                    let is_ok = git_is_clean(&path);
                    tx.send((i, j, is_ok)).unwrap();
                });
                handles.push(handle);
            }
        }

        for handle in handles {
            handle.join().unwrap();
        }
        drop(tx);
        rx.recv().unwrap();
        for (i, j, is_ok) in rx {
            self.groups[i].projs[j].is_ok = is_ok;
        }
    }


    pub fn list_dir(&mut self, path: &str, mut depth: i32) -> io::Result<()> {
        if depth == 0 { return Ok(()); }
        depth -= 1;

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
                    self.list_dir(path_str, depth)?;
                }
            }
        };
        Ok(())
    }
}