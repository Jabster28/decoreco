use std::process::Command;

// parse the command line arguments
use clap::{App, Arg, Shell, SubCommand};
use man::{Author, Example, Flag, Manual, Opt, Section};
use tempfile::Builder;

pub fn cli() -> App<'static, 'static> {
    App::new("decoreco")
        .about("Re-encode video and audio files to save space.")
        .setting(clap::AppSettings::InferSubcommands)
        .setting(clap::AppSettings::ColoredHelp)
        .setting(clap::AppSettings::UnifiedHelpMessage)
        .version("1.0.0")
        .author("Jabster28 <justynboyer@gmail.com>").subcommand(
                    SubCommand::with_name("completions")
                .about("Generate tab-completion scripts for your shell")
                .after_help(r"The command outputs on 'stdout', allowing you to re-direct the output to the file of your choosing. Where you place the file will depend on which shell and which operating system you are using. Your particular configuration may also determine where these scripts need to be placed. Here are some common set ups: 

#### BASH ####
Append the following to your '~/.bashrc':

source <(decoreco completions bash)

You will need to reload your shell session (or execute the same command in your current one) for changes to take effect.

#### ZSH ####

Append the following to your '~/.zshrc':

autoload -U +X bashcompinit && bashcompinit
source <(decoreco completions bash)

Please contribute a guide for your favorite shell! "
                )
                .arg(
                    Arg::with_name("shell")
                        .required(true)
                        .takes_value(true)
                        .index(1)
                        .possible_values(&Shell::variants()),
                ))
        .subcommand(SubCommand::with_name("manpage").about("Opens our man page.").alias("info")

    )
        .arg(
            Arg::with_name("path")
                .case_insensitive(true)
                .takes_value(true)
                .index(1)
                .help("path to check for media files"),
        )
        // also add a dry run option
        .arg(
            Arg::with_name("dry-run")
                .short("d")
                .long("dry-run")
                .help("don't actually do anything"),
        )
        .arg(
            Arg::with_name("images")
                .short("i")
                .long("images")
                .help("only convert images"),
        )
        // and a depth option
        .arg(
            Arg::with_name("depth")
                .short("D")
                .long("depth")
                .takes_value(true)
                .help("how many levels deep to search for media files"),
        )
        // and an option to print the files that would be processed and their sizes and exit
        .arg(
            Arg::with_name("list")
                .short("l")
                .long("list")
                .help("list files that would be processed and their sizes"),
        )
        // and an option to sort the files by size
        .arg(
            Arg::with_name("sort")
                .short("s")
                .long("sort")
                .help("sort the files by size"),
        )
        // and an option to reverse the sort
        .arg(
            Arg::with_name("reverse")
                .short("r")
                .long("reverse")
                .help("reverse the sort"),
        )
        // only process sets of files
        .arg(
            Arg::with_name("set")
                .short("S")
                .long("set")
                .takes_value(true)
                .multiple(true)
                .help("process only these files, ignore path"),
        )
        // add a video codec option
        .arg(
            Arg::with_name("video-codec")
                .short("v")
                .long("video-codec")
                .takes_value(true)
                .help("video codec to use")
                .default_value("h264")
                .possible_values(&["h264", "hevc", "vp9", "vp8", "av1"]),
        )
        // add an audio codec option
        .arg(
            Arg::with_name("audio-codec")
                .short("a")
                .long("audio-codec")
                .takes_value(true)
                .help("audio codec to use")
                .default_value("aac")
                .possible_values(&["aac", "opus", "vorbis", "mp3"]),
        )
        // add an image codec option
        .arg(
            Arg::with_name("image-codec")
                .short("I")
                .long("image-codec")
                .takes_value(true)
                .help("image codec to use")
                .default_value("jxl")
                .possible_values(&["jxl"]),
        )
               .arg(
            Arg::with_name("threads")
                .short("t")
                .long("threads")
                .takes_value(true)
                .help("number of threads to use. default is zero, which means max")
                .default_value("0"))
}

pub fn man() {
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
        .and_then(|()| {
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
}
