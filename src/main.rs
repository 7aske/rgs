use std::{io, thread};
use std::env;
use std::fs;
use std::path::Path;
use std::process;
use std::sync::mpsc;
use std::thread::JoinHandle;
use std::time::Duration;

struct Proj {
    name: String,
    path: String,
    is_ok: bool,
}

struct Lang {
    name: String,
    path: String,
    projs: Vec<Proj>,
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
        }
    }
    fn add_proj(&mut self, proj: Proj) {
        self.projs.push(proj)
    }
}


fn main() {
    let mut color = true;
    let mut ptype = PrintType::ShortPrint;
    let mut dir_print = false;

    let mut langs = Vec::new();
    let mut count = 0;
    let code = match env::var("CODE") {
        Ok(val) => val,
        Err(_) => {
            eprintln!("ERROR: Cannot find 'CODE' environmental variable");
            process::exit(1);
        }
    };
    let args: Vec<String> = env::args().collect();

    match args.len() {
        1 => {}
        _ => {
            for arg in args {
                match arg.as_str() {
                    "-l" => ptype = PrintType::LongPrint,
                    "-ll" => ptype = PrintType::LongLongPrint,
                    "--no-color" => color = false,
                    "-d" => dir_print = true,
                    _ => {}
                }
            }
        }
    }


    list_dir(code.as_str(), 2, &mut count, &mut langs).expect("ERROR: Failed reading 'CODE' directory.");

    update_status(&mut langs);

    match ptype {
        PrintType::LongPrint => long_print(&langs),
        PrintType::LongLongPrint => long_long_print(&langs),
        _ => short_print(&langs, dir_print),
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
    for l in langs {
        println!("{:16} {:16}", l.name, l.projs.len());
        for p in &l.projs {
            if !p.is_ok {
                println!("{:16} {:16}", l.name, p.name);
            }
        }
    }
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

fn update_status(langs: &mut Vec<Lang>) {
    let (tx, rx) = mpsc::channel();
    let mut handles: Vec<JoinHandle<()>> = Vec::new();
    while !langs.is_empty() {
        let tx_local = mpsc::Sender::clone(&tx);
        let mut l = langs.pop().unwrap();
        let handle = thread::spawn(move || {
            for p in &mut l.projs {
                check_status(p);
            }
            tx_local.send(l).unwrap();
        });
        handles.push(handle);
    }
    for handle in handles {
        handle.join();
    }
    drop(tx);
    for lang in rx {
        langs.push(lang);
    }
}

fn check_status(proj: &mut Proj) -> bool {
    return match process::Command::new("git")
        .arg("-C")
        .arg((*proj).path.as_str())
        .arg("status")
        .arg("--porcelain")
        .output() {
        Ok(out) => {
            if out.stdout.len() == 0 {
                true
            } else {
                proj.is_ok = false;
                false
            }
        }
        Err(_) => {
            proj.is_ok = false;
            false
        }
    };
}

fn is_git_repo(path: &Path) -> bool {
    let entries = fs::read_dir(path);
    return match entries {
        Ok(dir) => {
            dir.into_iter().any(|x| {
                match x {
                    Ok(e) => e.file_name() == ".git",
                    Err(_) => false,
                }
            })
        }
        Err(_) => false
    };
}

fn list_dir(path: &str, mut depth: i32, count: &mut i32, langs: &mut Vec<Lang>) -> io::Result<()> {
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
            if is_git_repo(&path) {
                let parstr = path.parent().unwrap().to_str().unwrap();
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
                    list_dir(pstr, depth, count, langs)?;
                }
            }
        }
    };
    Ok(())
}

