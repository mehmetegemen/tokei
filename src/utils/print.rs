use std::{collections::HashMap, io::Write};

pub fn print_dl_progress(progress: &HashMap<String, (String, usize)>) {
    let mut lines: Vec<String> = vec![];

    progress.iter().for_each(|(k, v)| {
        match (v.0.as_str(), v.1) {
            ("co", 100) => lines.push(format!("{k} => +Checkout Completed\n")),
            ("co", _) => lines.push(format!("{k} => ^Checking out {}%\n", v.1)),
            ("fo", 100) => lines.push(format!("{k} => +Fetched \n")),
            ("fo", _) => lines.push(format!("{k} => @Fetching {}%\n", v.1)),
            ("do", 100) => lines.push(format!("{k} => +Resolved Deltas\n")),
            ("do", _) => lines.push(format!("{k} => #Resolving Deltas {}%\n", v.1)),
            (_, _) => lines.push(format!("{k} => (o.O) Undefined Behavior {}%\n", v.1)),
        }
    });

    print!("{}\r", lines.join(" "));
    (0..lines.len()).for_each(|_| print!("\x1b[1A\x1b[2K"));
}
