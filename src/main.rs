#![forbid(unsafe_code)]
use clap::Shell;
use colored::Colorize;
use indicatif::{ProgressBar, ProgressStyle};

use prettytable::{cell, row, Cell, Row, Table};
use rayon::prelude::*;
use std::{
    process::Command,
    sync::{Arc, Mutex},
};
use tempfile::Builder;

mod cli;
fn main() {
    // truncation function and add ellipsis
    let truncate = |s: &str, max: i32| -> String {
        if max <= 0 {
            return String::from("...");
        }
        let max = max as usize;
        if s.len() > max {
            format!("{}...", &s[..max])
        } else {
            s.to_string()
        }
    };

    // make bytes human readable with a precision of 2 decimal places
    let humanize_bytes = |bytes: f64| -> String {
        let mut bytes = bytes;
        let mut unit = "B";
        if bytes >= 1024.0 {
            bytes /= 1024.0;
            unit = "KiB";
        }
        if bytes >= 1024.0 {
            bytes /= 1024.0;
            unit = "MiB";
        }
        if bytes >= 1024.0 {
            bytes /= 1024.0;
            unit = "GiB";
        }
        if bytes >= 1024.0 {
            bytes /= 1024.0;
            unit = "TiB";
        }
        format!("{:.2} {}", bytes, unit)
    };
    let app = cli::cli();
    let matches = app.clone().get_matches();

    // if command is completions, print the completions and exit
    if matches.is_present("completions") {
        let shell = matches.value_of("shell").unwrap_or("bash");
        let mut app = app.clone();
        app.gen_completions_to(
            env!("CARGO_PKG_NAME"),
            match shell.to_ascii_lowercase().as_str() {
                "bash" => Shell::Bash,
                "fish" => Shell::Fish,
                "powershell" => Shell::PowerShell,
                "zsh" => Shell::Zsh,
                _ => unreachable!(),
            },
            &mut std::io::stdout(),
        );
        return;
    }

    // if manpage is requested, print the manpage and exit
    if matches.is_present("manpage") {
        cli::man();
    }
    rayon::ThreadPoolBuilder::new()
        .num_threads(matches.value_of("threads").unwrap().parse().unwrap())
        .build_global()
        .unwrap();

    // run the appropriate command
    // don't search for files if set is specified
    let mut files: Vec<&str>;
    // what???
    let mut _tmp = String::new();
    if matches.is_present("set") {
        files = matches.values_of("set").unwrap().collect();
    } else if matches.is_present("path") {
        let check_path = matches.value_of("path").unwrap();
        // searches for media files in the given path
        println!("searching for media files in {check_path}");
        let mut cmd = Command::new("find");
        let cmd = cmd
            .arg(check_path)
            // set the depth to search
            .args(if matches.is_present("depth") {
                vec!["-maxdepth", matches.value_of("depth").unwrap()]
            } else {
                vec![]
            });
        let list = if matches.is_present("images") {
            cmd.arg("-type")
                .arg("f")
                .args({
                    let types = "png jpg jpeg avif heic".split(' ').collect::<Vec<&str>>();
                    let mut args = Vec::new();
                    for t in types {
                        args.push("-o");
                        args.push("-name");
                        args.push(&*format!("*.{}", t));
                    }
                    args
                })
                .output()
                .unwrap()
        } else {
            // only search for media files
            cmd.arg("-type")
                .arg("f")
                .args({
                    let types = "mp4 mkv webm mov avi".split(' ').collect::<Vec<&str>>();
                    let mut args = Vec::new();
                    for t in types {
                        args.push("-o");
                        args.push("-name");
                        args.push(&*format!("*.{}", t));
                    }
                    args
                })
                .output()
                .unwrap()
        };
        _tmp = String::from_utf8(list.stdout).unwrap();
        files = _tmp.split('\n').collect();
        files.retain(|x| !x.is_empty());
    } else {
        // errors out and prints help if no arguments are given
        app.clone().print_help().unwrap();
        return;
    }

    // exits if there are no files to process
    if files.is_empty() {
        println!("no files found!");
        return;
    }
    // remove empty strings from the list of files
    files.retain(|x| !x.trim().is_empty());

    // remove empty files from the list of files
    files.retain(|x| std::fs::metadata(x).unwrap().len() != 0);
    println!(
        "found {} file{}!",
        files.len(),
        if files.len() == 1 { "" } else { "s" }
    );
    // sort the files by size if the user requested it
    if matches.is_present("sort") {
        files.sort_by(|a, b| {
            let a = std::fs::metadata(a).unwrap().len();
            let b = std::fs::metadata(b).unwrap().len();
            a.cmp(&b)
        });
        if matches.is_present("reverse") {
            files.reverse();
        }
    }

    // if flag is set, print the files and their sizes in a table and exit
    if matches.is_present("list") {
        let mut table = Table::new();
        table.set_format(*prettytable::format::consts::FORMAT_BOX_CHARS);
        table.set_titles(row!["file", "size"]);

        for file in files {
            let metadata = std::fs::metadata(file).unwrap();
            table.add_row(Row::new(vec![
                // truncate to terminal width minus the size column
                Cell::new(&truncate(
                    file,
                    (term_size::dimensions().unwrap().0 - 20) as i32,
                )),
                Cell::new(&humanize_bytes(metadata.len() as f64)),
            ]));
        }
        table.printstd();
        return;
    }

    // keep a list of files that have been processed and their old and new sizes
    let processed: Vec<(String, u64, u64)> = Vec::new();
    let shared_processed = Arc::new(Mutex::new(processed));

    // let user know if dry run is enabled
    if matches.is_present("dry-run") {
        println!("dry run enabled, no files will be modified.");
    }
    // starts a timer
    let start = std::time::Instant::now();

    // creates a progress bar
    let pb = ProgressBar::new(files.len() as u64);
    pb.set_style(
        ProgressStyle::default_bar()
            .template("[{elapsed_precise}] {bar:40.cyan/blue} {pos:>7}/{len:7} {msg}")
            .progress_chars("##-"),
    );
    pb.enable_steady_tick(500);
    let tmp = Builder::new().prefix("decoreco").tempdir().unwrap();
    let saved_size = Arc::new(Mutex::new(0_u64));
    let total_size = Arc::new(Mutex::new(0_u64));

    // iterates through the files
    files.par_iter().enumerate().for_each(|(i, file)| {
        let i = i.to_string() + "." + file.split('.').last().unwrap();

        if res.status.success() {
            // check if file is bigger than the original
            let orig_file_size = Command::new("stat")
                .arg("--printf=%s")
                .arg(file)
                .output()
                .unwrap();

            let orig_file_size: u64 = String::from_utf8(orig_file_size.stdout)
                .unwrap()
                .trim()
                .parse()
                .unwrap();

            let new_file_size = Command::new("stat")
                .arg("--printf=%s")
                .arg(tmp.path().join(i.clone()).to_str().unwrap())
                .output()
                .unwrap();
            let new_file_size: u64 = String::from_utf8(new_file_size.stdout)
                .unwrap()
                .trim()
                .parse()
                .unwrap();
            // println!("old: {}, new: {}", orig_file_size, new_file_size);
            pb.inc(1);

            if new_file_size < orig_file_size {
                pb.set_message(format!(
                    "{} {}",
                    format!(
                        "file is {}% smaller than original",
                        100 - (new_file_size * 100) / orig_file_size
                    )
                    .green(),
                    file
                ));
                let mut save = saved_size.lock().unwrap();
                *save += orig_file_size - new_file_size;
                let mut total = total_size.lock().unwrap();
                *total += orig_file_size;
                // move the file to the original location if it's not a dry run
                if !matches.is_present("dry-run") {
                    Command::new("mv")
                        .arg(tmp.path().join(i.clone()).to_str().unwrap())
                        .arg(
                            // if it's an img make sure to add the img ext
                            if matches.is_present("images") {
                                format!("{}.jxl", file)
                            } else {
                                file.to_string()
                            },
                        )
                        .output()
                        .unwrap();
                    if matches.is_present("images") {
                        Command::new("rm").arg(file).output().unwrap();
                    }
                }
                // add the file to the list of processed files
                let mut processed = shared_processed.lock().unwrap();

                processed.push(((*file).to_string(), orig_file_size, new_file_size));
            } else {
                println!("{} {}", orig_file_size, new_file_size);
                pb.set_message(format!(
                    "{} {}",
                    format!(
                        "file is {}% larger than original",
                        (orig_file_size * 100) / new_file_size
                    )
                    .red(),
                    file
                ));
            }

            // updates the progress bar
        } else {
            println!("{}", String::from_utf8(res.stderr).unwrap());
        }
    });

    // finishes the progress bar
    pb.finish_and_clear();
    // print finished in rainbows
    println!("{}", "finished!".green().bold().on_blue().underline());

    // if saved_size == 0 {
    let saved_size = *saved_size.lock().unwrap();
    let total_size = *total_size.lock().unwrap();
    if saved_size == 0 {
        println!("no files were compressed.");
    } else {
        // print the total size saved
        println!(
            "total size saved: {} ({}% of original)",
            humanize_bytes(saved_size as f64),
            (saved_size * 100) / total_size
        );
        // print the files that were compressed
        println!("files compressed:");
        let mut table = Table::new();
        table.set_format(*prettytable::format::consts::FORMAT_BOX_CHARS);
        table.set_titles(row!["file", "old size", "new size", "saved size"]);
        let processed = shared_processed.lock().unwrap();
        for (file, old_size, new_size) in processed.clone() {
            table.add_row(Row::new(vec![
                Cell::new(&truncate(
                    &file,
                    (term_size::dimensions().unwrap().0 - 60) as i32,
                )),
                Cell::new(&humanize_bytes(old_size as f64)).style_spec("br"),
                Cell::new(&humanize_bytes(new_size as f64)).style_spec("br"),
                Cell::new(&humanize_bytes(old_size as f64 - new_size as f64)).style_spec("br"),
            ]));
        }

        table.set_format(*prettytable::format::consts::FORMAT_BOX_CHARS);
        // add a separator row
        table.add_row(Row::new(vec![
            Cell::new(""),
            Cell::new(""),
            Cell::new(""),
            Cell::new(""),
        ]));

        // add total size saved
        table.add_row(Row::new(vec![
            Cell::new("total"),
            Cell::new(&humanize_bytes(total_size as f64)).style_spec("Frr"),
            Cell::new(&humanize_bytes((total_size - saved_size) as f64)).style_spec("Fgr"),
            Cell::new(&humanize_bytes(saved_size as f64)).style_spec("Fbr"),
        ]));

        table.printstd();

        // print time elapsed
        let elapsed = start.elapsed();
        println!("{} {}", elapsed.as_millis(), total_size);
        println!(
            "took {} total, on average {} per MB",
            format!(
                "{}:{:02}:{:02}",
                (elapsed.as_millis() / 1000) / 3600,
                ((elapsed.as_millis() / 1000) / 60) % 60,
                (elapsed.as_millis() / 1000) % 60
            )
            .green(),
            format!(
                "{:02}:{:02}:{:02}",
                (((elapsed.as_millis() as f64 / 1000.0).floor())
                    / (total_size as f64 / 1_000_000.0)
                    / 3600.0)
                    .floor(),
                ((elapsed.as_millis() as f64 / 1000.0).floor()
                    / (total_size as f64 / 1_000_000.0)
                    / 60.0)
                    .floor() as u128
                    % 60,
                ((elapsed.as_millis() as f64 / 1000.0).floor() / (total_size as f64 / 1_000_000.0))
                    .floor() as u128
                    % 60
            )
            .green(),
        );
    }
    // delete tempdir
    tmp.close().unwrap();
}

