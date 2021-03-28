#![allow(unused_must_use)]
mod git;
mod lang;

use std::{io, thread};
use std::env;
use std::fs;
use std::path::Path;
use std::process;
use std::sync::mpsc;
use std::thread::JoinHandle;
use crate::git::{git_is_dirty, git_is_inside_work_tree};
use crate::lang::{Lang, Proj};
use std::io::BufRead;
use glob::Pattern;
use std::fs::File;
use getopts::Options;
use colored::*;


#[derive(Eq, PartialEq)]
enum OutputType {
    All,
    Dir,
}

#[derive(Eq, PartialEq)]
enum SummaryType {
    Default,
    Verbose,
    VeryVerbose,
}

fn main() {
    let mut out_types: Vec<OutputType> = vec![];
    let mut langs = Vec::new();
    let mut count = 0;

    let args: Vec<String> = env::args().collect();
    let program = args[0].clone();
    let program = Path::new(program.as_str())
        .file_stem()
        .unwrap()
        .to_str()
        .unwrap();

    let code = match env::var("CODE") {
        Ok(val) => val,
        Err(_) => {
            eprintln!("{}: 'CODE' env not set", program);
            process::exit(1);
        }
    };

    let mut opts = Options::new();
    opts.optflag("h", "help", "show help");
    opts.optflagmulti("v", "verbose", "verbose");
    opts.optflag("a", "all", "show all repositories");
    opts.optflag("d", "dir", "show all repository directories");
    opts.optflag("i", "no-ignore", "doesn't read .codeignore file");
    opts.optopt("D", "depth", "project search recursive depth", "DEPTH");

    let matches = match opts.parse(&args[1..]) {
        Ok(m) => m,
        Err(f) => panic!(f.to_string())
    };

    let verbose = matches.opt_count("verbose");
    let summary_type = match verbose {
        1 =>  SummaryType::Verbose,
        2 =>  SummaryType::VeryVerbose,
        _ =>  SummaryType::Default,
    };

    if matches.opt_present("all") {
        out_types.push(OutputType::All)
    }

    if matches.opt_present("dir") {
        out_types.push(OutputType::Dir);
    }

    let no_codeignore = matches.opt_present("no-ignore");

    let depth = matches.opt_str("depth").unwrap_or(String::from("2"));
    let depth = depth.parse::<i32>().unwrap_or(2);


    let codeignore: Vec<Pattern>;
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
            Err(_) => codeignore = vec![]
        };
    } else {
        codeignore = vec![]
    }

    match list_dir(code.as_str(), depth, &mut count, &mut langs, &codeignore, &code) {
        Ok(_) => {
            update_langs(&mut langs);
            langs.sort_by(|a, b| a.name.cmp(&b.name));

            match summary_type {
                SummaryType::VeryVerbose => very_verbose_print(&langs),
                SummaryType::Verbose => verbose_print(&langs),
                _ => default_print(&langs, &out_types),
            }
        }

        Err(err) => { eprintln!("{}: error: {}", program, err.to_string()) }
    }
}

fn default_print(langs: &Vec<Lang>, out_types: &Vec<OutputType>) {
    let mut print: Box<fn(&Lang, &Proj)> = Box::new(|l: &Lang, p: &Proj| println!("{:16} {:16}", l.name.magenta(), p.name.yellow()));
    let mut filter: Box<fn(&&Proj) -> bool> = Box::new(|p: &&Proj| !p.is_ok);
    for out_type in out_types {
        match out_type {
            OutputType::All => {
                filter = Box::new(|_p: &&Proj| true);
            }
            OutputType::Dir => {
                print = Box::new(|_l: &Lang, p: &Proj| println!("{}", p.path));
            }
        }
    }

    for l in langs {
        for p in l.projs.iter().filter(filter.as_ref()) {
            print(l, p);
        }
    }
}


