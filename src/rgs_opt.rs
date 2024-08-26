use glob::Pattern;
use crate::print::{OutputType, SortType, SummaryType};
use std::{io, env};
use std::collections::HashSet;
use std::fs::{File};
use std::path::{Path, PathBuf};
use std::iter::FromIterator;
use std::io::{BufRead, Read};
use structopt::StructOpt;
use serde_derive::Deserialize;
use toml::value::Table;

#[derive(Debug, StructOpt, Deserialize)]
#[structopt(name = "rgs",
about = "Batch repository check tool (github.com/7aske/rgs)",
author = "Nikola Tasić - 7aske.com",
version = env!("CARGO_PKG_VERSION"))]
pub struct RgsOptStruct {
    #[structopt(short = "c", long = "code", env, help = "override CODE variable")]
    pub code: String,
    #[structopt(short = "C", long = "print-code", help = "print CODE variable")]
    pub print_code: bool,
    #[structopt(short = "v", long = "verbose", parse(from_occurrences), help = "print additional information")]
    pub verbose: u8,
    #[structopt(short = "i", long = "no-ignore", help = "don't read .codeignore file")]
    pub no_ignore: bool,
    #[structopt(short = "s", long = "sort", parse(from_str), help = "sort by: directory (d), modifications (m), time (t), ahead-behind (a)")]
    pub sort: Option<SortType>,
    #[structopt(short = "f", long = "fetch", help = "also fetch from origin")]
    pub fetch: bool,
    #[structopt(short = "F", long = "ff", help = "also fast-forward default branch")]
    pub fast_forward: bool,
    #[structopt(short = "D", long = "depth", default_value = "2", help = "project search recursive depth")]
    pub depth: usize,
    #[structopt(short = "p", long = "profile", help = "load profile configuration from 'coderc'")]
    pub profile: Option<String>,

    #[structopt(short = "j", long = "jobs", help = "number of threads, default: number of logical cpus")]
    pub threads: Option<usize>,

    #[structopt(short = "t", long, help = "show execution time")]
    pub time: bool,
    #[structopt(short = "a", long, help = "show both clean and dirty repositories")]
    pub all: bool,
    #[structopt(short = "d", long, help = "show all repository directories (turns off -t and -m flags)")]
    pub dir: bool,
    #[structopt(short = "m", long = "mod", help = "show modifications or ahead/behind status")]
    pub modification: bool,
    #[structopt(short = "b", long = "branches", help = "show remote branch ahead/behind status (assumes -m flag)")]
    pub branches: bool,

    #[structopt(flatten)]
    pub watch_options: RgsWatchOptStruct,
}

#[derive(StructOpt, Debug, Deserialize)]
pub struct RgsWatchOptStruct {
    #[structopt(short = "w", long = "watch", help = "list of repositories to watch")]
    pub repos: Vec<String>,

    #[structopt(short = "T", long = "timeout", default_value = "60", help = "timeout in seconds between git fetches")]
    pub timeout: u64,

    #[structopt(short = "e", long = "exit", help = "exit on first non-zero repository ahead-behind diff")]
    pub exit: bool,

    #[structopt(short = "-n", long = "notify", help = "send an OS notification on every non-zero diff")]
    pub notify: bool,
}


impl RgsOptStruct {
    fn update_with(&mut self, table: &Table) {
        if table.contains_key("code") {
            self.code = String::from(table.get("code").unwrap().as_str().unwrap());
        }
        if table.contains_key("no-ignore") {
            self.no_ignore = table.get("no-ignore").unwrap().as_bool().unwrap();
        }

        if table.contains_key("sort") {
            self.sort = Some(SortType::from(table.get("sort").unwrap().as_str().unwrap()))
        }
        if table.contains_key("fetch") {
            self.fetch = table.get("fetch").unwrap().as_bool().unwrap();
        }
        if table.contains_key("depth") {
            self.depth = table.get("depth").unwrap().as_integer().unwrap() as usize;
        }
        if table.contains_key("jobs") {
            self.threads = Some(table.get("jobs").unwrap().as_integer().unwrap() as usize);
        }
        if table.contains_key("time") {
            self.time = table.get("time").unwrap().as_bool().unwrap();
        }
        if table.contains_key("all") {
            self.all = table.get("all").unwrap().as_bool().unwrap();
        }
        if table.contains_key("dir") {
            self.dir = table.get("dir").unwrap().as_bool().unwrap();
        }
        if table.contains_key("modification") {
            self.modification = table.get("modification").unwrap().as_bool().unwrap();
        } else if table.contains_key("mod") {
            self.modification = table.get("mod").unwrap().as_bool().unwrap();
        }
        if table.contains_key("branches") {
            self.branches = table.get("branches").unwrap().as_bool().unwrap();
        }
    }