fn video(
    file: &&str,
    matches: &clap::ArgMatches<'_>,
    tmp: &tempfile::TempDir,
    i: &String,
) -> std::process::Output {
    Command::new("ffmpeg")
        .arg("-i")
        .arg(file)
        .arg("-c:v")
        .arg(matches.value_of("video-codec").unwrap())
        .arg("-c:a")
        .arg("copy")
        // keep subs
        .arg("-c:s")
        .arg("copy")
        // keep metadata
        .arg("-map_metadata")
        .arg("0")
        .arg("-y")
        .arg(tmp.path().join(i.clone()).to_str().unwrap())
        .output()
        .unwrap()
}

fn audio(
    file: &&str,
    matches: &clap::ArgMatches<'_>,
    tmp: &tempfile::TempDir,
    i: &String,
) -> std::process::Output {
    Command::new("ffmpeg")
        .arg("-i")
        .arg(file)
        .arg("-c:a")
        .arg(matches.value_of("audio-codec").unwrap())
        .arg("-c:v")
        .arg("copy")
        // keep subs
        .arg("-c:s")
        .arg("copy")
        // keep metadata
        .arg("-map_metadata")
        .arg("0")
        .arg("-y")
        .arg(tmp.path().join(i.clone()).to_str().unwrap())
        .output()
        .unwrap()
}
