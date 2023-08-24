use glob::{GlobResult};
use mpsc::Sender;
use savefile::prelude::*;
use std::ops::Sub;
use std::path::{Path, PathBuf};
use std::sync::mpsc::channel;
use std::sync::mpsc;
use std::time::{Instant, SystemTime, Duration};
use std::{fs, io, thread, process};
use threadpool::ThreadPool;

use crate::lang::{Group, Project};
use crate::print::{OutputType, print_projects};
use crate::rgs_opt::RgsOpt;
use std::fmt::{Display, Formatter};
use chrono::{NaiveDateTime};
use crate::git;
use crate::notify::notify;
use colored::*;

extern crate savefile;

#[derive(Debug)]
pub struct RgsError {
    message: String,
}

impl Display for RgsError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "cgs: {}", self.message)
    }
}

impl From<&str> for RgsError {
    fn from(val: &str) -> Self {
        RgsError { message: String::from(val) }
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
        let threads = opts.threads;
        Rgs {
            opts,
            count: 0,
            groups: vec![],
            pool: ThreadPool::new(threads),
        }
    }

    fn validate_code(&self) -> Result<(), RgsError> {
        if self.opts.code.is_empty() {
            return Err(RgsError::from("'CODE' env variable is not set"));
        }

        if !Path::new(&self.opts.code).exists() {
            return Err(RgsError::from(format!("{}: no such file or directory", self.opts.code).as_str()));
        }
        Ok(())
    }

    pub fn run(&mut self) -> Result<(), RgsError> {
        self.validate_code()?;
        if self.opts.print_code {
            println!("{}", self.opts.code);
        } else if self.opts.watch {
            self.run_watch()?
        } else {
            self.load_repos();

            if self.opts.fetch {
                self.fetch_projs();
            }

            if !self.is_showing_only_all_dirs() {
                self.update_projs();
            }

            // we do ff after updating proj stats
            // otherwise it would require doing ahead/behind status parsing twice
            if self.opts.fast_forward {
                self.fast_forward_projs();
            }

            self.print();

            if self.opts.notify {
                for grp in &self.groups {
                    for proj in &grp.projs {
                        let repo = Path::new(&proj.path).to_path_buf();
                        if proj.ahead_behind.1 != 0 {
                            self.parse_and_notify(&repo, &proj.current_branch);
                        }
                    }
                }
                self.pool.join();
            }
        }
        Ok(())
    }

    #[inline(always)]
    fn parse_and_notify(&self, repo: &PathBuf, branch: &String) {
        const SUMMARY_ABBR_LEN: usize = 60;
        const MAX_COMMITS_BODY: usize = 10;
        if self.opts.watch {
            println!("{}:{}\n", repo.to_str().unwrap(), branch);
        }

        let commits_opt = git::behind_commits(&repo.to_str().unwrap(), branch);
        match commits_opt {
            Ok(commits) => {
                if !commits.is_empty() {
                    let commits_len = commits.len();
                    let mut notify_body = String::from(format!("{}:{} ({})\n\n", repo.to_str().unwrap(), branch, commits_len));
                    let mut processed_commits = 0;
                    for commit in commits {
                        if processed_commits < MAX_COMMITS_BODY {
                            let time = NaiveDateTime::from_timestamp(commit.time.seconds(), 0);
                            if self.opts.watch {
                                println!("{}\n{}\n{} @ {}\n", commit.id, commit.summary, commit.author, time.format("%Y-%m-%d %H:%M:%S"));
                            }
                            let commit_abbr: String = commit.id.chars().take(8).collect();
                            let summary_abbr: String = commit.summary.chars().take(SUMMARY_ABBR_LEN).collect();
                            let dots = if commit.summary.len() > SUMMARY_ABBR_LEN {
                                "..."
                            } else {
                                ""
                            };

                            notify_body += format!("{} {}{}\n", commit_abbr, summary_abbr, dots).as_str();
                            processed_commits += 1;
                        } else if processed_commits == MAX_COMMITS_BODY {
                            let more_commits_msg = format!("\n{} more commit(s)...", commits_len - processed_commits);
                            notify_body += more_commits_msg.as_str();
                            if self.opts.watch {
                                println!("{}", more_commits_msg);
                            }
                            processed_commits += 1;
                        } else {
                            break;
                        }
                    }

                    if self.opts.notify {
                        let repo = PathBuf::from(repo);
                        self.pool.execute(move || {
                            notify(&repo, &notify_body);
                        });
                    }

                    if self.opts.exit {
                        self.pool.join();
                        process::exit(commits_len as i32);
                    }
                }
            }
            Err(_) => {}
        }
    }

    fn run_watch(&mut self) -> Result<(), RgsError> {
        // check only once if there is an invalid repository
        for repo in &self.opts.repos {
            if !git::is_inside_work_tree(&repo.to_str().unwrap()) {
                return Err(RgsError::from(format!("'{}': not a valid repository", repo.to_str().unwrap()).as_str()));
            }
        }

        loop {
            for repo in &self.opts.repos {
                let branch = git::current_branch_from_path(repo).unwrap_or_default();
                let fetch = git::fetch(repo.to_str().unwrap(), &String::from("origin"), &[&branch]);
                if fetch.is_ok() {
                    self.parse_and_notify(repo, &branch);
                } else if !fetch.is_ok() && self.opts.repos.len() == 1 {
                    return Err(RgsError::from(fetch.unwrap_err().message()));
                }
            }

            thread::sleep(Duration::from_secs(self.opts.timeout));
        }
    }

