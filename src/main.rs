use clap::Shell;
use colored::Colorize;
use indicatif::{ProgressBar, ProgressStyle};
use man::prelude::*;
use prettytable::{cell, row, Cell, Row, Table};
use std::process::Command;
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
        let page = Manual::new("decoreco")
        .about("re-encode video and audio files to save space")
        .author(Author::new("Justyn Boyer (Jabster28)").email("justynboyer@gmail.com"))
        // also add a dry run flag
        .flag(
            Flag::new()
                .short("d")
                .long("dry-run")
                .help("don't actually do anything"),
        )
        // and a depth option
        .option(
            Opt::new("depth")
                .short("D")
                .long("depth")
                .help("set the depth of the tree to search for files"),
        )
        // and a list flag
        .flag(
            Flag::new()
                .short("l")
                .long("list")
                .help("list all files that would be processed and their sizes"),
        )
        // and a sort flag
        .flag(
            Flag::new()
                .short("s")
                .long("sort")
                .help("sort the list of files by size"),
        )
        // and a reverse flag
        .flag(
            Flag::new()
                .short("r")
                .long("reverse")
                .help("reverse the sort order"),
        )
        // and a set option
        .option(
            Opt::new("set")
                .short("S")
                .long("set")
                .help("process only these files, ignore path"),
        )
        // and a video codec option
        .option(
            Opt::new("video-codec")
                .short("v")
                .long("video-codec")
                .help("set the video codec to use. see CODECS for more info")
                .default_value("h264"),
        )
        // and an audio codec option
        .option(
            Opt::new("audio-codec")
                .short("a")
                .long("audio-codec")
                .help("set the audio codec to use. see CODECS for more info")
                .default_value("aac"),
        )
        .example(
            Example::new()
                .text("re-encode all video files in your downloads folder to h264 and aac")
                .command("decoreco ~/Downloads"),
        )
        .example(
            Example::new()
                .text("re-encode all video files in your downloads folder to hevc and mp3")
                .command("decoreco -v hevc -a mp3 ~/Downloads"),
        )
        .example(
            Example::new()
                .text("list all video files in your home folder and sort them by size")
                .command("decoreco -l -s ~/"),
        )
        .example(
            Example::new()
                .text("perform a dry run of converting your movies folder to avi")
                .command("decoreco -d -v avi ~/Movies"),
        )
                .custom(
            Section::new("codecs")
                .paragraph("the following codecs are supported in order of general size while retaining quality, smallest to largest:")
                .paragraph("(video) hevc, vp9, [h264], , vp8").paragraph("(audio) [aac], opus, vorbis, mp3")
                .paragraph("HEVC (also known as H.265) isn't supported by many web browsers or operating systems at the moment, and as such some videos might not play after you re-encode them. This codec should only be used if you don't plan on sharing the files over the internet without transcoding them (like using a media server such as plex or emby), or unless you're confident that your software and hardware can play it.").paragraph("Encoding HEVC also takes quite a bit longer thn h264, due to the higher compression ratio.")
        );
        // save to a tempdir
        let tempdir = Builder::new().prefix("decoreco").tempdir().unwrap();
        let manpage = tempdir.path().join("decoreco.1");
        let mut file = std::fs::File::create(&manpage).unwrap();
        std::io::Write::write_all(&mut file, page.render().as_bytes())
            .and_then(|_| {
                Command::new("man").arg(manpage).status().map(|status| {
                    if status.success() {
                        Ok(())
                    } else {
                        Err(std::io::Error::new(std::io::ErrorKind::Other, "man failed"))
                    }
                })
            })
            .unwrap()
            .unwrap();
        return;
    }

    // run the appropriate command
    // don't search for files if set is specified
    let mut files: Vec<&str>;
    // what???
    let mut _tmp = "".to_string();
    if matches.is_present("set") {
        files = matches.values_of("set").unwrap().collect();
    } else if matches.is_present("path") {
        let check_path = matches.value_of("path").unwrap();
        // searches for media files in the given path
        println!("searching for video files in {}", check_path);
        let list = Command::new("find")
            .arg(check_path)
            // set the depth to search
            .args(if matches.is_present("depth") {
                vec!["-maxdepth", matches.value_of("depth").unwrap()]
            } else {
                vec![]
            })
            // only search for video files
            .arg("-type")
            .arg("f")
            .arg("-name")
            .arg("*.mp4")
            .arg("-o")
            .arg("-name")
            .arg("*.mkv")
            .arg("-o")
            .arg("-name")
            .arg("*.mov")
            .arg("-o")
            .arg("-name")
            .arg("*.avi")
            .output()
            .unwrap();
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
    println!("found {} files!", files.len());
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
                Cell::new(&*humanize_bytes(metadata.len() as f64)),
            ]));
        }
        table.printstd();
        return;
    }

    // keep a list of files that have been processed and their old and new sizes
    let mut processed: Vec<(String, u64, u64)> = Vec::new();

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
    let mut saved_size: u64 = 0;
    let mut total_size: u64 = 0;

    // iterates through the files
    for (i, file) in files.iter().enumerate() {
        let i = i.to_string() + "." + file.split('.').last().unwrap();
        // checks if the file is a video file
        let is_video = String::from_utf8(
            Command::new("ffprobe")
                .arg("-show_streams")
                .arg(file)
                .output()
                .unwrap()
                .stdout,
        )
        .unwrap()
        .contains("codec_type=video");
        // if not, returns
        if !is_video {
            pb.set_message(format!("{} is not a video file", file));
            continue;
        }
        let res = Command::new("ffmpeg")
            .arg("-i")
            .arg(file)
            .arg("-c:v")
            .arg(matches.value_of("codec").unwrap())
            .arg("-c:a")
            .arg(matches.value_of("audio").unwrap())
            // keep subs
            .arg("-c:s")
            .arg("copy")
            // keep metadata
            .arg("-map_metadata")
            .arg("0")
            .arg("-y")
            .arg(tmp.path().join(i.clone()).to_str().unwrap())
            .output()
            .unwrap();
        if res.status.success() {
            // check if file is bigger than the original
            let orig_file_size = Command::new("stat")
                .arg("-f")
                .arg("%z")
                .arg(file)
                .output()
                .unwrap();

            let orig_file_size: u64 = String::from_utf8(orig_file_size.stdout)
                .unwrap()
                .trim()
                .parse()
                .unwrap();

            let new_file_size = Command::new("stat")
                .arg("-f")
                .arg("%z")
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
                saved_size += orig_file_size - new_file_size;
                total_size += orig_file_size;
                // move the file to the original location if it's not a dry run
                if !matches.is_present("dry-run") {
                    Command::new("mv")
                        .arg(tmp.path().join(i.clone()).to_str().unwrap())
                        .arg(file)
                        .output()
                        .unwrap();
                }
                // add the file to the list of processed files
                processed.push((file.to_string(), orig_file_size, new_file_size));
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
    }

    // finishes the progress bar
    pb.finish_and_clear();
    // print finished in rainbows
    println!("{}", "finished!".green().bold().on_blue().underline());

    // if saved size is 0, no files were compressed
    if saved_size == 0 {
        println!("no files were compressed.");
    } else {
        // print the total size saved
        println!(
            "total size saved: {} ({}%)",
            humanize_bytes(saved_size as f64),
            (saved_size * 100) / total_size
        );
        // print the files that were compressed
        println!("files compressed:");
        let mut table = Table::new();
        table.set_format(*prettytable::format::consts::FORMAT_BOX_CHARS);
        table.set_titles(row!["file", "old size", "new size", "saved size"]);
        for (file, old_size, new_size) in processed {
            table.add_row(Row::new(vec![
                Cell::new(&truncate(
                    &file,
                    (term_size::dimensions().unwrap().0 - 60) as i32,
                )),
                Cell::new(&*humanize_bytes(old_size as f64)).style_spec("br"),
                Cell::new(&*humanize_bytes(new_size as f64)).style_spec("br"),
                Cell::new(&*humanize_bytes(old_size as f64 - new_size as f64)).style_spec("br"),
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
        println!(
            "took {}",
            format!(
                "{}:{:02}:{:02}",
                elapsed.as_secs() / 3600,
                (elapsed.as_secs() / 60) % 60,
                elapsed.as_secs() % 60
            )
            .green()
        );
    }
    // delete tempdir
    tmp.close().unwrap();
}
