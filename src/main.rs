#![allow(unused_must_use)]

use std::{io, thread};
use std::env;
use std::fs;
use std::path::Path;
use std::process;
use std::sync::mpsc;
use std::thread::JoinHandle;
use std::env::join_paths;

struct Proj {
    name: String,
    path: String,
    is_ok: bool,
}

struct Lang {
    name: String,
    path: String,
    projs: Vec<Proj>,
    not_ok: i32,
}

enum PrintType {
    LongPrint,
    LongLongPrint,
    ShortPrint,
}

impl Proj {
    fn new(name: &str, path: &str) -> Self {
        Proj {
            name: String::from(name),
            path: String::from(path),
            is_ok: true,
        }
    }
}

impl Lang {
    fn new(name: &str, path: &str) -> Self {
        Lang {
            name: String::from(name),
            path: String::from(path),
            projs: vec![],
            not_ok: 0,
        }
    }
    fn add_proj(&mut self, proj: Proj) {
        self.projs.push(proj)
    }
}


fn main() {
    let mut ptype = PrintType::ShortPrint;
    let mut dir_print = false;

    let mut codeignore = Vec::new();
    let mut langs = Vec::new();
    let mut count = 0;
    let code = match env::var("CODE") {
        Ok(val) => val,
        Err(_) => {
            eprintln!("ERROR: Cannot find 'CODE' environmental variable");
            process::exit(1);
        }
    };


    let contents = fs::read_to_string(Path::new(&code).join(".codeignore")).unwrap_or("".to_string());

    for line in contents.split("\n") {
        let line = String::from(line);
        if !line.starts_with("#") {
            codeignore.push(line);
        }
    }


    let args: Vec<String> = env::args().collect();

    match args.len() {
        1 => {}
        _ => {
            for arg in args {
                match arg.as_str() {
                    "-l" => ptype = PrintType::LongPrint,
                    "-ll" => ptype = PrintType::LongLongPrint,
                    "-d" => dir_print = true,
                    _ => {}
                }
            }
        }
    }


    match list_dir(code.as_str(), 2, &mut count, &mut langs, &codeignore) {
        Ok(_) => {
            update_langs(&mut langs);
            langs.sort_by(|a, b| a.name.cmp(&b.name));
            match ptype {
                PrintType::LongPrint => long_print(&langs),
                PrintType::LongLongPrint => long_long_print(&langs),
                _ => short_print(&langs, dir_print),
            }
        }
        Err(err) => { eprintln!("ERROR: {}", err.to_string()) }
    }
}

fn short_print(langs: &Vec<Lang>, dir_print: bool) {
    for l in langs {
        for p in &l.projs {
            if !p.is_ok {
                if dir_print {
                    println!("{:32}", p.path);
                } else {
                    println!("{:16} {:16}", l.name, p.name);
                }
            }
        }
    }
}

fn long_print(langs: &Vec<Lang>) {
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

fn long_long_print(langs: &Vec<Lang>) {
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
            proj.is_ok = git_is_ok(&proj.path);
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

fn git_is_ok(path: &str) -> bool {
    return match process::Command::new("git")
        .arg("-C")
        .arg(path)
        .arg("status")
        .arg("--porcelain")
        .output() {
        Ok(out) => out.stdout.len() == 0,
        Err(_) => false
    };
}

fn is_git_repo(path: &Path) -> bool {
    let entries = fs::read_dir(path);
    return match entries {
        Ok(dir) => {
            dir.into_iter().any(|x| {
                match x {
                    Ok(e) => e.file_name() == ".git" && e.path().is_dir(),
                    Err(_) => false,
                }
            })
        }
        Err(_) => false
    };
}

fn list_dir(path: &str, mut depth: i32, count: &mut i32, langs: &mut Vec<Lang>, codeignore: &Vec<String>) -> io::Result<()> {
    if depth == 0 {
        return Ok(());
    } else {
        depth -= 1;
    }
    for entry in fs::read_dir(path)? {
        let entry = entry?;
        let path = entry.path();
        let pstr = path.to_str().unwrap();
        if path.is_dir() {
            let nstr = path.file_name().unwrap().to_str().unwrap();
            let parstr = path.parent().unwrap().to_str().unwrap();
            if !codeignore.contains(&nstr.to_string()) {
                if is_git_repo(&path) {
                    *count += 1;
                    match langs.pop() {
                        None => {
                            let mut oddl = Lang::new(nstr, pstr);
                            oddl.add_proj(Proj::new(nstr, pstr));
                            langs.push(oddl);
                        }
                        Some(mut lang) => {
                            if parstr.starts_with(&lang.path) {
                                lang.add_proj(Proj::new(nstr, pstr));
                                langs.push(lang);
                            } else {
                                langs.push(lang);
                                let mut oddl = Lang::new(nstr, pstr);
                                oddl.add_proj(Proj::new(nstr, pstr));
                                langs.push(oddl);
                            }
                        }
                    }
                } else {
                    if !nstr.starts_with(".") && !nstr.starts_with("_") {
                        langs.push(Lang::new(nstr, pstr));
                        list_dir(pstr, depth, count, langs, codeignore)?;
                    }
                }
            }
        }
    };
    Ok(())
}

