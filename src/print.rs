use crate::lang::{Project, Group};
use colored::*;

#[derive(Eq, PartialEq)]
pub enum OutputType {
    All,
    Dir,
}

#[derive(Eq, PartialEq)]
pub enum SummaryType {
    Default,
    Verbose,
    VeryVerbose,
}


const DIRTY_COLOR: &str = "yellow";
const OK_COLOR: &str = "green";
const FG_COLOR: &str = "blue";

pub fn print_groups(langs: &Vec<Group>, summary_type: &SummaryType, output_types: &Vec<OutputType>) {
    match summary_type {
        SummaryType::VeryVerbose => very_verbose_print(langs),
        SummaryType::Verbose => verbose_print(langs),
        _ => default_print(langs, output_types),
    }
}

fn default_print(langs: &Vec<Group>, out_types: &Vec<OutputType>) {
    let mut print: Box<fn(&Group, &Project)> = Box::new(|l: &Group, p: &Project| {
        let color = match p.is_ok {
            true => "green",
            false => "yellow"
        };
        println!("{:16} {:16}", l.name.color(FG_COLOR), p.name.color(color))
    });

    let mut filter: Box<fn(&&Project) -> bool> = Box::new(|p: &&Project| !p.is_ok);
    for out_type in out_types {
        match out_type {
            OutputType::All => {
                filter = Box::new(|_p: &&Project| true);
            }
            OutputType::Dir => {
                print = Box::new(|_l: &Group, p: &Project| println!("{}", p.path));
            }
        }
    }

    for l in langs {
        for p in l.projs.iter().filter(filter.as_ref()) {
            print(&l, p);
        }
    }
}


fn verbose_print(langs: &Vec<Group>) {
    let mut summary = String::from("\n");

    for l in langs {
        if l.projs.len() > 0 {
            println!("{:8} {:4} {:2} {}", l.name.color(FG_COLOR), l.projs.len().to_string().color(OK_COLOR), if l.not_ok > 0 { l.not_ok.to_string().color(DIRTY_COLOR).bold() } else { "".to_string().white() }, l.path.color("white"));
            for p in &l.projs {
                if !p.is_ok {
                    summary += format!("{:16} {:16}\n", l.name.color(FG_COLOR), p.name.color(DIRTY_COLOR)).as_str();
                }
            }
        }
    }
    print!("{}", summary);
}

fn very_verbose_print(langs: &Vec<Group>) {
    for (i, l) in langs.iter().enumerate() {
        if i == langs.len() - 1 {
            println!("└──{} {}", l.name.color(FG_COLOR), format!("({})", l.projs.len()).black());
        } else {
            println!("├──{} {}", l.name.color(FG_COLOR), format!("({})", l.projs.len()).black());
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
            if p.is_ok {
                out += format!("{}", p.name.color(OK_COLOR)).as_str();
            } else {
                out += format!("{}  *", p.name.color(DIRTY_COLOR).bold()).as_str();
            }
            println!("{}", out);
        }
    }
}
