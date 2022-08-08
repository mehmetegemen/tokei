use std::{collections::HashMap, io::Write};

use super::dl::GitPhase;

pub fn print_dl_progress(progress: &HashMap<String, GitPhase>) {
    let mut lines: Vec<String> = vec![];

    progress.iter().for_each(|(k, v)| {
        match v {
            GitPhase::Fetch(c, t) => lines.push(format!("{k} => @Fetching {progress}% - {c}/{t} bytes\n", progress = c * 100 / t)),
            GitPhase::DeltaResolve(c, t) => lines.push(format!("{k} => #Resolving Deltas {progress}% - {c}/{t} deltas\n", progress = c * 100 / t)),
            GitPhase::Checkout(c, t) => lines.push(format!("{k} => ^Checking out {progress}% - {c}/{t} objects\n", progress = c * 100 / t)),
        }
    });

    print!("{}\r", lines.join(""));
    (0..lines.len()).for_each(|_| print!("\x1b[1A\x1b[2K"));
}
