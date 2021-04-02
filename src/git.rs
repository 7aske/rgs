use git2::{Repository, Status, Error};

pub fn git_is_clean(path: &str) -> bool {
    return match Repository::open(path) {
        Ok(repo) => {
            let statuses = repo.statuses(None).unwrap();
            statuses.iter().filter(|s| s.status() != Status::IGNORED).count() == 0
        }
        Err(_) => false
    };
}


pub fn git_is_inside_work_tree(path: &str) -> bool {
    return match Repository::open(path) {
        Ok(_) => true,
        Err(_) => false
    };
}

fn git_get_current_branch(repo: &Repository) -> Result<String, Error> {
    let branch = repo.branches(None)?
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
    let res = repo.find_remote("origin")?.fetch(&[branch], None, None); res
}

pub fn git_is_clean_remote(path: &str) -> Result<bool, Error> {
    let repo = Repository::open(path)?;
    let branch = git_get_current_branch(&repo)?;
    let head = repo.head()?.peel_to_tree()?;
    let origin_head = repo.find_reference(format!("refs/remotes/origin/{}", branch).as_str())?.peel_to_tree()?;
    let diff = repo.diff_tree_to_tree(Some(&head), Some(&origin_head), None)?;
    let stats = diff.stats()?;
    Ok(stats.deletions() == 0 && stats.files_changed() == 0 && stats.insertions() == 0)
}