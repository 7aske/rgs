use crate::lang::{Group, Project};
use std::{fs, io, env, fmt};
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
use getopts::{Matches};
use std::collections::HashSet;
use std::iter::FromIterator;
use std::convert::TryFrom;
use std::fmt::{Display, Formatter};

#[derive(Debug)]
pub struct RgsOpt {
    code: String,
    codeignore: Vec<Pattern>,
    out_types: Vec<OutputType>,
    sort: SortType,
    summary_type: SummaryType,
    fetch: bool,
    depth: i32,
}

#[derive(Debug)]
pub struct ParseError {
    message: String,
}

impl Display for ParseError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "ParseError: {}", self.message)
    }
}

impl From<&str> for ParseError {
    fn from(value: &str) -> Self {
        ParseError { message: String::from(value) }
    }
}

impl TryFrom<&Matches> for RgsOpt {
    type Error = ParseError;

    fn try_from(matches: &Matches) -> Result<Self, Self::Error> {
        let summary_type = SummaryType::from_occurrences(matches.opt_count("verbose") as u64);

        let code = match matches.opt_str("code") {
            Some(code) => code,
            None => match env::var("CODE") {
                Ok(val) => val,
                Err(_) => {
                    return Err(ParseError::from("'CODE' env variable not set"));
                }
            }
        };

        let mut out_types: HashSet<OutputType> = HashSet::new();
        if matches.opt_present("all") {
            out_types.insert(OutputType::All);
        }

        if matches.opt_present("time") {
            out_types.insert(OutputType::Time);
        }

        if matches.opt_present("modification") {
            out_types.insert(OutputType::Modification);
        }

        if matches.opt_present("dir") {
            out_types.insert(OutputType::Dir);
            out_types.retain(|x| *x != OutputType::Modification && *x != OutputType::Time);
        }

        let mut sort = SortType::None;
        if matches.opt_present("sort") {
            sort = SortType::from(&matches.opt_str("sort").unwrap());

            if sort == SortType::Time {
                out_types.insert(OutputType::Time);
            }

            if sort == SortType::Mod {
                out_types.insert(OutputType::Modification);
            }
        }

        let no_codeignore = matches.opt_present("no-ignore");
        let codeignore = if no_codeignore {
            vec![]
        } else {
            match File::open(Path::new(&code).join(".codeignore")) {
                Ok(file) => {
                    io::BufReader::new(file)
                        .lines()
                        .filter_map(|line| line.ok())
                        .filter(|line| !line.starts_with("#"))
                        .map(|line| Pattern::new(line.as_str()).unwrap())
                        .collect()
                }
                Err(_) => { vec![] }
            }
        };

        let fetch = matches.opt_present("fetch");

        let depth = matches.opt_get_default("depth", String::from("2"))
            .unwrap()
            .parse::<i32>().unwrap();

        Ok(RgsOpt {
            code,
            codeignore,
            out_types: Vec::from_iter(out_types),
            sort,
            summary_type,
            fetch,
            depth,
        })
    }
}

pub struct Rgs {
    opts: RgsOpt,
    groups: Vec<Group>,
    count: i32,
    pool: ThreadPool,
}

impl Rgs {
    pub fn new(opts: RgsOpt) -> Rgs {
        Rgs {
            opts,
            count: 0,
            groups: vec![],
            pool: ThreadPool::new(num_cpus::get()),
        }
    }

    pub fn run(&mut self) {
        self.load_repos();
        if self.opts.fetch {
            self.fetch_projs();
        }
        if !self.is_showing_only_all_dirs() {
            self.update_projs();
        }
        self.print();
    }

    fn is_showing_only_all_dirs(&self) -> bool {
        self.opts.out_types.contains(&OutputType::Dir) && self.opts.out_types.contains(&OutputType::All)
    }

    pub fn load_repos(&mut self) {
        let cache = Path::new(&self.opts.code).join(".codecache");
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

        match self.list_dir(String::from(&self.opts.code).as_str(), self.opts.depth) {
            Ok(_) => {}
            Err(err) => { eprintln!("rgs: error: {}", err.to_string()) }
        }

        self.groups.sort_by(|a, b| a.name.cmp(&b.name));
        save_file(cache.to_str().unwrap(), 0, &self.groups).unwrap();
    }

    pub fn print(&mut self) {
        print_groups(&self.groups, &self.opts.summary_type, &self.opts.out_types, &self.opts.sort)
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
            let replaced = path_str.replace(&self.opts.code, "");
            let path_root = replaced.as_str();

            if self.opts.codeignore.iter().any(|g| g.matches(path_root)) {
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
                            if res.starts_with(self.opts.code.as_str()) {
                                continue;
                            }
                        }
                        Err(_) => {}
                    }

                    self.count += 1;

                    // last or new
                    let mut lang = self.groups.pop().unwrap_or(Group::new(dir_name, path_str));

                    // if its a top-level repository (eg. uni)
                    if self.opts.code.as_str() == par_name {
                        self.groups.push(lang);
                        lang = Group::new(dir_name, path_str);
                    } else {
                        let code = Path::new(&self.opts.code);
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
                    if self.opts.code == par_name {
                        self.groups.push(Group::new(dir_name, path_str));
                    }
                    self.list_dir(path_str, depth - 1)?;
                }
            }
        };
        Ok(())
    }
}