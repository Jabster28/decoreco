#![forbid(unsafe_code)]

use clap::Shell;
use colored::Colorize;
use indicatif::{ProgressBar, ProgressStyle};

use prettytable::{row, Cell, Row, Table};
use rayon::prelude::*;
use std::{
    process::Command,
    sync::{Arc, Mutex},
    time::Duration,
};
use tempfile::Builder;

mod cli;

/// Truncates a given string to a maximum length and appends "..." to the end if truncated.
///
/// # Arguments
///
/// * `s` - A string slice to be truncated.
/// * `max` - The maximum length of the truncated string.
///
/// # Returns
///
/// * A new `String` containing the truncated string.
///
/// # Examples
///
/// ```
/// let s = "Hello, world!";
/// let truncated = truncate(s, 5);
/// assert_eq!(truncated, "Hello...");
/// ```
fn truncate(s: &str, max: usize) -> String {
    if max == 0 {
        return String::from("...");
    }
    if s.len() > max {
        format!("{}...", &s[..max])
    } else {
        s.to_string()
    }
}

/// Converts bytes to a human-readable string.
///
/// # Arguments
///
/// * `bytes` - The number of bytes to convert.
///
/// # Examples
///
/// ```
/// let bytes = 1024;
/// let humanized = humanize_bytes(bytes);
/// assert_eq!(humanized, "1 KiB");
/// ```
#[allow(clippy::useless_let_if_seq)]
fn humanize_bytes<
    T: std::cmp::PartialOrd<T> + std::ops::DivAssign<T> + std::fmt::Display + From<u32>,
>(
    bytes: T,
) -> String {
    let mut bytes = bytes;
    let mut unit = if bytes >= T::from(1024) {
        bytes /= T::from(1024);
        "KiB"
    } else {
        "B"
    };
    if bytes >= T::from(1024) {
        bytes /= T::from(1024);
        unit = "MiB";
    }
    if bytes >= T::from(1024) {
        bytes /= T::from(1024);
        unit = "GiB";
    }
    if bytes >= T::from(1024) {
        bytes /= T::from(1024);
        unit = "TiB";
    }
    format!("{bytes} {unit}")
}

