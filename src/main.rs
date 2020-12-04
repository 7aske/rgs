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


enum OutputType {
    Verbose,
    VeryVerbose,
    Short,
    All,
    Dir,
}

fn main() {
    let mut out_type = OutputType::Short;
    let mut no_codeignore = false;
    let mut langs = Vec::new();
    let mut count = 0;
    let code = match env::var("CODE") {
        Ok(val) => val,
        Err(_) => {
            eprintln!("rgs: 'CODE' env not set");
            process::exit(1);
        }
    };

    let args: Vec<String> = env::args().collect();

    for arg in args {
        match arg.as_str() {
            "-v" => out_type = OutputType::Verbose,
            "-vv" => out_type = OutputType::VeryVerbose,
            "-d" => out_type = OutputType::Dir,
            "-a" => out_type = OutputType::All,
            "-i" => no_codeignore = true,
            _ => {}
        }
    }


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

    match list_dir(code.as_str(), 2, &mut count, &mut langs, &codeignore, &code) {
        Ok(_) => {
            update_langs(&mut langs);
            langs.sort_by(|a, b| a.name.cmp(&b.name));
            match out_type {
                OutputType::Verbose => verbose_print(&langs),
                OutputType::VeryVerbose => very_verbose_print(&langs),
                OutputType::All => all_print(&langs),
                OutputType::Dir => dir_print(&langs),
                _ => default_print(&langs),
            }
        }
        Err(err) => { eprintln!("ERROR: {}", err.to_string()) }
    }
}

fn all_print(langs: &Vec<Lang>) {
    for l in langs {
        for p in &l.projs {
            println!("{}", p.path);
        }
    }
}

fn dir_print(langs: &Vec<Lang>) {
    for l in langs {
        for p in l.projs.iter().filter(|p| !p.is_ok) {
            println!("{}", p.path);
        }
    }
}

fn default_print(langs: &Vec<Lang>) {
    for l in langs {
        for p in l.projs.iter().filter(|p| !p.is_ok) {
            println!("{:16} {:16}", l.name, p.name)
        }
    }
}


fn verbose_print(langs: &Vec<Lang>) {
    let mut summary = String::from("\n");

    for l in langs {
        if l.projs.len() > 0 {
            println!("{:8} {:4} {:2} {}", l.name, l.projs.len(), if l.not_ok > 0 { l.not_ok.to_string() } else { "".to_string() }, l.path);
            for p in &l.projs {
                if !p.is_ok {
                    summary += format!("{:16} {:16}\n", l.name, p.name).as_str();
                }
            }
        }
    }
    print!("{}", summary);
}

fn very_verbose_print(langs: &Vec<Lang>) {
    for (i, l) in &mut langs.iter().enumerate() {
        if i == langs.len() - 1 {
            println!("└──{} ({})", l.name, l.projs.len());
        } else {
            println!("├──{} ({})", l.name, l.projs.len());
        }
        for (j, p) in &mut l.projs.iter().enumerate() {
            if i == langs.len() - 1 {
                if j == l.projs.len() - 1 {
                    if p.is_ok {
                        println!("   └──{}", p.name);
                    } else {
                        println!("   └──{}  *", p.name);
                    }
                } else if p.is_ok {
                    println!("   ├──{}", p.name);
                } else {
                    println!("   ├──{}  *", p.name);
                }
            } else {
                if j == l.projs.len() - 1 {
                    if p.is_ok {
                        println!("│  └──{}", p.name);
                    } else {
                        println!("│  └──{}  *", p.name);
                    }
                } else {
                    if p.is_ok {
                        println!("│  ├──{}", p.name);
                    } else {
                        println!("│  ├──{}  *", p.name);
                    }
                }
            }
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

