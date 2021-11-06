// parse the command line arguments
use clap::{App, Arg, Shell, SubCommand};

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

source <(kamu completions bash)

You will need to reload your shell session (or execute the same command in your current one) for changes to take effect.

#### ZSH ####

Append the following to your '~/.zshrc':

autoload -U +X bashcompinit && bashcompinit
source <(kamu completions bash)

Please contribute a guide for your favorite shell! "
                )
                .arg(
                    Arg::with_name("shell")
                        .required(true)
                        .takes_value(true)
                        .index(1)
                        .possible_values(&Shell::variants()),
                ))
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
}
