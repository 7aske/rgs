use std::{env, fs};
use std::path::{Path, PathBuf};

use colored::Colorize;
use git2::{Commit, Cred, CredentialType, Error, FetchOptions, Oid, ProxyOptions, RemoteCallbacks, RemoteRedirect, Repository, Revspec, Sort, Status, Time};
use git2::BranchType::Local;
use git2::build::CheckoutBuilder;
use http::Uri;
use http::uri::InvalidUri;
use ssh_config::SSHConfig;

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
    let branch = repo.branches(Option::from(Local))?
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
    let branches = repo.branches(Some(Local));
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

fn expand_tilde<P: AsRef<Path>>(path_user_input: P) -> PathBuf {
    let p = path_user_input.as_ref();
    if !p.starts_with("~") {
        return p.to_path_buf();
    }

    let home = env::var("HOME").unwrap();

    if p == Path::new("~") {
        return PathBuf::from(home);
    }

    return PathBuf::from(home).join(p.strip_prefix("~").unwrap());
}

#[inline(always)]
fn parse_url(url_string: &str) -> Result<Uri, InvalidUri> {
    let mut default_schema =  String::from("ssh://");
    default_schema.push_str(url_string);
    default_schema.parse::<Uri>()
}

/// Wrapper for fetching from a remote.
pub fn fetch(path: &str, remote: &String, branches: &[&String]) -> Result<(), Error> {
    let repo = Repository::open(path)?;

    let mut callbacks = RemoteCallbacks::default();
    callbacks.credentials(|url_str, username_from_url, _allowed_types| {
        // If the available type is SSH we do additional parsing
        if _allowed_types.contains(CredentialType::SSH_KEY) {

            let config_path_string = expand_tilde("~/.ssh/config");
            let config_path = Path::new(&config_path_string);
            // If the config file is not present we continue with the default
            // configuration.
            if !config_path.exists() {
                return Cred::ssh_key_from_agent(username_from_url.unwrap());
            }

            // Parse the remote url into the Uri struct so that we can extract
            // the host required to match SSH config file sections.
            let url = parse_url(url_str);
            if url.is_err() {
                // If we fail I guess we return the default agent configuration
                eprintln!("{}", url.err().unwrap().to_string().red());
                return Cred::ssh_key_from_agent(username_from_url.unwrap());
            }
            let url = url.unwrap();

            // Read the config file. If in any case we fail return the default
            // ssh-agent configuration.
            let config_str = fs::read_to_string(config_path);
            if config_str.is_err() {
                return Cred::ssh_key_from_agent(username_from_url.unwrap());
            }
            let config_str = config_str.unwrap();

            let config = SSHConfig::parse_str(config_str.as_str());
            if config.is_err() {
                return Cred::ssh_key_from_agent(username_from_url.unwrap());
            }
            let config = config.unwrap();
            let host = url.host();
            if host.is_none() {
                return Cred::ssh_key_from_agent(username_from_url.unwrap());
            }
            let host_config = config.query(url.host().unwrap());

            // We care only about two ConfigKeys - User and IdentityFile.

            // First we parse the identity file from the configuration or
            // fallback to a sane default.
            let identity_file = host_config.get("IdentityFile");
            let priv_key_path =  match identity_file {
                None => {
                    let paths = vec![
                        "~/.ssh/id_rsa",
                        "~/.ssh/id_ed25519",
                        "~/.ssh/id_ecdsa",
                        "~/.ssh/id_dsa",
                    ];
                    paths.iter()
                        .map(|p| PathBuf::from(expand_tilde(p)))
                        .find(|p| p.exists())
                        .unwrap_or(PathBuf::from("~/.ssh/id_rsa"))
                },
                Some(iden) => expand_tilde(PathBuf::from(iden))
            };

            // Second is the User. If it is not overridden we default to the
            // one provided by the remote URL.
            let user = host_config.get("User");
            let user =  match user {
                None => username_from_url.unwrap_or("git"),
                Some(_user) => _user
            };

            let priv_key = Path::new(&priv_key_path);
            Cred::ssh_key(
                user,
                None,
                priv_key,
                None,
            )
        } else {
            Cred::default()
        }
    });

    // Only thing needed apart from the defaults is the credentials callback.
    let mut fetch_opts = FetchOptions::default();
    let mut proxy_opts = ProxyOptions::default();
    proxy_opts.auto();
    fetch_opts.proxy_options(proxy_opts);
    fetch_opts.follow_redirects(RemoteRedirect::All);
    fetch_opts.remote_callbacks(callbacks);

    let mut rmt = repo.find_remote(remote)?;
    let msg = format!("fetching {}:{}", path, remote);
    eprintln!("{}", msg.green());
    rmt.fetch(branches, Some(&mut fetch_opts), None)
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
    for branch in repo.branches(Some(Local))? {
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

pub fn fast_forward<P: AsRef<Path>>(path: &P, reference: &String) -> Result<(), Error> {
    if is_clean(path.as_ref().to_str().unwrap()) > 0 {
        return Ok(());
    }

    let repo = Repository::open(path)?;
    let adjusted_remote_reference = if reference.contains("/") {
        reference.clone()
    } else {
        format!("origin/{}", reference)
    };
    let adjusted_local_reference = if reference.contains("/") {
        reference.split("/").last().unwrap().to_string()
    } else {
        reference.clone()
    };

    let remote_refname = format!("refs/remotes/{}", adjusted_remote_reference);
    let local_refname = format!("refs/heads/{}", adjusted_local_reference);

    let fetch_head = repo.find_reference(&remote_refname)?;
    let fetch_commit = repo.reference_to_annotated_commit(&fetch_head)?;
    let mut local_head = repo.find_reference(&local_refname)?;

    let analysis = repo.merge_analysis_for_ref(&local_head, &[&fetch_commit])?;
    if analysis.0.is_up_to_date() {
        Ok(())
    } else if analysis.0.is_fast_forward() {
        local_head.set_target(fetch_commit.id(), "fast-forward")?;
        repo.checkout_head(Some(CheckoutBuilder::default().force()))?;
        Ok(())
    } else if analysis.0.is_normal() {
        Err(Error::from_str("Merge required"))
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