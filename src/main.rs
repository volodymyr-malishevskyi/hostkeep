mod app;
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
#[command(name = paths::APP_NAME, version, disable_help_flag = true, disable_version_flag = true)]
struct Cli {
    #[arg(short, long, value_name = "PATH")]
    file: Option<PathBuf>,
    #[arg(long, value_name = "LANG", value_parser = ["uk", "en"], hide_possible_values = true)]
    lang: Option<String>,
    #[arg(short, long, action = ArgAction::Help)]
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

    // clap's own headings are English-only, so the template supplies translated ones.
    let template = format!(
        "{{about}}\n\n{}: {{usage}}\n\n{}:\n{{options}}",
        t!("cli-usage"),
        t!("cli-options")
    );
    let command = Cli::command()
        .about(t!("cli-about"))
        .help_template(template)
        .mut_arg("file", |a| a.help(t!("cli-file")))
        .mut_arg("lang", |a| a.help(t!("cli-lang")))
        .mut_arg("help", |a| a.help(t!("cli-help")))
        .mut_arg("version", |a| a.help(t!("cli-version")));
    let cli = match Cli::from_arg_matches(&command.get_matches()) {
        Ok(cli) => cli,
        Err(e) => e.exit(),
    };

    if let Some(e) = config_error {
        eprintln!(
            "{}",
            t!("error-config", path = config_path.display().to_string(), error = e)
        );
    }
    if !std::io::stdout().is_terminal() {
        eprintln!("{}", t!("error-not-terminal"));
        return ExitCode::FAILURE;
    }

    let path = cli.file.unwrap_or_else(|| config.hosts_path.clone());
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
