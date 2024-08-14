use colored::*;
use std::cmp::Ordering;
use std::str::FromStr;

use crate::lang::{Project, Group};
use std::fmt::{Display, Formatter};
use serde_derive::Deserialize;

// @formatter:off
const COLOR_DIRTY:  &str = "yellow";
const COLOR_CLEAN:  &str = "green";
const COLOR_FG:     &str = "blue";
const COLOR_BRANCH: &str = "blue";
const COLOR_AHEAD:  &str = "cyan";
const COLOR_BEHIND: &str = "magenta";

const SYMBOL_MOD:   &str = "±";
const SYMBOL_AHEAD: &str = "↑";
const SYMBOL_BEHIND:&str = "↓";
const SYMBOL_FF:    &str = "→";
// @formatter:on

#[derive(Eq, PartialEq, Debug, Clone, Copy, Hash)]
pub enum OutputType {
    All,
    Dir,
    Time,
    Modification,
    Branches,
}

// @formatter:off
impl FromStr for OutputType {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err>{
        let res = match s {
            "dir"                  | "d" => Option::Some(OutputType::Dir),
            "modification" | "mod" | "m" => Option::Some(OutputType::Modification),
            "time"                 | "t" => Option::Some(OutputType::Time),
            "all"                        => Option::Some(OutputType::All),
            _ => Option::None
        };

        match res {
            None => Err(()),
            Some(res) => Ok(res)
        }
    }
}
// @formatter:on

#[derive(Eq, PartialEq, Debug, Clone, Copy)]
pub enum SummaryType {
    Default,
    Verbose,
    VeryVerbose,
}

impl SummaryType {
    pub fn from_occurrences(num: u64) -> Self {
        match num {
            1 => SummaryType::Verbose,
            2 => SummaryType::VeryVerbose,
            _ => SummaryType::Default,
        }
    }
}

impl Default for SummaryType {
    fn default() -> Self {
        SummaryType::Default
    }
}

#[derive(Eq, PartialEq, Debug, Clone, Copy, Deserialize)]
pub enum SortType {
    None,
    Dir,
    Time,
    Mod,
    AheadBehind,
}

// @formatter:off
impl FromStr for SortType {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let sort = match s {
            "modifications" | "mod" | "m" => SortType::Mod,
            "time"                  | "t" => SortType::Time,
            "ahead-behind"  | "ab"  | "a" => SortType::AheadBehind,
            "directory"     | "dir" | "d" => SortType::Dir,
            _                             => SortType::None,
        };
        Result::Ok(sort)
    }
}

// @formatter:on

impl From<&str> for SortType {
    fn from(string: &str) -> Self {
        Self::from_str(string).unwrap()
    }
}

impl From<&String> for SortType {
    fn from(string: &String) -> Self {
        Self::from_str(string.as_str()).unwrap()
    }
}

impl Default for SortType {
    fn default() -> Self {
        SortType::None
    }
}

impl Display for SortType {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.to_string())
    }
}

pub fn print_projects(langs: &Vec<Group>, summary_type: &SummaryType, output_types: &Vec<OutputType>, sort: &SortType) {
    match summary_type {
        SummaryType::VeryVerbose => very_verbose_print(langs),
        SummaryType::Verbose => verbose_print(langs, output_types, sort),
        _ => summary_print(langs, output_types, sort),
    }
}

fn sort_default(_: &Project, _: &Project) -> Ordering {
    Ordering::Equal
}

fn sort_dir(proj_a: &Project, proj_b: &Project) -> Ordering {
    proj_a.path.cmp(&proj_b.path)
}

fn sort_ahead_behind(proj_a: &Project, proj_b: &Project) -> Ordering {
    let sum_changes_a = proj_a.ahead_behind.0 + proj_a.ahead_behind.1;
    let sum_changes_b = proj_b.ahead_behind.0 + proj_b.ahead_behind.1;

    if sum_changes_a != 0 || sum_changes_b != 0 {
        sum_changes_a.partial_cmp(&sum_changes_b).unwrap()
    } else {
        proj_a.modified.partial_cmp(&proj_b.modified).unwrap()
    }
}

