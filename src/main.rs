#![allow(unused_must_use)]

mod git;
mod lang;
mod rgs;
mod print;

use std::env;
use std::path::Path;
use std::process;
use crate::rgs::Rgs;
use getopts::Options;
use crate::print::{SummaryType, OutputType};
use std::time::Instant;
use colored::*;

fn usage(program: &str, opts: &Options) {
    let brief = format!("Usage: {} [options]", program);
    print!("{}", opts.usage(&brief));
    process::exit(0);
}

fn main() {
    let now = Instant::now();
    let mut out_types: Vec<OutputType> = vec![];

    let args: Vec<String> = env::args().collect();
    let program = args[0].clone();
    let program = Path::new(program.as_str())
        .file_stem()
        .unwrap()
        .to_str()
        .unwrap();

    let mut code = match env::var("CODE") {
        Ok(val) => val,
        Err(_) => {
            eprintln!("{}: 'CODE' env not set", program);
            process::exit(1);
        }
    };

    let mut opts = Options::new();
    opts.optflag("h", "help", "show this message and exit");
    opts.optflagmulti("v", "verbose", "verbose");
    opts.optflag("a", "all", "show all repositories");
    opts.optflag("d", "dir", "show all repository directories");
    opts.optflag("i", "no-ignore", "doesn't read .codeignore file");
    opts.optopt("D", "depth", "project search recursive depth", "NUM");
    opts.optopt("c", "code", "set CODE variable", "PATH");
    opts.optflag("C", "print-code", "print CODE variable and exit");
    opts.optflag("f", "fetch", "also fetch from origin");
    opts.optflag("t", "time", "show time elapsed per project");
    opts.optflag("m", "modification", "show modifications or ahead/behind status");

    let matches = match opts.parse(&args[1..]) {
        Ok(m) => m,
        Err(f) => {
            eprintln!("{}: {}", program, f.to_string());
            process::exit(2);
        }
    };

    let verbose = matches.opt_count("verbose");
    let summary_type = match verbose {
        1 => SummaryType::Verbose,
        2 => SummaryType::VeryVerbose,
        _ => SummaryType::Default,
    };

    if matches.opt_present("code") {
        code = matches.opt_get("code")
            .unwrap()
            .unwrap_or(code);
    }

    if matches.opt_present("print-code") {
        println!("{}", code);
        process::exit(0);
    }

    if matches.opt_present("help") {
        usage(program, &opts);
    }

    if matches.opt_present("all") {
        out_types.push(OutputType::All)
    }

    if matches.opt_present("dir") {
        out_types.push(OutputType::Dir);
    }

    if matches.opt_present("time") {
        out_types.push(OutputType::Time);
    }

    if matches.opt_present("modification") {
        out_types.push(OutputType::Modification);
    }

    let no_codeignore = matches.opt_present("no-ignore");
    let fetch = matches.opt_present("fetch");

    let depth = matches.opt_str("depth").unwrap_or(String::from("2"));
    let depth = depth.parse::<i32>().unwrap_or(2);

    let mut rgs = Rgs::new(code, no_codeignore, fetch, out_types, summary_type, depth);
    rgs.run();
    let time = now.elapsed();
    if matches.opt_present("time") {
        eprintln!("{}", format!("{}ms", time.as_millis()).black());
    }
}


