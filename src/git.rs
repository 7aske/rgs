use git2::{Repository, Status, Error, BranchType, FetchOptions, RemoteCallbacks, Cred};
use std::env;
use std::path::Path;

pub fn git_is_clean(path: &str) -> usize {
    return match Repository::open(path) {
        Ok(repo) => {
            let statuses = repo.statuses(None).unwrap();
            statuses.iter().filter(|s| s.status() != Status::IGNORED).count()
        }
        Err(_) => 0
    };
}


pub fn git_is_inside_work_tree(path: &str) -> bool {
    return match Repository::open(path) {
        Ok(_) => true,
        Err(_) => false
    };
}

fn git_get_current_branch(repo: &Repository) -> Result<String, Error> {
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
    return Err(Error::from_str("error"));
}

pub fn git_fetch(path: &str) -> Result<(), Error> {
    let repo = Repository::open(path)?;
    let branch = git_get_current_branch(&repo)?;
    let mut callbacks = RemoteCallbacks::default();
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
    let mut fetch_opts = FetchOptions::default();
    fetch_opts.remote_callbacks(callbacks);
    match repo.find_remote("origin").unwrap().fetch(&[String::from(&branch)], Option::Some(&mut fetch_opts), None) {
        Ok(_) => {
            eprintln!("fetching {}:{}", path, branch)
        }
        Err(_) => {
            eprintln!("error fetching {}:{}", path, branch)
        }
    }
    Ok(())
}

pub fn git_ahead_behind(path: &str) -> Result<(usize, usize), Error> {
    let repo = Repository::open(path)?;
    let branch = git_get_current_branch(&repo)?;
    let rev = repo.revparse(format!("HEAD..origin/{}", branch).as_str())?;
    let res = repo.graph_ahead_behind(rev.from().unwrap().id(), rev.to().unwrap().id())?;
    Ok(res)
}