fn sort_time(proj_a: &Project, proj_b: &Project) -> Ordering {
    proj_a.time.partial_cmp(&proj_b.time).unwrap()
}

fn sort_modification(proj_a: &Project, proj_b: &Project) -> Ordering {
    if proj_a.modified != 0 || proj_b.modified != 0 {
        proj_a.modified.partial_cmp(&proj_b.modified).unwrap()
    } else {
        let sum_changes_a = proj_a.ahead_behind.0 + proj_a.ahead_behind.1;
        let sum_changes_b = proj_b.ahead_behind.0 + proj_b.ahead_behind.1;

        sum_changes_a.partial_cmp(&sum_changes_b).unwrap()
    }
}

fn filter_stub(_: &&Project) -> bool {
    true
}

fn filter_modification(p: &&Project) -> bool {
    !p.is_clean() || p.fast_forwarded
}

fn print_stub(_: &Project) {}

fn print_branch_stub(_: &Project, _: usize) {}

fn print_default(p: &Project, grp_len: usize, proj_len: usize, branch_len: usize) {
    let color = match p.is_clean() {
        true => COLOR_CLEAN,
        false => COLOR_DIRTY
    };
    let pull_flag = if p.fast_forwarded {
        SYMBOL_FF.color(COLOR_BEHIND)
    } else {
        " ".color(COLOR_BEHIND)
    };
    let p_name = p.name.color(color);
    let g_name = p.grp_name.color(COLOR_FG);
    let branch = format!("{:size$} ", p.current_branch, size = branch_len).color(COLOR_BRANCH);
    print!("{:grp$} {:proj$}{}{}", g_name, p_name, pull_flag, branch, grp = grp_len, proj = proj_len);
}

fn print_modification(p: &Project) {
    let ahead_behind = if p.is_ahead_behind() {
        let ahead = format!("{}{:3}", SYMBOL_AHEAD, p.ahead_behind.0).color(COLOR_AHEAD);
        let behind = format!("{}{:3}", SYMBOL_BEHIND, p.ahead_behind.1).color(COLOR_BEHIND);
        format!("{:4} {:4}", ahead, behind)
    } else {
        String::new()
    };

    let color = match p.modified > 0 {
        true => COLOR_DIRTY,
        false => COLOR_CLEAN,
    };

    print!("{:5} {:9} ", format!("{}{}", SYMBOL_MOD, p.modified).color(color), ahead_behind);
}

fn print_dir(p: &Project, _: usize, _: usize, _: usize) {
    print!("{}", p.path)
}

fn print_extra(p: &Project) {
    let time = p.time.to_string() + "ms";
    print!("{:5}", time.black());
}

fn print_branches(p: &Project, maxlen: usize) {
    for key in p.remote_ahead_behind.keys() {
        // Do not duplicate showing current remote/branch combination twice
        if *key != p.current_branch && *key != format!("origin/{}", p.current_branch) {
            let ahead_behind = p.remote_ahead_behind.get(key).unwrap();
            if ahead_behind.0 > 0 || ahead_behind.1 > 0 {
                let ahead = format!("{}{:3}", SYMBOL_AHEAD, ahead_behind.0).color(COLOR_AHEAD);
                let behind = format!("{}{:3}", SYMBOL_BEHIND, ahead_behind.1).color(COLOR_BEHIND);
                let ahead_behind_str = format!("{:4} {:4}", ahead, behind);
                let branch = format!("{}", key).color(COLOR_BRANCH);
                print!("{:size$} {} ", branch, ahead_behind_str, size = maxlen);
            }
        }
    }
}


