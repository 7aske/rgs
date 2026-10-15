use glob::Pattern;
use crate::print::{OutputType, SortType, SummaryType};
use std::{io, env, fs};
use std::collections::HashSet;
use std::fs::File;
use std::path::{Path, PathBuf};
use std::iter::FromIterator;
use std::io::BufRead;
use structopt::StructOpt;
use serde_derive::Deserialize;
use toml::value::Table;

#[derive(Debug, StructOpt, Deserialize)]
#[structopt(name = "rgs",
about = "Batch repository check tool (github.com/7aske/rgs)",
author = "Nikola Tasić - 7aske.com",
version = env!("CARGO_PKG_VERSION"))]
pub struct RgsOptStruct {
    #[structopt(short = "c", long = "code", env, default_value = "", help = "override CODE variable")]
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
    #[structopt(long = "list-profiles", help = "list profiles from 'coderc'")]
    pub list_profiles: bool,
    #[structopt(long = "set-profile", name = "name", help = "set default profile in '~/.coderc'")]
    pub set_profile: Option<String>,
    #[structopt(long = "unset-profile", help = "unset default profile in '~/.coderc'")]
    pub unset_profile: bool,

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
        } else if table.contains_key("branch") {
            // Alias to support singular key in coderc files.
            self.branches = table.get("branch").unwrap().as_bool().unwrap();
        }
    }

    pub fn load_profile(&mut self) {
        let config = match read_coderc() {
            Some((content, _)) => content,
            None => return,
        };

        let config: toml::Value = match toml::from_str(config.as_str()) {
            Ok(v) => v,
            Err(e) => {
                eprintln!("cgs: failed to parse coderc: {}", e);
                return;
            }
        };
        let config = match config.as_table() {
            Some(t) => t,
            None => return,
        };

        let top_level = top_level_options(config);
        if !top_level.is_empty() {
            self.update_with(&top_level);
        }

        let profile = resolve_profile_name(&self.profile, config);
        if profile.is_empty() {
            return;
        }

        if let Some(toml::Value::Table(table)) = config.get(&profile) {
            self.update_with(table);
        } else {
            eprintln!("cgs: profile '{}' not found", profile);
        }
    }

    pub fn list_profiles() -> Result<(), String> {
        let (content, _) = read_coderc().ok_or_else(|| String::from("coderc file not found"))?;
        let config: toml::Value = toml::from_str(content.as_str())
            .map_err(|e| format!("failed to parse coderc: {}", e))?;
        let config = config.as_table()
            .ok_or_else(|| String::from("invalid coderc format"))?;

        let active = resolve_profile_name(&None, config);
        let mut names: Vec<&String> = config.iter()
            .filter(|(_, v)| v.is_table())
            .map(|(k, _)| k)
            .collect();
        names.sort();

        for name in names {
            if *name == active {
                println!("* {}", name);
            } else {
                println!("  {}", name);
            }
        }
        Ok(())
    }

    pub fn set_default_profile(name: &str) -> Result<(), String> {
        if name.is_empty() {
            return Err(String::from("profile name cannot be empty"));
        }

        let (home, config) = coderc_paths();
        let content = if home.exists() {
            fs::read_to_string(&home).map_err(|e| format!("failed to read {}: {}", home.display(), e))?
        } else if config.exists() {
            fs::read_to_string(&config).map_err(|e| format!("failed to read {}: {}", config.display(), e))?
        } else {
            return Err(String::from("coderc file not found"));
        };

        let parsed: toml::Value = toml::from_str(content.as_str())
            .map_err(|e| format!("failed to parse coderc: {}", e))?;
        let table = parsed.as_table()
            .ok_or_else(|| String::from("invalid coderc format"))?;

        match table.get(name) {
            Some(toml::Value::Table(_)) => {}
            _ => return Err(format!("profile '{}' not found", name)),
        }

        let new_content = upsert_profile_key(&content, name);
        fs::write(&home, new_content)
            .map_err(|e| format!("failed to write {}: {}", home.display(), e))?;
        println!("profile set to '{}'", name);
        Ok(())
    }

    pub fn unset_default_profile() -> Result<(), String> {
        let (home, config) = coderc_paths();
        let content = if home.exists() {
            fs::read_to_string(&home).map_err(|e| format!("failed to read {}: {}", home.display(), e))?
        } else if config.exists() {
            fs::read_to_string(&config).map_err(|e| format!("failed to read {}: {}", config.display(), e))?
        } else {
            return Err(String::from("coderc file not found"));
        };

        let parsed: toml::Value = toml::from_str(content.as_str())
            .map_err(|e| format!("failed to parse coderc: {}", e))?;
        parsed.as_table()
            .ok_or_else(|| String::from("invalid coderc format"))?;

        let new_content = remove_profile_key(&content);
        fs::write(&home, new_content)
            .map_err(|e| format!("failed to write {}: {}", home.display(), e))?;
        println!("profile unset");
        Ok(())
    }
}

