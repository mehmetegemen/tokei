use std::{collections::HashMap, io::Write};

use super::dl::GitPhase;

pub fn print_dl_progress(progress: &HashMap<String, (GitPhase, usize)>) {
    let mut lines: Vec<String> = vec![];

    progress.iter().for_each(|(k, v)| {
        match (&v.0, v.1) {
            (GitPhase::Fetch, 100) => lines.push(format!("{k} => +Fetched \n")),
            (GitPhase::Fetch, _) => lines.push(format!("{k} => @Fetching {progress}%\n", progress = v.1)),
            (GitPhase::DeltaResolve, 100) => lines.push(format!("{k} => +Resolved Deltas\n")),
            (GitPhase::DeltaResolve, _) => lines.push(format!("{k} => #Resolving Deltas {progress}%\n", progress = v.1)),
            (GitPhase::Checkout, 100) => lines.push(format!("{k} => +Checkout Completed\n")),
            (GitPhase::Checkout, _) => lines.push(format!("{k} => ^Checking out {progress}%\n", progress = v.1)),
        }
    });

    print!("{}\r", lines.join(" "));
    (0..lines.len()).for_each(|_| print!("\x1b[1A\x1b[2K"));
}