fn summary_print(langs: &Vec<Group>, out_types: &Vec<OutputType>, sort: &SortType) {
    let mut print_fn: fn(&Project, usize, usize, usize) = print_default;
    let mut print_modification_fn: fn(&Project) = print_stub;
    let mut print_extra_fn: fn(&Project) = print_stub;
    let mut print_branches_fn: fn(&Project, usize) = print_branch_stub;
    let mut filter: fn(&&Project) -> bool = filter_modification;

    // out_types contain only unique values anyways
    for out_type in out_types {
        match out_type {
            OutputType::All => {
                filter = filter_stub;
            }
            OutputType::Dir => {
                print_fn = print_dir;
                print_modification_fn = print_stub;
                print_extra_fn = print_stub;
                print_branches_fn = print_branch_stub;
            }
            OutputType::Time => {
                if !out_types.contains(&OutputType::Dir) {
                    print_extra_fn = print_extra;
                }
            }
            OutputType::Modification => {
                if !out_types.contains(&OutputType::Dir) {
                    print_modification_fn = print_modification;
                }
            }
            OutputType::Branches => {
                if !out_types.contains(&OutputType::Dir) {
                    print_branches_fn = print_branches;
                }
            }
        }
    }

    let mut projs: Vec<Project> = langs.iter()
        .flat_map(|l| l.projs.to_vec())
        .filter(|p| filter(&p))
        .collect();

    let mut grp_maxlen = 0;
    let mut proj_maxlen = 0;
    let mut branch_maxlen = 0;
    for proj in &projs {
        if proj.grp_name.len() > grp_maxlen {
            grp_maxlen = proj.grp_name.len();
        }

        if proj.name.len() > proj_maxlen {
            proj_maxlen = proj.name.len();
        }

        if proj.current_branch.len() > branch_maxlen {
            branch_maxlen = proj.current_branch.len();
        }

        for branch in &proj.branches {
            if branch.len() > branch_maxlen {
                let ahead_behind = proj.remote_ahead_behind.get(branch.as_str());
                if ahead_behind.is_some(){
                    let ahead_behind = ahead_behind.unwrap();
                    if ahead_behind.0 > 0 || ahead_behind.1 > 0 {
                        branch_maxlen = branch.len();
                    }
                }
            }
        }
    }


    if *sort != SortType::None { // @formatter:off
        let sort_fn: fn(&Project, &Project) -> Ordering = match sort {
            SortType::Dir         => sort_dir,
            SortType::Time        => sort_time,
            SortType::Mod         => sort_modification,
            SortType::AheadBehind => sort_ahead_behind,
            _                     => sort_default,
        };
        projs.sort_by(sort_fn);
    } // @formatter:on

    for p in &projs {
        print_fn(p, grp_maxlen, proj_maxlen, branch_maxlen);
        print_modification_fn(p);
        print_extra_fn(p);
        print_branches_fn(p, branch_maxlen);
        print!("\n");
    }
}


fn verbose_print(langs: &Vec<Group>, out_types: &Vec<OutputType>, sort: &SortType) {
    for l in langs {
        if l.projs.len() > 0 {
            let time = out_types.iter().find(|o| o == &&OutputType::Time).is_some();
            let g_name = &l.name;
            let g_projs = l.projs.len().to_string();
            let time: String = if time {
                l.projs.iter().map(|p| p.time).max().unwrap().to_string() + "ms"
            } else {
                String::new()
            };
            println!("{:8} {:4} {:6} {}", g_name, g_projs.color(COLOR_CLEAN), time.color("black"), l.path.color("white"));
        }
    }
    summary_print(langs, out_types, sort);
}

fn very_verbose_print(langs: &Vec<Group>) {
    for (i, l) in langs.iter().enumerate() {
        if i == langs.len() - 1 {
            println!("└──{} {}", l.name.color(COLOR_FG), format!("({})", l.projs.len()).black());
        } else {
            println!("├──{} {}", l.name.color(COLOR_FG), format!("({})", l.projs.len()).black());
        }
        for (j, p) in &mut l.projs.iter().enumerate() {
            if i == langs.len() - 1 {
                print!("   ");
            } else {
                print!("|  ");
            }
            if j == l.projs.len() - 1 {
                print!("└──");
            } else {
                print!("├──");
            }
            if p.is_clean() {
                print!("{}", p.name.color(COLOR_CLEAN));
            } else {
                print!("{}  *", p.name.color(COLOR_DIRTY).bold());
            }
            println!();
        }
    }
}