fn verbose_print(langs: &Vec<Lang>) {
    let mut summary = String::from("\n");

    for l in langs {
        if l.projs.len() > 0 {
            println!("{:8} {:4} {:2} {}", l.name.magenta(), l.projs.len().to_string().green(), if l.not_ok > 0 { l.not_ok.to_string().red().bold() } else { "".to_string().white() }, l.path.yellow());
            for p in &l.projs {
                if !p.is_ok {
                    summary += format!("{:16} {:16}\n", l.name.magenta(), p.name.yellow()).as_str();
                }
            }
        }
    }
    print!("{}", summary);
}

fn very_verbose_print(langs: &Vec<Lang>) {
    for (i, l) in &mut langs.iter().enumerate() {
        if i == langs.len() - 1 {
            println!("└──{} {}", l.name.magenta(), format!("({})", l.projs.len()).black());
        } else {
            println!("├──{} {}", l.name.magenta(), format!("({})", l.projs.len()).black());
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
                out += format!("{}", p.name.green()).as_str();
            } else {
                out += format!("{}  *", p.name.red().bold()).as_str();
            }
            println!("{}", out);
        }
    }
}


fn update_projs(lang: &mut Lang) {
    let (txp, rxp) = mpsc::channel();
    let mut phandles: Vec<JoinHandle<()>> = Vec::new();

    while !lang.projs.is_empty() {
        let txp_local = mpsc::Sender::clone(&txp);
        let mut proj = lang.projs.pop().unwrap();
        let phandle = thread::spawn(move || {
            proj.is_ok = git_is_dirty(&proj.path);
            txp_local.send(proj);
        });
        phandles.push(phandle);
    }
    for phandle in phandles {
        phandle.join();
    }
    drop(txp);
    for proj in rxp {
        if !proj.is_ok {
            lang.not_ok += 1;
        }
        lang.projs.push(proj);
    }
}

fn update_langs(langs: &mut Vec<Lang>) {
    let (tx, rx) = mpsc::channel();
    let mut handles: Vec<JoinHandle<()>> = Vec::new();
    while !langs.is_empty() {
        let tx_local = mpsc::Sender::clone(&tx);
        let mut l = langs.pop().unwrap();
        let handle = thread::spawn(move || {
            update_projs(&mut l);
            tx_local.send(l).unwrap();
        });
        handles.push(handle);
    }
    for handle in handles {
        let _ = handle.join();
    }
    drop(tx);
    for lang in rx {
        langs.push(lang);
    }
}


fn list_dir(path: &str, mut depth: i32, count: &mut i32, langs: &mut Vec<Lang>, codeignore: &Vec<Pattern>, code: &String) -> io::Result<()> {
    if depth == 0 { return Ok(()); }
    depth -= 1;

    for entry in fs::read_dir(path)? {
        let path = entry?.path();
        let path_str = path.to_str().unwrap();
        let replaced = path_str.replace(code, "");
        let path_root = replaced.as_str();

        if codeignore.iter().any(|g| g.matches(path_root)) {
            continue;
        }

        if path.is_dir() {
            let dir_name = path.file_name().unwrap().to_str().unwrap();
            let par_name = path.parent().unwrap().to_str().unwrap();


            if git_is_inside_work_tree(&path_str) {
                *count += 1;

                // last or new
                let mut lang = langs.pop().unwrap_or(Lang::new(dir_name, path_str));
                if !par_name.ends_with(&lang.name) {
                    langs.push(lang);
                    lang = Lang::new(dir_name, path_str);
                }
                lang.add_proj(Proj::new(dir_name, path_str));
                langs.push(lang);
            } else {
                // if its a top-level repository (eg. uni)
                if code.as_str() == path.parent().unwrap().to_path_buf().to_str().unwrap() {
                    langs.push(Lang::new(dir_name, path_str));
                }
                list_dir(path_str, depth, count, langs, codeignore, code)?;
            }
        }
    };
    Ok(())
}

