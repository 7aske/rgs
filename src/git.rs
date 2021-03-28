
use std::{process};
use std::process::Stdio;

pub fn git_is_dirty(path: &str) -> bool {
    return match process::Command::new("git")
        .args(&["-C", path, "status", "--porcelain"])
        .stderr(Stdio::null())
        .output() {
        Ok(out) => out.stdout.len() == 0,
        Err(_) => false
    };
}

#[allow(dead_code)]
pub fn git_status(path: &str) -> String {
    return match process::Command::new("git")
        .args(&["-C", path, "status"])
        .env("LS_COLORS", "rs=0:di=38;5;27:mh=44;38;5;15")
        .output() {
        Ok(out) => String::from_utf8(out.stdout).unwrap_or_default(),
        Err(_) => String::from("")
    };
}

pub fn git_is_inside_work_tree(path: &str) -> bool {
    return match process::Command::new("git")
        .args(&["-C", path, "rev-parse", "--is-inside-work-tree"])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status() {
        Ok(out) => out.success(),
        Err(_) => false
    };
}