    #[inline]
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

        match self.list_dir(String::from(&self.opts.code), self.opts.depth) {
            Ok(_) => {}
            Err(err) => { eprintln!("cgs: error: {}", err.to_string()) }
        }

        let paths = self.opts.codeignore_exclude
            .iter()
            .map(|p| String::from(&self.opts.code) + p.as_str())
            .map(|p| glob::glob(p.as_str()).unwrap())
            .flat_map(|g| g.into_iter())
            .collect::<Vec<GlobResult>>();
        let paths = paths
            .iter()
            .map(|r| r.as_ref().unwrap().to_str().unwrap())
            .collect::<Vec<&str>>();

        for path in paths {
            self.process_possible_git_dir(Path::new(path), 1);
        }

        self.groups.sort_by(|a, b| a.name.cmp(&b.name));

        // save cache only if it has loaded all the repositories
        if self.opts.codeignore.len() == 0 {
            save_file(cache.to_str().unwrap(), 0, &self.groups).unwrap();
        }
    }

    pub fn print(&mut self) {
        print_projects(&self.groups, &self.opts.summary_type, &self.opts.out_types, &self.opts.sort)
    }

    pub fn fetch_projs(&mut self) {
        let (tx, rx) = channel();

        for i in 0..self.groups.len() {
            for j in 0..self.groups[i].projs.len() {
                let path = String::from(&self.groups[i].projs[j].path);
                // let branch = String::from(&self.groups[i].projs[j].current_branch);
                let tx = Sender::clone(&tx);
                self.pool.execute(move || {
                    let now = Instant::now();
                    let res = git::fetch_all(&path);
                    if res.is_err() {
                        let err_msg = format!("error fetching {} - {}", path, res.err().unwrap().message());
                        eprintln!("{}", err_msg.red());
                    }
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
                let branch = String::from(&self.groups[i].projs[j].current_branch);
                let branches = self.opts.branches;
                let tx = Sender::clone(&tx);
                self.pool.execute(move || {
                    let now = Instant::now();
                    let modified = git::is_clean(&path);
                    let ahead_behind = git::ahead_behind(&path, &branch).unwrap_or((0, 0));
                    let ahead_behind_remote = if branches {
                        git::ahead_behind_remote(&path).unwrap_or(vec![])
                    } else {
                        vec![]
                    };
                    tx.send((i, j, modified, ahead_behind, ahead_behind_remote, now.elapsed().as_millis() as u64)).unwrap();
                });
            }
        }

        drop(tx);
        self.pool.join();

        for (i, j, modified, ahead_behind, ahead_behind_remote, time) in rx {
            let proj = &mut self.groups[i].projs[j];
            proj.modified = modified;
            proj.ahead_behind = ahead_behind;
            for data in ahead_behind_remote {
                proj.remote_ahead_behind.insert(format!("{}/{}", data.0, data.1), (data.2, data.3));
            }
            proj.time += time;
        }
    }


    pub fn list_dir(&mut self, path: String, depth: usize) -> io::Result<()> {
        if depth == 0 { return Ok(()); }

        for entry in fs::read_dir(path)? {
            self.process_possible_git_dir(&entry.unwrap().path(), depth);
        };
        Ok(())
    }

    fn process_possible_git_dir(&mut self, path: &Path, depth: usize) {
        let path_str = path.to_str().unwrap();
        let replaced = path_str.replace(&self.opts.code, "");
        let path_root = replaced.as_str();

        let mut skip = false;
        if self.opts.codeignore.iter().any(|g| g.matches(path_root)) {
            skip = true;
        }

        if self.opts.codeignore_exclude.iter().any(|g| g.matches(path_root)) {
            skip = false;
        }

        if skip {
            return;
        }

        if path.is_dir() {
            let dir_name = path.file_name().unwrap().to_str().unwrap();
            let par_name = path.parent().unwrap().to_str().unwrap();

            if git::is_inside_work_tree(&path_str) {

                self.count += 1;

                // last or new
                let mut lang = self.groups.pop().unwrap_or(Group::new(dir_name, path_str));

                // if its a top-level repository (eg. uni)
                if self.opts.code.as_str() == par_name {
                    self.groups.push(lang);
                    lang = Group::new("CODE", path_str);
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
                self.list_dir(path_str.to_string(), depth - 1);
            }
        }
    }

    fn fast_forward_projs(&mut self) {
        let (tx, rx) = channel();

        for i in 0..self.groups.len() {
            for j in 0..self.groups[i].projs.len() {
                let proj = &self.groups[i].projs[j];
                if proj.ahead_behind.1 == 0 {
                    continue;
                }

                let path = String::from(&proj.path);
                let branch = String::from(&proj.current_branch);
                let tx = Sender::clone(&tx);
                self.pool.execute(move || {
                    let now = Instant::now();
                    let res= git::fast_forward(&path, &branch);
                    if res.is_err() {
                        let err_msg = format!("error fast-forwarding {}:{} - {}", path, branch, res.as_ref().err().unwrap().message());
                        eprintln!("{}", err_msg.red());
                    }
                    tx.send((i, j, now.elapsed().as_millis() as u64, res.is_ok())).unwrap()
                });
            }
        }

        drop(tx);

        for (i, j, time, result) in rx {
            let proj = &mut self.groups[i].projs[j];
            proj.fast_forwarded = result;

            // if ff was successful we manually set behind commits to zero
            if result {
                proj.ahead_behind.1 = 0
            }
            proj.time += time;
        }

        self.pool.join();
    }
}