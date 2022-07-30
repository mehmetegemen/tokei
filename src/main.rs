#[macro_use]
extern crate log;

mod cli;
mod cli_utils;
mod input;
mod utils;

use std::{error::Error, process, collections::HashMap, io::Write, cell::RefCell, ops::Add, sync::Arc};

use parking_lot::Mutex;
use rayon::{iter::{IntoParallelRefIterator, ParallelIterator}, vec};
use tokei::{Config, Languages, Sort, Language, LanguageType};
use utils::dl::download_repo;

use std::io;

use crate::{
    cli::Cli,
    cli_utils::{Printer, FALLBACK_ROW_LEN},
    input::{add_input, create_repo_dl_path},
};

fn main() -> Result<(), Box<dyn Error>> {
    let mut cli = Cli::from_args();

    if cli.print_languages {
        Cli::print_supported_languages();
        process::exit(0);
    }

    let config = cli.override_config(Config::from_config_files());
    let mut languages = Languages::new();

    if let Some(input) = cli.file_input() {
        if !add_input(input, &mut languages) {
            Cli::print_input_parse_failure(input);
            process::exit(1);
        }
    }

    let mut input = cli.input();
    let mut remote_input: Vec<String> = vec![];

    let (remote_tx, remote_rx) = crossbeam_channel::unbounded::<(String, String, usize, usize)>();

    if let Ok(paths) = create_repo_dl_path(&input) {
        // paths
        //     .par_iter()
        //     .for_each_with(remote_tx, |sender, (path, uri)| {
        //         download_repo(&path, &uri, sender);
        //     });

        let inputs_to_be_added: Arc<Mutex<Vec<String>>> = Arc::new(Mutex::new(Vec::new()));
        
        rayon::scope(|scope| {
            {
                for (path, uri) in paths {
                    let mut sender = remote_tx.clone();   
                    let data = Arc::clone(&inputs_to_be_added);
                    scope.spawn(move |_| {
                        download_repo(path.clone().as_str(), uri.as_str(), &mut sender);
                        let mut i = data.lock();
                        (*i).push(path);
                        drop(sender);
                    });
                }
            }

            drop(remote_tx);

            scope.spawn(move |_| {
                let mut progress = HashMap::new();
                while let Ok((op_code, uri, cur, total)) = remote_rx.recv() {
                    progress.insert(uri, (op_code, cur * 100 / total));
                    let mut lines: Vec<String> = vec![];
                    progress.iter().for_each(|(k, v)| {
                        let op =  if v.0 == "co" && v.1.eq(&100) { "Completed" } else if v.0 == "co" { "Checking Out" } else { "Fetching" };
                            lines.push(format!("{k} => {op} {}%\n", v.1));
                    });
                    print!("{}\r", lines.join(" "));
                    std::io::stdout().flush().unwrap();
                    (0..lines.len()).for_each(|_| print!("\x1b[1A\x1b[2K"));
                }
            });
        });

        let data = Arc::clone(&inputs_to_be_added);
        let data = data.lock();
        
        for i in (*data).iter() {
            remote_input.push(i.to_owned());
        };

        // Pass it to lang stat
    } else {
        for path in &input {
            if ::std::fs::metadata(path).is_err() {
                eprintln!("Error: '{}' not found.", path);
                process::exit(1);
            }
        }
    }

    input.extend(remote_input.iter().map(|item| item.as_str()));
    input = input.into_iter().filter(|item| !item.contains("github")).collect();
    println!("debug {:?}", input);

    let columns = cli
        .columns
        .or(config.columns)
        .or_else(|| {
            if cli.files {
                term_size::dimensions().map(|(w, _)| w)
            } else {
                None
            }
        })
        .unwrap_or(FALLBACK_ROW_LEN)
        .max(FALLBACK_ROW_LEN);

    if cli.streaming == Some(crate::cli::Streaming::Simple) {
        println!(
            "#{:^10} {:^80} {:^12} {:^12} {:^12} {:^12}",
            "language", "path", "lines", "code", "comments", "blanks"
        );
        println!(
            "{:>10} {:<80} {:>12} {:>12} {:>12} {:>12}",
            (0..10).map(|_| "#").collect::<String>(),
            (0..80).map(|_| "#").collect::<String>(),
            (0..12).map(|_| "#").collect::<String>(),
            (0..12).map(|_| "#").collect::<String>(),
            (0..12).map(|_| "#").collect::<String>(),
            (0..12).map(|_| "#").collect::<String>()
        );
    }


    languages.get_statistics(&input, &cli.ignored_directories(), &config);
    if config.for_each_fn.is_some() {
        process::exit(0);
    }

    if let Some(format) = cli.output {
        print!("{}", format.print(&languages).unwrap());
        process::exit(0);
    }

    let mut printer = Printer::new(
        columns,
        cli.files,
        io::BufWriter::new(io::stdout()),
        cli.number_format,
    );

    if languages.iter().any(|(_, lang)| lang.inaccurate) {
        printer.print_inaccuracy_warning()?;
    }

    printer.print_header()?;

    if let Some(sort_category) = cli.sort.or(config.sort) {
        for (_, ref mut language) in &mut languages {
            language.sort_by(sort_category);
        }

        let mut languages: Vec<_> = languages.iter().collect();

        match sort_category {
            Sort::Blanks => languages.sort_by(|a, b| b.1.blanks.cmp(&a.1.blanks)),
            Sort::Comments => languages.sort_by(|a, b| b.1.comments.cmp(&a.1.comments)),
            Sort::Code => languages.sort_by(|a, b| b.1.code.cmp(&a.1.code)),
            Sort::Files => languages.sort_by(|a, b| b.1.reports.len().cmp(&a.1.reports.len())),
            Sort::Lines => languages.sort_by(|a, b| b.1.lines().cmp(&a.1.lines())),
        }

        if cli.sort_reverse {
            printer.print_results(languages.into_iter().rev(), cli.compact)?;
        } else {
            printer.print_results(languages.into_iter(), cli.compact)?;
        }
    } else {
        printer.print_results(languages.iter(), cli.compact)?;
    }

    printer.print_total(&languages)?;

    Ok(())
}
