use notify_rust::Notification;
use crate::git;
use std::process;
use std::path::PathBuf;

pub fn notify(repo: &PathBuf, notify_body: &String) {
    match Notification::new()
        .summary(format!("cgs watch ({})", repo.file_name().unwrap().to_str().unwrap()).as_str())
        .body(notify_body.as_str())
        .icon("git")
        .action("pull", "Pull")
        .action("open", "Open")
        .show() {
        Ok(handle) => {
            #[cfg(not(target_os = "windows"))]
                handle.wait_for_action(|id| {
                match id {
                    "pull" => {
                        let branch = git::current_branch_from_path(repo).unwrap_or_default();
                        let ff_res = git::fast_forward(repo, &branch);
                        if ff_res.is_ok() {
                            Notification::new()
                                .summary("cgs fast-forward")
                                .body(format!("Fast-forwarded: {}", repo.to_str().unwrap()).as_str())
                                .icon("git")
                                .show();
                        } else {
                            Notification::new()
                                .summary("cgs fast-forward")
                                .body(format!("Fast-forward failed: {}", repo.to_str().unwrap()).as_str())
                                .icon("abrt")
                                .show();
                        }
                    }
                    "open" => {
                        #[cfg(target_os = "linux")]
                            let command = "xdg-open";
                        #[cfg(target_os = "windows")]
                            let command = "explorer";
                        #[cfg(target_os = "macos")]
                            let command = "open";
                        process::Command::new(command)
                            .arg(repo.to_str().unwrap())
                            .spawn()
                            .unwrap();
                    }
                    _ => {}
                };
            })
        }
        Err(err) => { eprintln!("cgs: {}: unable to show notification", err) }
    }
}