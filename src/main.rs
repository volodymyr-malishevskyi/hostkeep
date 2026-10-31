mod app;
mod cli;
mod config;
mod hosts;
mod i18n;
mod paths;
mod ui;

use std::io::IsTerminal;
use std::path::PathBuf;
use std::process::ExitCode;

use clap::{ArgAction, CommandFactory, FromArgMatches, Parser};

use config::Config;

#[derive(Parser)]
#[command(
    name = paths::APP_NAME,
    version,
    disable_help_flag = true,
    disable_version_flag = true,
    disable_help_subcommand = true
)]
struct Cli {
    #[command(subcommand)]
    command: Option<cli::Command>,
    #[arg(short, long, value_name = "PATH", global = true)]
    file: Option<PathBuf>,
    #[arg(long, value_name = "LANG", value_parser = ["uk", "en"], hide_possible_values = true, global = true)]
    lang: Option<String>,
    #[arg(long, global = true)]
    dry_run: bool,
    #[arg(short, long, global = true)]
    quiet: bool,
    #[arg(short, long, action = ArgAction::Help, global = true)]
    help: Option<bool>,
    #[arg(short = 'V', long, action = ArgAction::Version)]
    version: Option<bool>,
}

/// `--lang` must be known before clap renders a localized `--help`.
fn lang_arg() -> Option<String> {
    let args: Vec<String> = std::env::args().collect();
    args.iter().enumerate().find_map(|(i, a)| {
        a.strip_prefix("--lang=")
            .map(str::to_string)
            .or_else(|| (a == "--lang").then(|| args.get(i + 1).cloned()).flatten())
    })
}

fn main() -> ExitCode {
    let config_path = paths::config_file();
    let (config, config_error) = match Config::load(&config_path) {
        Ok(c) => (c, None),
        Err(e) => (Config::default(), Some(format!("{e:#}"))),
    };
    i18n::set_lang(i18n::resolve(lang_arg().as_deref(), &config.language));

    let command = cli::localize(
        Cli::command()
            .about(t!("cli-about"))
            .mut_arg("file", |a| a.help(t!("cli-file")))
            .mut_arg("lang", |a| a.help(t!("cli-lang")))
            .mut_arg("dry_run", |a| a.help(t!("cli-dry-run")))
            .mut_arg("quiet", |a| a.help(t!("cli-quiet")))
            .mut_arg("help", |a| a.help(t!("cli-help")))
            .mut_arg("version", |a| a.help(t!("cli-version"))),
    );
    let cli = match Cli::from_arg_matches(&command.clone().get_matches()) {
        Ok(cli) => cli,
        Err(e) => e.exit(),
    };

    if let Some(e) = config_error {
        eprintln!(
            "{}",
            t!("error-config", path = config_path.display().to_string(), error = e)
        );
    }
    let path = cli.file.unwrap_or_else(|| config.hosts_path.clone());
    if let Some(cmd) = cli.command {
        let ctx = cli::Context {
            path,
            config,
            dry_run: cli.dry_run,
            quiet: cli.quiet,
        };
        return match cli::run(cmd, &ctx, command) {
            Ok(()) => ExitCode::SUCCESS,
            Err(e) => {
                eprintln!("{e:#}");
                ExitCode::FAILURE
            }
        };
    }

    if !std::io::stdout().is_terminal() {
        eprintln!("{}", t!("error-not-terminal"));
        return ExitCode::FAILURE;
    }

    let app = match app::App::new(path.clone(), config, config_path) {
        Ok(app) => app,
        Err(e) => {
            eprintln!(
                "{}",
                t!(
                    "error-open",
                    path = path.display().to_string(),
                    error = format!("{e:#}")
                )
            );
            return ExitCode::FAILURE;
        }
    };
    match app::run(app) {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("{e:#}");
            ExitCode::FAILURE
        }
    }
}
