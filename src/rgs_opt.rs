use glob::Pattern;
use crate::print::{OutputType, SortType, SummaryType};
use std::{io, env};
use std::collections::HashSet;
use std::fs::File;
use std::path::Path;
use std::iter::FromIterator;
use std::io::{BufRead, Read};
use structopt::StructOpt;
use serde_derive::Deserialize;
use toml::value::Table;


#[derive(Debug, Deserialize)]
pub struct CodeRc {
    profiles: Vec<RgsOptStruct>,
}

#[derive(Debug, StructOpt, Deserialize)]
#[structopt(name = "rgs",
about = "Batch repository check tool (github.com/7aske/rgs)",
author = "Nikola Tasić - 7aske.com",
version = "1.0.3")]
pub struct RgsOptStruct {
    #[structopt(short = "c", long = "code", env, help = "override CODE variable")]
    pub code: String,
    #[structopt(short = "v", long = "verbose", parse(from_occurrences), help = "print additional information")]
    pub verbose: u8,
    #[structopt(short = "i", long = "no-ignore", help = "don't raed .codeignore file")]
    pub no_ignore: bool,
    #[structopt(short = "s", long = "sort", parse(from_str), help = "sort by: directory (d), modifications (m), time (t), ahead-behind (a)")]
    pub sort: Option<SortType>,
    #[structopt(short = "f", long = "fetch", help = "also fetch from origin")]
    pub fetch: bool,
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
    }

    pub fn load_profile(&mut self) {
        if self.profile.is_none() {
            return;
        }
        let profile = self.profile.as_ref().unwrap().as_str();

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
        if config.contains_key(profile) {
            let config = config.get(profile).unwrap().as_table().unwrap();
            self.update_with(config);
        } else {
            eprintln!("rgs: profile '{}' not found", profile);
        }
    }
}

pub struct RgsOpt {
    pub code: String,
    pub codeignore: Vec<Pattern>,
    pub codeignore_exclude: Vec<Pattern>,
    pub out_types: Vec<OutputType>,
    pub sort: SortType,
    pub summary_type: SummaryType,
    pub fetch: bool,
    pub depth: usize,
    pub threads: usize,
}

#[inline(always)]
fn parse_codeignore(code: &String, no_codeignore: bool) -> (Vec<Pattern>, Vec<Pattern>) {
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
            } else if !no_codeignore {
                codeignore.push(Pattern::new(line.as_str()).unwrap());
            }
        }
    };

    return (codeignore, codeignore_exclude);
}


impl From<&RgsOptStruct> for RgsOpt {
    fn from(opt: &RgsOptStruct) -> Self {
        let code = String::from(&opt.code);

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

        let out_types = Vec::from_iter(out_types);

        let sort = opt.sort.unwrap_or_default();
        let fetch = opt.fetch;
        let depth = opt.depth;
        let threads = opt.threads.unwrap_or(num_cpus::get());
        let summary_type = SummaryType::from_occurrences(opt.verbose as u64);

        RgsOpt {
            code,
            codeignore,
            codeignore_exclude,
            summary_type,
            out_types,
            sort,
            fetch,
            depth,
            threads,
        }
    }
}