#[macro_use]
extern crate log;

mod cli;
mod cli_utils;
mod input;
mod utils;

use std::{collections::HashMap, error::Error, process};

use input::is_git;
use tokei::{Config, Language, LanguageType, Languages, Sort};
use utils::{dl::{download_repo, GitPhase}, print::print_dl_progress};

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
    let mut remote_inputs: Vec<String> = vec![];

    let (remote_tx, remote_rx) = crossbeam_channel::unbounded::<(GitPhase, String)>();

    if let Ok(paths) = create_repo_dl_path(&input) {
        // When there is a pre-existing folder, it can cause a bug to hang
        std::fs::remove_dir_all(std::path::Path::new("/tmp/tokei"))?;

        rayon::scope(|scope| {
            for (path, uri) in paths {
                let mut sender = remote_tx.clone();
                remote_inputs.push(path.clone());

                scope.spawn(move |_| {
                    download_repo(path.as_str(), uri.as_str(), &mut sender);
                });
            }

            drop(remote_tx);

            input.extend(remote_inputs.iter().map(|a| a.as_str()));
            input = input
                .clone()
                .into_iter()
                .filter(|item| !is_git(item))
                .collect();

            // Track cloning progress in a separate thread because
            // recv() is blocking
            scope.spawn(move |_| {
                let mut progress = HashMap::new();
                while let Ok((phase, uri)) = remote_rx.recv() {
                    progress.insert(uri, phase);

                    print_dl_progress(&progress);
                }
            });
        });

        // Pass it to lang stat
    } else {
        for path in &input {
            if ::std::fs::metadata(path).is_err() {
                eprintln!("Error: '{}' not found.", path);
                process::exit(1);
            }
        }
    }

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

    // Don't leave any artifacts
    std::fs::remove_dir_all(std::path::Path::new("/tmp/tokei"))?;

    Ok(())
}
