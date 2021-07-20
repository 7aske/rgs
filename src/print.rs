use colored::*;
use std::cmp::Ordering;
use std::str::FromStr;

use crate::lang::{Project, Group};
use std::fmt::{Display, Formatter};
use serde_derive::Deserialize;

#[derive(Eq, PartialEq, Debug, Clone, Copy, Hash)]
pub enum OutputType {
    All,
    Dir,
    Time,
    Modification,
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

// @formatter:off
const COLOR_DIRTY:  &str = "yellow";
const COLOR_CLEAN:  &str = "green";
const COLOR_FG:     &str = "blue";
const COLOR_AHEAD:  &str = "cyan";
const COLOR_BEHIND: &str = "magenta";
// @formatter:on

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

fn sort_modifications(proj_a: &Project, proj_b: &Project) -> Ordering {
    if proj_a.modified != 0 || proj_b.modified != 0 {
        proj_a.modified.partial_cmp(&proj_b.modified).unwrap()
    } else {
        let sum_changes_a = proj_a.ahead_behind.0 + proj_a.ahead_behind.1;
        let sum_changes_b = proj_b.ahead_behind.0 + proj_b.ahead_behind.1;

        sum_changes_a.partial_cmp(&sum_changes_b).unwrap()
    }
}

pub fn print_groups(langs: &Vec<Group>, summary_type: &SummaryType, output_types: &Vec<OutputType>, sort: &SortType) {
    match summary_type {
        SummaryType::VeryVerbose => very_verbose_print(langs),
        SummaryType::Verbose => verbose_print(langs, output_types, sort),
        _ => default_print(langs, output_types, sort),
    }
}

fn default_print(langs: &Vec<Group>, out_types: &Vec<OutputType>, sort: &SortType) {
    let mut print: Box<fn(&Project, usize, usize)> = Box::new(|p: &Project, grp_len, proj_len| {
        let color = match p.is_clean() {
            true => COLOR_CLEAN,
            false => COLOR_DIRTY
        };
        let pull_flag = if p.fast_forwarded {
            "*".color(COLOR_DIRTY)
        } else {
            " ".color(COLOR_DIRTY)
        };
        let p_name = p.name.color(color);
        let g_name = p.grp_name.color(COLOR_FG);
        print!("{:grp$} {:proj$}{} ", g_name, p_name, pull_flag, grp = grp_len, proj = proj_len);
    });

    let mut print_modified: Box<fn(&Project)> = Box::new(|_p: &Project| { print!(""); });
    let mut print_extra: Box<fn(&Project)> = Box::new(|_p| { print!(""); });

    let mut filter: Box<fn(&&Project) -> bool> = Box::new(|p: &&Project| p.modified > 0 || p.ahead_behind.0 > 0 || p.ahead_behind.1 > 0 || p.fast_forwarded);
    for out_type in out_types {
        match out_type {
            OutputType::All => {
                filter = Box::new(|_p: &&Project| true);
            }
            OutputType::Dir => {
                print = Box::new(|_p: &Project, _, _| print!("{}", _p.path));
                print_modified = Box::new(|_p: &Project| { print!(""); });
            }
            OutputType::Time => {
                print_extra = Box::new(|p| {
                    let time = p.time.to_string() + "ms";
                    print!("{}", time.black());
                });
            }
            OutputType::Modification => {
                print_modified = Box::new(|p: &Project| {
                    let ahead_behind = if p.is_ahead_behind() {
                        let ahead = format!("↑{:3}", p.ahead_behind.0).color(COLOR_AHEAD);
                        let behind = format!("↓{:3}", p.ahead_behind.1).color(COLOR_BEHIND);
                        format!("{:4} {:4}", ahead, behind)
                    } else {
                        String::new()
                    };

                    let color = match p.modified > 0 {
                        true => COLOR_DIRTY,
                        false => COLOR_CLEAN,
                    };

                    print!("{:5} {:9} ", format!("±{}", p.modified).color(color), ahead_behind);
                });
            }
        }
    }

    let mut projs: Vec<Project> = langs.iter().flat_map(|l| l.projs.to_vec()).collect();

    let mut grp_maxlen = 0;
    let mut proj_maxlen = 0;
    for proj in &projs {
        // do not factor in projects that are not going to be shown
        if !out_types.contains(&OutputType::All) && proj.is_clean() {
            continue
        }

        if proj.grp_name.len() > grp_maxlen {
            grp_maxlen = proj.grp_name.len();
        }

        if proj.name.len() > proj_maxlen {
            proj_maxlen = proj.name.len();
        }
    }


    if *sort != SortType::None { // @formatter:off
        let sort_fn: fn(&Project, &Project) -> Ordering = match sort {
            SortType::Dir         => sort_dir,
            SortType::Time        => sort_time,
            SortType::Mod         => sort_modifications,
            SortType::AheadBehind => sort_ahead_behind,
            _                     => sort_default,
        };
        projs.sort_by(sort_fn);
    } // @formatter:on

    for p in projs.iter().filter(filter.as_ref()) {
        print(p, grp_maxlen, proj_maxlen);
        print_modified(p);
        print_extra(p);
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
    default_print(langs, out_types, sort);
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