    pub fn load_profile(&mut self) {
        let env_profile = env::var("RGS_PROFILE");
        // if RGS_PROFILE and cli argument profile is not set we don't have
        // anything to do.
        if self.profile.is_none() && env_profile.is_err() {
            return;
        }

        let profile = if env_profile.is_ok() {
            env_profile.unwrap()
        } else {
            self.profile.clone().unwrap()
        };

        // If the RGS_PROFILE is set but it is set to an empty string, we don't
        // have anything to do also.
        if profile.is_empty() {
            return;
        }

        let location_config = Path::new(&env::var("HOME").unwrap()).join(".config").join("coderc");
        let location_home = Path::new(&env::var("HOME").unwrap()).join(".coderc");
        let mut config_string = String::new();
        if location_home.exists() {
            let mut file = File::open(location_home).unwrap();
            file.read_to_string(&mut config_string);
        } else if location_config.exists() {
            let mut file = File::open(location_config).unwrap();
            file.read_to_string(&mut config_string);
        }

        let config: toml::Value = toml::from_str(config_string.as_str()).unwrap();
        let config = config.as_table().unwrap();
        if config.contains_key(&profile) {
            let config = config.get(&profile).unwrap().as_table().unwrap();
            self.update_with(config);
        } else {
            eprintln!("cgs: profile '{}' not found", profile);
        }
    }
}

pub struct RgsOpt {
    pub code: String,
    pub print_code: bool,
    pub codeignore: Vec<Pattern>,
    pub codeignore_exclude: Vec<Pattern>,
    pub out_types: Vec<OutputType>,
    pub sort: SortType,
    pub summary_type: SummaryType,
    pub fetch: bool,
    pub fast_forward: bool,
    pub depth: usize,
    pub threads: usize,

    pub watch: bool,
    pub repos: Vec<PathBuf>,
    pub timeout: u64,
    pub exit: bool,
    pub notify: bool,
    pub branches: bool,
}

#[inline(always)]
fn parse_codeignore(code: &String, no_codeignore: bool) -> (Vec<Pattern>, Vec<Pattern>) {
    if no_codeignore {
        return (vec![], vec![]);
    }

    let mut codeignore = vec![];
    let mut codeignore_exclude = vec![];
    let file = File::open(Path::new(code).join(".codeignore"));
    if file.is_ok() {
        let file = file.unwrap();
        let lines = io::BufReader::new(file)
            .lines()
            .filter_map(|line| line.ok())
            .filter(|line| !line.starts_with("#"))
            .collect::<Vec<String>>();
        for line in lines {
            if line.starts_with("!") {
                let line = line.chars().skip(1).collect::<String>();
                codeignore_exclude.push(Pattern::new(line.as_str()).unwrap());
            } else  {
                codeignore.push(Pattern::new(line.as_str()).unwrap());
            }
        }
    };

    return (codeignore, codeignore_exclude);
}


impl From<&RgsOptStruct> for RgsOpt {
    fn from(opt: &RgsOptStruct) -> Self {
        let code = String::from(&opt.code);
        let print_code = opt.print_code;

        let (codeignore, codeignore_exclude) = parse_codeignore(&code, opt.no_ignore);
        

        let mut out_types: HashSet<OutputType> = HashSet::new();
        if opt.all {
            out_types.insert(OutputType::All);
        }

        if opt.time {
            out_types.insert(OutputType::Time);
        }

        if opt.modification {
            out_types.insert(OutputType::Modification);
        }

        if opt.dir {
            out_types.insert(OutputType::Dir);
            out_types.retain(|x| *x != OutputType::Modification && *x != OutputType::Time);
        }

        if opt.branches {
            out_types.insert(OutputType::Branches);
            out_types.insert(OutputType::Modification);
        }
        if opt.fetch {
            out_types.insert(OutputType::Branches);
        }

        let out_types = Vec::from_iter(out_types);

        let sort = opt.sort.unwrap_or_default();
        let fetch = opt.fetch;
        let fast_forward = opt.fast_forward;
        let depth = opt.depth;
        let branches = opt.branches;
        let threads = opt.threads.unwrap_or(num_cpus::get());
        let summary_type = SummaryType::from_occurrences(opt.verbose as u64);

        let watch = opt.watch_options.repos.len() > 0;
        let repos = opt.watch_options.repos.clone().iter()
            .map(|repo| {
                let repo_path = PathBuf::from(repo);

                return if repo_path.is_absolute() {
                    repo_path
                } else {
                    Path::new(&code).join(Path::new(repo))
                };
            })
            .collect::<Vec<PathBuf>>();

        let timeout = opt.watch_options.timeout;
        let exit = opt.watch_options.exit;
        let notify = opt.watch_options.notify;

        RgsOpt {
            code,
            print_code,
            codeignore,
            codeignore_exclude,
            summary_type,
            out_types,
            sort,
            fetch,
            fast_forward,
            depth,
            threads,
            watch,
            repos,
            timeout,
            exit,
            notify,
            branches,
        }
    }
}