#![allow(unused_must_use)]

mod git;
mod lang;
mod rgs;
mod print;

use colored::*;
use crate::rgs::{Rgs, RgsOpt};
use getopts::Options;
use std::{env};
use std::path::Path;
use std::process;
use std::time::Instant;
use std::convert::TryFrom;

#[macro_use]
extern crate savefile_derive;

fn usage(program: &str, opts: &Options) -> ! {
    let brief = format!("Usage: {} [options]", program);
    print!("{}", opts.usage(&brief));
    process::exit(0);
}

fn main() {
    let now = Instant::now();

    let args: Vec<String> = env::args().collect();
    let program = args[0].clone();
    let program = Path::new(program.as_str())
        .file_stem()
        .unwrap()
        .to_str()
        .unwrap();

    let mut opts = Options::new();
    opts.optflag("h", "help", "show this message and exit");
    opts.optflagmulti("v", "verbose", "verbose");
    opts.optflag("a", "all", "show all repositories");
    opts.optflag("d", "dir", "show all repository directories (turns off -t and -m flags)");
    opts.optflag("i", "no-ignore", "doesn't read .codeignore file");
    opts.optopt("D", "depth", "project search recursive depth", "NUM");
    opts.optopt("c", "code", "set CODE variable", "PATH");
    opts.optflag("C", "print-code", "print CODE variable and exit");
    opts.optflag("f", "fetch", "also fetch from origin");
    opts.optflag("t", "time", "show time elapsed per project");
    opts.optflag("m", "modification", "show modifications or ahead/behind status");
    opts.optopt("s", "sort", "sort by: directory (d), modifications (m), time (t), ahead-behind (a)", "SORT");

    let matches = match opts.parse(&args[1..]) {
        Ok(m) => m,
        Err(f) => {
            eprintln!("{}: {}", program, f.to_string());
            usage(program, &opts);
        }
    };

    match RgsOpt::try_from(&matches) {
        Ok(rgs_opt) => {
            let mut rgs = Rgs::new(rgs_opt);
            rgs.run();
        }
        Err(err) => {
            eprintln!("{}: {}", program, err);
            usage(program, &opts);
        }
    }

    let time = now.elapsed();
    if matches.opt_present("time") {
        eprintln!("{}", format!("{}ms", time.as_millis()).black());
    }
}