fn main() {
    // truncation function and add ellipsis
    // let truncate =

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
        .num_threads({
            usize::from(
                0 == matches
                    .value_of("threads")
                    .expect("no specified thread number?")
                    .parse()
                    .expect("not a usize?")
                    && !matches.is_present("images"),
            )
        })
        .build_global()
        .expect("failed to set rayon thread number. is the thread count valid?");

    // run the appropriate command
    // don't search for files if set is specified
    let mut files: Vec<&str>;
    // what???
    let tmp: String;
    if matches.is_present("set") {
        files = matches
            .values_of("set")
            .expect("set arg was empty")
            .collect();
    } else if matches.is_present("path") {
        let check_path = matches.value_of("path").expect("path arg was empty");
        // searches for media files in the given path
        println!("searching for media files in {check_path}");
        let mut cmd = Command::new("find");
        let cmd = cmd
            .arg(check_path)
            // set the depth to search
            .args(if matches.is_present("depth") {
                vec![
                    "-maxdepth",
                    matches.value_of("depth").expect("depth arg was empty"),
                ]
            } else {
                vec![]
            });
        let list = if matches.is_present("images") {
            cmd.arg("-type")
                .arg("f")
                .args({
                    let types = "jpg jpeg avif heic".split(' ').collect::<Vec<&str>>();
                    let mut args: Vec<String> = Vec::new();
                    args.push("-name".to_owned());
                    args.push("*.png".to_string());
                    for t in types {
                        args.push("-o".to_owned());
                        args.push("-name".to_owned());
                        args.push(format!("*.{t}"));
                    }
                    args
                })
                .output()
                .unwrap_or_else(|e| panic!("failed to find files: {e}"))
        } else {
            // only search for media files
            cmd.arg("-type")
                .arg("f")
                .args({
                    let types = "mkv webm mov avi".split(' ').collect::<Vec<&str>>();
                    let mut args: Vec<String> = Vec::new();
                    args.push("-name".to_owned());
                    args.push("*.mp4".to_string());
                    for t in types {
                        args.push("-o".to_owned());
                        args.push("-name".to_owned());
                        args.push(format!("*.{t}"));
                    }
                    args
                })
                .output()
                .unwrap_or_else(|e| panic!("failed to find files: {e}"))
        };
        tmp = String::from_utf8(list.stdout).expect("failed to read file list, invalid utf-8?");
        files = tmp.split('\n').collect();
        files.retain(|x| !x.is_empty());
    } else {
        // errors out and prints help if no arguments are given
        app.clone().print_help().expect("idek");
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
    files.retain(|x| match std::fs::metadata(x) {
        Ok(e) => e.len() != 0,
        Err(err) => {
            println!("{}", format!("failed to read file '{x}': {err}").red());
            false
        }
    });
    println!(
        "found {} file{}!",
        files.len(),
        if files.len() == 1 { "" } else { "s" }
    );
    // sort the files by size if the user requested it
    if matches.is_present("sort") {
        files.sort_by(|a, b| {
            let a = std::fs::metadata(a)
                .expect("failed to read file info")
                .len();
            let b = std::fs::metadata(b)
                .expect("failed to read file info")
                .len();
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
            let metadata = std::fs::metadata(file).expect("failed to read file info");
            table.add_row(Row::new(vec![
                // truncate to terminal width minus the size column
                Cell::new(&truncate(
                    file,
                    term_size::dimensions()
                        .expect("failed to get terminal dimensions")
                        .0
                        - 20,
                )),
                Cell::new(&humanize_bytes(metadata.len())),
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
            .expect("failed to set progress bar template")
            .progress_chars("##-"),
    );
    pb.enable_steady_tick(Duration::from_millis(100));
    let tmp = Builder::new()
        .prefix("decoreco")
        .tempdir()
        .expect("failed to maek temp dir");
    #[allow(clippy::mutex_integer)]
    let saved_size = Arc::new(Mutex::new(0_u64));
    #[allow(clippy::mutex_integer)]
    let total_size = Arc::new(Mutex::new(0_u64));

    // iterates through the files
    files.par_iter().enumerate().for_each(|(i, file)| {
        let i = i.to_string() + "." + file.split('.').last().expect("no file ext");

        match decoreco(&matches, &tmp, &i, file) {
            Ok(()) => {
                let new_path = tmp
                    .path()
                    .join(i.clone())
                    .to_str()
                    .expect("failed to get new path")
                    .to_string();
                // check if file is bigger than the original
                let orig_file_size: u64 = String::from_utf8(
                    Command::new("stat")
                        .arg("--printf=%s")
                        .arg(file)
                        .output()
                        .expect("failed to read size")
                        .stdout,
                )
                .expect("failed to parse size into utf-8")
                .trim()
                .parse()
                .expect("failed to parse size as u64");

                let new_file_size: u64 = String::from_utf8(
                    Command::new("stat")
                        .arg("--printf=%s")
                        .arg(
                            tmp.path()
                                .join(i.clone())
                                .to_str()
                                .expect("failed to get path of tmpdir"),
                        )
                        .output()
                        .expect("failed to read size")
                        .stdout,
                )
                .expect("failed to parse size into utf-8")
                .trim()
                .parse()
                .expect("failed to parse size as u64");
                pb.inc(1);

                if new_file_size < orig_file_size {
                    pb.set_message(format!(
                        "{} {}",
                        format!(
                            "smaller by {}% ",
                            100 - (new_file_size * 100) / orig_file_size
                        )
                        .green(),
                        file
                    ));
                    *(saved_size.lock().expect("poisoned")) += orig_file_size - new_file_size;
                    *(total_size.lock().expect("poisoned")) += orig_file_size;
                    // move the file to the original location if it's not a dry run
                    if !matches.is_present("dry-run") {
                        Command::new("mv")
                            .arg(new_path)
                            .arg(
                                // if it's an img make sure to add the img ext
                                if matches.is_present("images") {
                                    format!("{}.jxl", file)
                                } else {
                                    (*file).to_string()
                                },
                            )
                            .output()
                            .expect("failed to add extension");
                        if matches.is_present("images") {
                            Command::new("rm")
                                .arg(file)
                                .output()
                                .expect("failed to remove old file");
                        }
                    }
                    // add the file to the list of processed files
                    let mut processed = shared_processed.lock().expect("poisoned");

                    processed.push(((*file).to_string(), orig_file_size, new_file_size));
                } else {
                    pb.set_message(format!(
                        "{} {}",
                        format!(" larger by {}% ", (orig_file_size * 100) / new_file_size).red(),
                        file
                    ));
                }

                // updates the progress bar
            }
            Err(str) => {
                let thing = format!("failed to decoreco: {str}").red();
                pb.inc(1);
                println!("{thing}");
            }
        }
    });

    // finishes the progress bar
    pb.finish_and_clear();
    // print finished in rainbows
    println!("done.");

    println!(
        "{}",
        { "-".repeat(term_size::dimensions().expect("failed to get term size").0) }.bold()
    );

    // if saved_size == 0 {
    let saved_size = *saved_size.lock().expect("poisoned");
    let total_size = *total_size.lock().expect("poisoned");
    if saved_size == 0 {
        println!("no files were compressed.");
    } else {
        // print the total size saved
        println!(
            "total size saved: {} ({}% of original)",
            humanize_bytes(saved_size),
            (saved_size * 100) / total_size
        );
        // print the files that were compressed
        println!("files compressed:");
        let mut table = Table::new();
        table.set_format(*prettytable::format::consts::FORMAT_BOX_CHARS);
        table.set_titles(row!["file", "old size", "new size", "saved size"]);
        let processed = shared_processed.lock().expect("poisoned").clone();
        for (file, old_size, new_size) in processed {
            table.add_row(Row::new(vec![
                Cell::new(&truncate(
                    &file,
                    term_size::dimensions().expect("failed to get term size").0 - 60,
                )),
                Cell::new(&humanize_bytes(old_size)).style_spec("br"),
                Cell::new(&humanize_bytes(new_size)).style_spec("br"),
                Cell::new(&humanize_bytes(old_size - new_size)).style_spec("br"),
            ]));
        }

        table.set_format(*prettytable::format::consts::FORMAT_BOX_CHARS);

        // add total size saved
        table.add_row(Row::new(vec![
            Cell::new("total").style_spec("Fb"),
            Cell::new(&humanize_bytes(total_size)).style_spec("Frr"),
            Cell::new(&humanize_bytes(total_size - saved_size)).style_spec("Fgr"),
            Cell::new(&humanize_bytes(saved_size)).style_spec("Fbr"),
        ]));

        table.printstd();

        // print time elapsed
        let elapsed = start.elapsed();
        println!(
            "took {} total, on average {} per MB",
            time_human(elapsed.as_millis()).green(),
            time_human(
                elapsed.as_millis() / (u128::from(total_size).checked_div(1_000_000).unwrap_or(1))
            )
            .green(),
        );
    }
    // delete tempdir
    tmp.close().expect("failed to remove tempdir");
}

/// Transcodes/recompresses a file using the given options.
///
/// # Arguments
///
/// * `matches` - The `ArgMatches` struct from clap.
/// * `tmp` - A `TempDir` to store the new file in.
/// * `i` - The index of the file.
/// * `file` - The path to the file.
///
/// # Returns
///
/// * `Ok(())` if the command succeeds.
/// * `Err(String)` if the command fails, with the error message.
///
/// # Panics
///
/// Panics if the command fails.
fn decoreco(
    matches: &clap::ArgMatches<'_>,
    tmp: &tempfile::TempDir,
    i: &str,
    file: &str,
) -> Result<(), String> {
    let binding = tmp.path().join(i);
    let arg = binding.to_str().expect("failed to get path");

    let res = if matches.is_present("images") {
        let losslessimg = // extract extension and then use match
                    match file.split('.').last().expect("no extension?") {
                        "png" => true,
                        "jpg" | "jpeg" => false,
                        // "avif" => Command::new(program)
                        _ => {
                            return Err(format!("{file} is not a supported image format"));
                        }
                    };
        let mut cmd = Command::new("cjxl");
        let cmd: &mut Command = if losslessimg {
            cmd.arg("-d").arg("0")
        } else {
            &mut cmd
        };
        match cmd.arg(file).arg(arg).output() {
            Ok(it) => it,
            Err(err) => return Err(err.to_string()),
        }
    } else {
        match Command::new("ffmpeg")
            .arg("-i")
            .arg(file)
            .arg("-c:v")
            .arg(matches.value_of("video-codec").expect("no video codec"))
            .arg("-c:a")
            .arg(matches.value_of("audio-codec").expect("no audio codec"))
            // keep subs
            .arg("-c:s")
            .arg("copy")
            // keep metadata
            .arg("-map_metadata")
            .arg("0")
            .arg("-y")
            .arg(arg)
            .output()
        {
            Ok(it) => it,
            Err(err) => return Err(err.to_string()),
        }
    };

    if !res.status.success() {
        // return path of faulty file and stderr
        return Err(format!(
            "{}\n{}\n",
            file,
            String::from_utf8(res.stderr).expect("failed to convert error to UTF-8")
        ));
    }
    Ok(())
}

/// Converts a duration in milliseconds to a human-readable string.
///
/// # Arguments
///
/// * `t` - The duration in milliseconds.
///
/// # Returns
///
/// A string representing the duration in a human-readable format. If
/// it's less than 1 second then only milliseconds are used, otherwise
/// the format uses only minutes, hours, and seconds, and omits any
/// that are equal to zero.
fn time_human(t: u128) -> String {
    if t < 1000 {
        return format!("{t}ms");
    }
    let mut t = t / 1000;
    let s = t % 60;
    t /= 60;
    let m = t % 60;
    t /= 60;
    let h = t % 60;

    let mut res = String::new();
    if h != 0 {
        res += &format!("{h}h");
    }
    if m != 0 {
        res += &format!("{m}m");
    }
    if s != 0 {
        res += &format!("{s}s");
    }
    res
}
