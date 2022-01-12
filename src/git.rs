use std::env;
use std::path::Path;

use colored::Colorize;
use git2::{BranchType, Commit, Cred, Error, FetchOptions, Oid, RemoteCallbacks, Repository, Revspec, Sort, Status, Time};
use git2::BranchType::{Local};
use git2::build::CheckoutBuilder;

pub fn is_clean(path: &str) -> usize {
    return match Repository::open(path) {
        Ok(repo) => {
            let statuses = repo.statuses(None).unwrap();
            statuses.iter().filter(|s| s.status() != Status::IGNORED).count()
        }
        Err(_) => 0
    };
}

pub fn is_inside_work_tree(path: &str) -> bool {
    let repo = Repository::open(path);
    return if repo.is_ok() {
        !repo.unwrap().is_bare()
    } else {
        false
    };
}

fn current_branch(repo: &Repository) -> Result<String, Error> {
    let branch = repo.branches(Option::from(BranchType::Local))?
        .into_iter()
        .map(|b| b.unwrap().0)
        .find(|b| b.is_head());
    if branch.is_some() {
        let branch = branch.unwrap();
        let branch = branch.name().unwrap().unwrap();
        let branch = String::from(branch);
        return Ok(branch);
    }
    return Err(Error::from_str("error parsing current branch"));
}

pub fn current_branch_from_path<P: AsRef<Path>>(path: P) -> Result<String, Error> {
    let repo = Repository::open(path)?;
    current_branch(&repo)
}

/// Lists all Local branches for a repository in a path.
pub fn branches<P: AsRef<Path>>(path: P) -> Vec<String> {
    let repo = Repository::open(path);
    let mut result = vec![];
    if repo.is_err() { return result; }
    let repo = repo.unwrap();
    let branches = repo.branches(Option::Some(BranchType::Local));
    if branches.is_err() { return result; }
    for branch in branches.unwrap() {
        if branch.is_ok() {
            let name = String::from(branch.unwrap().0.name().unwrap().unwrap());
            result.push(name);
        }
    }

    return result;
}

/// Performs `git fetch --all`.
pub fn fetch_all(path: &str) -> Result<(), Error>{
    let repo = Repository::open(path)?;
    for remote in repo.remotes().unwrap().iter() {
        fetch(path, &String::from(remote.unwrap()), &[])?;
    }
    Ok(())
}

/// Wrapper for fetching from a remote.
pub fn fetch(path: &str, remote: &String, branches: &[&String]) -> Result<(), Error> {
    let repo = Repository::open(path)?;

    let mut callbacks = RemoteCallbacks::default();
    // @Incomplete Bare-bones working credentials callback capable of handling pubkey authentication
    // using the default private key generated from ssh-keygen command.
    callbacks.credentials(|_url, username_from_url, _allowed_types| {
        let priv_key_path = format!("{}/.ssh/id_rsa", env::var("HOME").unwrap());
        let priv_key = Path::new(&priv_key_path);
        return if username_from_url.is_some() && priv_key.exists() {
            Cred::ssh_key(
                username_from_url.unwrap(),
                None,
                priv_key,
                None,
            )
        } else {
            Cred::default()
        };
    });

    // Only thing needed apart from the defaults is the credentials callback.
    let mut fetch_opts = FetchOptions::default();
    fetch_opts.remote_callbacks(callbacks);

    let mut rmt = repo.find_remote(remote)?;
    let msg = format!("fetching {}:{}", path, remote);
    eprintln!("{}", msg.green());
    rmt.fetch(branches, Option::Some(&mut fetch_opts), None)
}

#[inline(always)]
fn rev_from_to(rev: &Revspec) -> (Oid, Oid) {
    (rev.from().unwrap().id(), rev.to().unwrap().id())
}

pub fn ahead_behind(path: &str, branch: &String) -> Result<(usize, usize), Error> {
    let repo = Repository::open(path)?;
    let rev = repo.revparse(format!("HEAD..origin/{}", branch).as_str())?;
    let (from, to) = rev_from_to(&rev);
    let res = repo.graph_ahead_behind(from, to)?;

    Ok(res)
}


pub fn ahead_behind_remote(path: &str) -> Result<Vec<(String, String, usize, usize)>, Error> {
    let repo = Repository::open(path)?;
    let mut result = vec![];
    let remotes = repo.remotes()?;
    let remotes = remotes
        .iter()
        .flatten()
        .collect::<Vec<&str>>();
    for branch in repo.branches(Option::Some(Local))? {
        let branch = branch.unwrap();
        let branch = branch.0.name().unwrap().unwrap();
        let branch = String::from(branch);
        for remote in &remotes {
            let spec_str = format!("{branch}..{remote}/{branch}", remote = *remote, branch = branch);
            let rev = repo.revparse(spec_str.as_str());
            if rev.is_err() {
                continue;
            }
            let (from, to) = rev_from_to(&rev.unwrap());
            let ahead_behind = repo.graph_ahead_behind(from, to);
            if ahead_behind.is_err() {
                continue;
            }
            let ahead_behind = ahead_behind.unwrap();
            result.push((String::from(*remote), branch.clone(), ahead_behind.0, ahead_behind.1))
        }
    }

    Ok(result)
}

pub fn fast_forward<P: AsRef<Path>>(path: &P, branch: &String) -> Result<(), Error> {
    let repo = Repository::open(path)?;

    let fetch_head = repo.find_reference("FETCH_HEAD")?;
    let fetch_commit = repo.reference_to_annotated_commit(&fetch_head)?;
    let analysis = repo.merge_analysis(&[&fetch_commit])?;
    if analysis.0.is_up_to_date() {
        Ok(())
    } else if analysis.0.is_fast_forward() {
        let refname = format!("refs/heads/{}", branch);
        let mut reference = repo.find_reference(&refname)?;
        reference.set_target(fetch_commit.id(), "Fast-Forward")?;
        repo.set_head(&refname)?;
        repo.checkout_head(Some(CheckoutBuilder::default().force()))
    } else {
        Err(Error::from_str("Unable to fast-forward"))
    }
}

pub struct CommitInfo {
    pub summary: String,
    pub author: String,
    pub id: String,
    pub time: Time,
}

impl From<&Commit<'_>> for CommitInfo {
    fn from(commit: &Commit) -> Self {
        CommitInfo {
            summary: commit.summary().unwrap_or_default().to_string().clone(),
            author: commit.author().name().unwrap_or_default().to_string().clone(),
            id: commit.id().to_string().clone(),
            time: commit.time().clone(),
        }
    }
}

pub fn behind_commits(path: &str, branch: &String) -> Result<Vec<CommitInfo>, Error> {
    let repo = Repository::open(path)?;
    let rev = repo.revparse(format!("HEAD..origin/{}", branch).as_str())?;
    let (from, to) = rev_from_to(&rev);
    let mut revwalk = repo.revwalk()?;
    revwalk.set_sorting(Sort::TIME & Sort::TOPOLOGICAL);
    revwalk.push(to);
    revwalk.hide(from);
    let mut commits = vec![];

    for entry in revwalk.into_iter() {
        let commit = repo.find_commit(entry.unwrap()).unwrap();
        commits.push(CommitInfo::from(&commit));
    }
    Ok(commits)
}