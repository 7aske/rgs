use crate::lang::{Project, Group};
use colored::*;

#[derive(Eq, PartialEq)]
pub enum OutputType {
    All,
    Dir,
    Time,
    Modification,
}

#[derive(Eq, PartialEq)]
pub enum SummaryType {
    Default,
    Verbose,
    VeryVerbose,
}


const COLOR_DIRTY: &str = "yellow";
const COLOR_CLEAN: &str = "green";
const COLOR_FG: &str = "blue";
const COLOR_AHEAD: &str = "cyan";
const COLOR_BEHIND: &str = "magenta";

pub fn print_groups(langs: &Vec<Group>, summary_type: &SummaryType, output_types: &Vec<OutputType>) {
    match summary_type {
        SummaryType::VeryVerbose => very_verbose_print(langs),
        SummaryType::Verbose => verbose_print(langs, output_types),
        _ => default_print(langs, output_types),
    }
}

fn default_print(langs: &Vec<Group>, out_types: &Vec<OutputType>) {
    let mut print: Box<fn(&Group, &Project)> = Box::new(|g: &Group, p: &Project| {
        let color = match p.is_clean() {
            true => "green",
            false => "yellow"
        };
        let mut p_name = p.name.clone();
        p_name.truncate(24);
        let mut g_name = g.name.clone();
        g_name.truncate(16);
        print!("{:16} {:24} ", g_name.color(COLOR_FG), p_name.color(color));
    });

    let mut print_modified: Box<fn(&Project)> = Box::new(|_p: &Project| { print!("{:16}", ""); });
    let mut print_extra: Box<fn(&Project)> = Box::new(|_p| { print!(""); });

    let mut filter: Box<fn(&&Project) -> bool> = Box::new(|p: &&Project| p.modified > 0 || p.ahead_behind.0 > 0 || p.ahead_behind.1 > 0);
    for out_type in out_types {
        match out_type {
            OutputType::All => {
                filter = Box::new(|_p: &&Project| true);
            }
            OutputType::Dir => {
                print = Box::new(|_l: &Group, p: &Project| print!("{}", p.path));
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

    for l in langs {
        for p in l.projs.iter().filter(filter.as_ref()) {
            print(l, p);
            print_modified(p);
            print_extra(p);
            print!("\n");
        }
    }
}


fn verbose_print(langs: &Vec<Group>, out_types: &Vec<OutputType>) {
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
    default_print(langs, out_types);
}

fn very_verbose_print(langs: &Vec<Group>) {
    for (i, l) in langs.iter().enumerate() {
        if i == langs.len() - 1 {
            println!("└──{} {}", l.name.color(COLOR_FG), format!("({})", l.projs.len()).black());
        } else {
            println!("├──{} {}", l.name.color(COLOR_FG), format!("({})", l.projs.len()).black());
        }
        for (j, p) in &mut l.projs.iter().enumerate() {
            let mut out = String::from("");

            if i == langs.len() - 1 {
                out += "   "
            } else {
                out += "|  "
            }
            if j == l.projs.len() - 1 {
                out += "└──"
            } else {
                out += "├──"
            }
            if p.modified > 0 {
                out += format!("{}", p.name.color(COLOR_CLEAN)).as_str();
            } else {
                out += format!("{}  *", p.name.color(COLOR_DIRTY).bold()).as_str();
            }
            println!("{}", out);
        }
    }
}

pub fn print_progress(total: i32, current: i32) {
    let progress = format!("{}/{}", total, total - current).black();
    println!("{}", progress);
}