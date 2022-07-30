use std::{collections::HashMap, io::Write};

pub fn print_dl_progress(progress: &HashMap<String, (String, usize)>) {
    let mut lines: Vec<String> = vec![];

    progress.iter().for_each(|(k, v)| {
        let op = 
            if v.0 == "co" && v.1.eq(&100) { "Check out Completed" }
            else if v.0 == "co" { "Checking Out" } 
            else if v.0 == "fo" && v.1.eq(&100) { "Fetch Completed" }
            else if v.0 == "fo" { "Fetching" }
            else if v.0 == "do" && v.1.eq(&100) { "Resolving Deltas Completed" }
            else if v.0 == "do" { "Resolving Deltas" } 
            else { "..." };
            lines.push(format!("{k} => {op} {}%\n", v.1));
    });
    
    print!("{}\r", lines.join(" "));
    std::io::stdout().flush().unwrap();
    (0..lines.len()).for_each(|_| print!("\x1b[1A\x1b[2K"));
}