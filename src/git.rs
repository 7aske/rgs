use git2::{Repository, Status};

pub fn git_is_dirty(path: &str) -> bool {
    return match Repository::open(path) {
        Ok(repo) => {
            let statuses = repo.statuses(Option::None).unwrap();
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
