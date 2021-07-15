#![allow(unused_must_use)]

mod git;
mod lang;
mod rgs;
mod print;
mod rgs_opt;

use colored::*;
use crate::rgs::{Rgs};
use crate::rgs_opt::{RgsOpt};
use std::time::Instant;
use crate::rgs_opt::RgsOptStruct;
use structopt::StructOpt;
use std::process;

#[macro_use]
extern crate savefile_derive;

fn main() {
    let mut opt: RgsOptStruct = RgsOptStruct::from_args();
    let now = Instant::now();

    opt.load_profile();
    let rgs_opt = RgsOpt::from(&opt);
    let mut rgs = Rgs::new(rgs_opt);
    match rgs.run() {
        Ok(_) => {}
        Err(err) => {
            eprintln!("{}", err);
            process::exit(2);
        }
    }

    let time = now.elapsed();
    if opt.time {
        eprintln!("{}", format!("{}ms", time.as_millis()).black());
    }
}


