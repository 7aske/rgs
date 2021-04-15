use git2::{Repository, Status, Error, BranchType, Commit, Signature, Time};

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
    let res = repo.find_remote("origin")?.fetch(&[branch], None, None);
    res
}

pub fn git_ahead_behind(path: &str) -> Result<(usize, usize), Error> {
    let repo = Repository::open(path)?;
    let branch = git_get_current_branch(&repo)?;
    let rev = repo.revparse(format!("HEAD..origin/{}", branch).as_str())?;
    let res = repo.graph_ahead_behind(rev.from().unwrap().id(), rev.to().unwrap().id())?;
    Ok(res)
}

pub fn git_commits<'a>(path: &'a str, email: &'a String) -> Result<Vec<(String, Time)>, Error> {
    let repo = Repository::open(path)?;
    let mut walk = repo.revwalk()?;
    walk.push_head();
    let commits = walk.into_iter().map(|r| {
        let r = r.unwrap();
        repo.find_commit(r).unwrap().clone()
    })
        .filter(|c| c.committer().email().unwrap() == email)
        .map(|c| {
            let committer = String::from(c.committer().email().unwrap());
            let time = c.time().clone();
            (committer, time)
        })
        .collect();
    Ok(commits)
}