fn coderc_paths() -> (PathBuf, PathBuf) {
    let home_dir = env::var("HOME").unwrap();
    let location_home = Path::new(&home_dir).join(".coderc");
    let location_config = Path::new(&home_dir).join(".config").join("coderc");
    (location_home, location_config)
}

fn read_coderc() -> Option<(String, PathBuf)> {
    let (location_home, location_config) = coderc_paths();
    if location_home.exists() {
        fs::read_to_string(&location_home).ok().map(|s| (s, location_home))
    } else if location_config.exists() {
        fs::read_to_string(&location_config).ok().map(|s| (s, location_config))
    } else {
        None
    }
}

fn resolve_profile_name(cli_profile: &Option<String>, config: &Table) -> String {
    if let Ok(profile) = env::var("RGS_PROFILE") {
        if !profile.is_empty() {
            return profile;
        }
    }
    if let Some(profile) = cli_profile {
        if !profile.is_empty() {
            return profile.clone();
        }
    }
    if let Some(toml::Value::String(profile)) = config.get("profile") {
        return profile.clone();
    }
    String::new()
}

fn upsert_profile_key(content: &str, name: &str) -> String {
    let profile_line = format!("profile = \"{}\"", name);
    let mut result = String::new();
    let mut replaced = false;
    let mut before_table = true;
    let had_trailing_newline = content.is_empty() || content.ends_with('\n');

    for line in content.lines() {
        let trimmed = line.trim();

        if before_table && trimmed.starts_with('[') {
            before_table = false;
            if !replaced {
                result.push_str(&profile_line);
                result.push('\n');
                if !result.ends_with("\n\n") {
                    result.push('\n');
                }
                replaced = true;
            }
        }

        if before_table {
            let key = trimmed.split('=').next().unwrap_or("").trim();
            if key == "profile" {
                result.push_str(&profile_line);
                result.push('\n');
                replaced = true;
                continue;
            }
        }

        result.push_str(line);
        result.push('\n');
    }

    if !replaced {
        if content.trim().is_empty() {
            return format!("{}\n", profile_line);
        }
        return format!("{}\n\n{}", profile_line, content.trim_start());
    }

    if !had_trailing_newline && result.ends_with('\n') {
        result.pop();
    }
    result
}

fn remove_profile_key(content: &str) -> String {
    let mut result = String::new();
    let mut before_table = true;
    let had_trailing_newline = content.is_empty() || content.ends_with('\n');

    for line in content.lines() {
        let trimmed = line.trim();

        if before_table && trimmed.starts_with('[') {
            before_table = false;
        }

        if before_table {
            let key = trimmed.split('=').next().unwrap_or("").trim();
            if key == "profile" {
                continue;
            }
        }

        result.push_str(line);
        result.push('\n');
    }

    // Normalize accidental triple newlines produced by removing top-level key.
    while result.contains("\n\n\n") {
        result = result.replace("\n\n\n", "\n\n");
    }

    if !had_trailing_newline && result.ends_with('\n') {
        result.pop();
    }
    result
}

fn top_level_options(config: &Table) -> Table {
    let mut top_level = Table::new();
    for (key, value) in config.iter() {
        if !value.is_table() {
            top_level.insert(key.clone(), value.clone());
        }
    }
    top_level
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