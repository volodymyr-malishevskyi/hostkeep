//! Commands that work on the file without the TUI: `hostkeep list`, `on`, `off`, …

use std::fs;
use std::io::{self, IsTerminal};
use std::path::{Path, PathBuf};

use anyhow::{Result, anyhow, bail};
use clap::{Args, Subcommand};
use clap_complete::Shell;
use serde::Serialize;

use crate::app::App;
use crate::config::{Config, SYSTEM_HOSTS};
use crate::hosts::model::{Document, Entry, Group};
use crate::hosts::validate::{self, is_protected_host};
use crate::hosts::writer::{self, WriteResult};
use crate::paths;
use crate::t;

#[derive(Subcommand)]
pub enum Command {
    #[command(disable_help_flag = true)]
    List {
        group: Option<String>,
        #[arg(long, conflicts_with = "disabled")]
        enabled: bool,
        #[arg(long)]
        disabled: bool,
        #[arg(long)]
        json: bool,
    },
    #[command(disable_help_flag = true)]
    Groups {
        #[arg(long)]
        json: bool,
    },
    #[command(disable_help_flag = true)]
    On(Targets),
    #[command(disable_help_flag = true)]
    Off {
        #[command(flatten)]
        targets: Targets,
        #[arg(long)]
        force: bool,
    },
    #[command(disable_help_flag = true)]
    Toggle {
        #[command(flatten)]
        targets: Targets,
        #[arg(long)]
        force: bool,
    },
    #[command(disable_help_flag = true)]
    Add {
        ip: String,
        #[arg(required = true, value_name = "HOST")]
        hosts: Vec<String>,
        #[arg(short, long)]
        group: Option<String>,
        #[arg(short, long)]
        comment: Option<String>,
        #[arg(long)]
        disabled: bool,
    },
    #[command(disable_help_flag = true)]
    Rm {
        #[arg(required = true, value_name = "NAME")]
        names: Vec<String>,
        #[arg(long)]
        group: bool,
        #[arg(long, requires = "group")]
        keep_entries: bool,
        #[arg(long)]
        force: bool,
    },
    #[command(disable_help_flag = true)]
    Completions { shell: Shell },
}

#[derive(Args)]
pub struct Targets {
    #[arg(required = true, value_name = "TARGET")]
    targets: Vec<String>,
    #[arg(long, conflicts_with = "host")]
    group: bool,
    #[arg(long)]
    host: bool,
}

const COMMANDS: [&str; 8] = ["list", "groups", "on", "off", "toggle", "add", "rm", "completions"];

/// Translated help texts and section headings; clap's own are English-only.
pub fn localize(cmd: clap::Command) -> clap::Command {
    fn helps(cmd: clap::Command, about: String, args: &[(&'static str, String)]) -> clap::Command {
        args.iter().fold(cmd.about(about), |c, (id, help)| {
            c.mut_arg(*id, |a| a.help(help.clone()))
        })
    }
    let targets = |force: bool| {
        let mut args = vec![
            ("targets", t!("arg-targets")),
            ("group", t!("arg-as-group")),
            ("host", t!("arg-as-host")),
        ];
        if force {
            args.push(("force", t!("arg-force")));
        }
        args
    };
    let mut cmd = cmd
        .mut_subcommand("list", |c| {
            helps(
                c,
                t!("cmd-list"),
                &[
                    ("group", t!("arg-list-group")),
                    ("enabled", t!("arg-enabled")),
                    ("disabled", t!("arg-disabled")),
                    ("json", t!("arg-json")),
                ],
            )
        })
        .mut_subcommand("groups", |c| helps(c, t!("cmd-groups"), &[("json", t!("arg-json"))]))
        .mut_subcommand("on", |c| helps(c, t!("cmd-on"), &targets(false)))
        .mut_subcommand("off", |c| helps(c, t!("cmd-off"), &targets(true)))
        .mut_subcommand("toggle", |c| helps(c, t!("cmd-toggle"), &targets(true)))
        .mut_subcommand("add", |c| {
            helps(
                c,
                t!("cmd-add"),
                &[
                    ("ip", t!("arg-ip")),
                    ("hosts", t!("arg-hosts")),
                    ("group", t!("arg-add-group")),
                    ("comment", t!("arg-comment")),
                    ("disabled", t!("arg-add-disabled")),
                ],
            )
        })
        .mut_subcommand("rm", |c| {
            helps(
                c,
                t!("cmd-rm"),
                &[
                    ("names", t!("arg-rm-names")),
                    ("group", t!("arg-rm-group")),
                    ("keep_entries", t!("arg-keep-entries")),
                    ("force", t!("arg-force")),
                ],
            )
        })
        .mut_subcommand("completions", |c| {
            helps(c, t!("cmd-completions"), &[("shell", t!("arg-shell"))])
        });
    for name in COMMANDS {
        cmd = cmd.mut_subcommand(name, |c| {
            let has_args = c.get_positionals().next().is_some();
            c.help_template(help_template(false, has_args))
        });
    }
    cmd.help_template(help_template(true, false))
}

fn help_template(commands: bool, arguments: bool) -> String {
    let mut out = format!("{{about}}\n\n{}: {{usage}}\n", t!("cli-usage"));
    if commands {
        out += &format!("\n{}:\n{{subcommands}}\n", t!("cli-commands"));
    }
    if arguments {
        out += &format!("\n{}:\n{{positionals}}\n", t!("cli-arguments"));
    }
    out + &format!("\n{}:\n{{options}}", t!("cli-options"))
}

pub struct Context {
    pub path: PathBuf,
    pub config: Config,
    pub dry_run: bool,
    pub quiet: bool,
}

pub fn run(command: Command, ctx: &Context, mut cli: clap::Command) -> Result<()> {
    if let Command::Completions { shell } = command {
        clap_complete::generate(shell, &mut cli, paths::APP_NAME, &mut io::stdout());
        return Ok(());
    }
    let old = fs::read_to_string(&ctx.path).map_err(|e| {
        anyhow!(t!(
            "error-open",
            path = ctx.path.display().to_string(),
            error = e.to_string()
        ))
    })?;
    let mut doc = Document::parse(&old);
    let paint = Paint::stdout();
    let messages = match command {
        Command::List {
            group,
            enabled,
            disabled,
            json,
        } => {
            let state = (enabled || disabled).then_some(enabled);
            return list(&doc, group.as_deref(), state, json, paint);
        }
        Command::Groups { json } => return groups(&doc, json, paint),
        Command::On(targets) => switch(&mut doc, &targets, Mode::On, false)?,
        Command::Off { targets, force } => switch(&mut doc, &targets, Mode::Off, force)?,
        Command::Toggle { targets, force } => switch(&mut doc, &targets, Mode::Toggle, force)?,
        Command::Add {
            ip,
            hosts,
            group,
            comment,
            disabled,
        } => add(&mut doc, &ip, hosts, group.as_deref(), comment, disabled)?,
        Command::Rm {
            names,
            group: true,
            keep_entries,
            force,
        } => remove_groups(&mut doc, &names, keep_entries, force)?,
        Command::Rm { names, force, .. } => remove_hosts(&mut doc, &names, force)?,
        Command::Completions { .. } => unreachable!(),
    };
    commit(ctx, &old, &doc, &messages, paint)
}

/// ANSI colors, only for a terminal and without `NO_COLOR`.
#[derive(Clone, Copy)]
struct Paint(bool);

impl Paint {
    fn stdout() -> Self {
        Paint(io::stdout().is_terminal() && std::env::var_os("NO_COLOR").is_none_or(|v| v.is_empty()))
    }

    fn apply(self, code: &str, text: &str) -> String {
        if self.0 {
            format!("\x1b[{code}m{text}\x1b[0m")
        } else {
            text.to_string()
        }
    }
}

const GREEN: &str = "32";
const RED: &str = "31";
const YELLOW: &str = "33";
const DIM: &str = "90";
const BOLD: &str = "1";

fn entry_text(e: &Entry) -> String {
    format!("{} {}", e.ip, e.hosts.join(" "))
}

fn group_label(g: &Group) -> String {
    g.name.clone().unwrap_or_else(|| t!("group-ungrouped"))
}

/// Named group by case-insensitive name.
fn find_named<'a>(groups: &'a [Group], name: &str) -> Option<&'a Group> {
    groups
        .iter()
        .find(|g| g.name.as_deref().is_some_and(|n| n.eq_ignore_ascii_case(name)))
}

fn has_host(e: &Entry, host: &str) -> bool {
    e.hosts.iter().any(|h| h.eq_ignore_ascii_case(host))
}

fn host_entries(doc: &Document, host: &str) -> Vec<usize> {
    doc.entries()
        .filter(|(_, e)| has_host(e, host))
        .map(|(i, _)| i)
        .collect()
}

// ---- list / groups ----

#[derive(Serialize)]
struct EntryJson<'a> {
    ip: &'a str,
    hosts: &'a [String],
    comment: Option<&'a str>,
    enabled: bool,
    group: Option<&'a str>,
    protected: bool,
}

#[derive(Serialize)]
struct GroupJson<'a> {
    name: Option<&'a str>,
    enabled: usize,
    total: usize,
}

fn list(doc: &Document, filter: Option<&str>, state: Option<bool>, json: bool, paint: Paint) -> Result<()> {
    let groups = doc.groups();
    let selected: Vec<&Group> = match filter {
        Some(name) => vec![find_named(&groups, name).ok_or_else(|| anyhow!(t!("cli-error-no-group", name = name)))?],
        None => groups.iter().collect(),
    };
    let shown = |g: &Group| -> Vec<(usize, &Entry)> {
        g.entries
            .iter()
            .filter_map(|&i| doc.entry(i).map(|e| (i, e)))
            .filter(|(_, e)| state.is_none_or(|s| e.enabled == s))
            .collect()
    };

    if json {
        let items: Vec<EntryJson> = selected
            .iter()
            .flat_map(|g| {
                shown(g).into_iter().map(|(_, e)| EntryJson {
                    ip: &e.ip,
                    hosts: &e.hosts,
                    comment: e.comment.as_deref(),
                    enabled: e.enabled,
                    group: g.name.as_deref(),
                    protected: e.is_protected(),
                })
            })
            .collect();
        println!("{}", serde_json::to_string_pretty(&items)?);
        return Ok(());
    }

    let report = validate::analyze(doc);
    let all: Vec<&Entry> = selected.iter().flat_map(|g| shown(g)).map(|(_, e)| e).collect();
    let ip_w = all.iter().map(|e| e.ip.chars().count()).max().unwrap_or(0);
    let hosts_w = all.iter().map(|e| e.hosts.join(" ").chars().count()).max().unwrap_or(0);
    let name_w = selected
        .iter()
        .map(|g| group_label(g).chars().count())
        .max()
        .unwrap_or(0);
    for g in &selected {
        let rows = shown(g);
        if rows.is_empty() && filter.is_none() {
            continue;
        }
        let label = group_label(g);
        let pad = " ".repeat(name_w - label.chars().count() + 2);
        let count = format!("{}/{}", g.enabled_count(doc), g.entries.len());
        println!("{}{pad}{}", paint.apply(BOLD, &label), paint.apply(DIM, &count));
        for (i, e) in rows {
            let state = if e.enabled {
                paint.apply(GREEN, "●")
            } else {
                paint.apply(DIM, "○")
            };
            let flag = if e.enabled && report.conflict_lines.contains(&i) {
                paint.apply(YELLOW, "⚠")
            } else if e.is_protected() {
                paint.apply(DIM, "◆")
            } else {
                " ".to_string()
            };
            let hosts = e.hosts.join(" ");
            let mut line = format!("  {state} {flag} {:<ip_w$}  {hosts}", e.ip);
            if let Some(c) = &e.comment {
                let pad = " ".repeat(hosts_w - hosts.chars().count() + 2);
                line += &format!("{pad}{}", paint.apply(DIM, &format!("# {c}")));
            }
            println!("{}", if e.enabled { line } else { paint.apply(DIM, &line) });
        }
    }
    Ok(())
}

fn groups(doc: &Document, json: bool, paint: Paint) -> Result<()> {
    let groups: Vec<Group> = doc
        .groups()
        .into_iter()
        .filter(|g| g.name.is_some() || !g.entries.is_empty())
        .collect();
    if json {
        let items: Vec<GroupJson> = groups
            .iter()
            .map(|g| GroupJson {
                name: g.name.as_deref(),
                enabled: g.enabled_count(doc),
                total: g.entries.len(),
            })
            .collect();
        println!("{}", serde_json::to_string_pretty(&items)?);
        return Ok(());
    }
    let name_w = groups.iter().map(|g| group_label(g).chars().count()).max().unwrap_or(0);
    for g in &groups {
        let label = group_label(g);
        let pad = " ".repeat(name_w - label.chars().count() + 2);
        let count = format!("{}/{}", g.enabled_count(doc), g.entries.len());
        println!("{label}{pad}{}", paint.apply(DIM, &count));
    }
    Ok(())
}

// ---- on / off / toggle ----

#[derive(Clone, Copy, PartialEq, Eq)]
enum Mode {
    On,
    Off,
    Toggle,
}

enum Target {
    /// Actual group name, as written in the file.
    Group(String),
    Host(String),
}

fn resolve(doc: &Document, name: &str, as_group: bool, as_host: bool) -> Result<Target> {
    let groups = doc.groups();
    let group = find_named(&groups, name).and_then(|g| g.name.clone());
    let is_host = !host_entries(doc, name).is_empty();
    match (group, is_host) {
        (Some(g), _) if as_group => Ok(Target::Group(g)),
        (Some(g), false) if !as_host => Ok(Target::Group(g)),
        (_, true) if as_host || !as_group => {
            if !as_host && find_named(&groups, name).is_some() {
                bail!(t!("cli-error-ambiguous", name = name));
            }
            Ok(Target::Host(name.to_string()))
        }
        _ if as_group => bail!(t!("cli-error-no-group", name = name)),
        _ if as_host => bail!(t!("cli-error-no-host", name = name)),
        _ => bail!(t!("cli-error-not-found", name = name)),
    }
}

fn switch(doc: &mut Document, targets: &Targets, mode: Mode, force: bool) -> Result<Vec<String>> {
    let resolved = targets
        .targets
        .iter()
        .map(|name| resolve(doc, name, targets.group, targets.host))
        .collect::<Result<Vec<_>>>()?;

    // Plan everything first, so one bad target changes nothing.
    let mut plan = Vec::new();
    for target in resolved {
        let idxs = match &target {
            Target::Group(name) => find_named(&doc.groups(), name)
                .map(|g| g.entries.clone())
                .unwrap_or_default(),
            Target::Host(host) => host_entries(doc, host),
        };
        let all_enabled = idxs.iter().all(|&i| doc.entry(i).is_some_and(|e| e.enabled));
        let enable = match mode {
            Mode::On => true,
            Mode::Off => false,
            Mode::Toggle => !all_enabled,
        };
        if let Target::Host(host) = &target
            && enable
            && idxs.len() > 1
        {
            let entries: Vec<String> = idxs
                .iter()
                .filter_map(|&i| doc.entry(i))
                .map(|e| format!("  {}", entry_text(e)))
                .collect();
            bail!(
                "{}\n{}",
                t!("cli-error-host-many", host = host.as_str()),
                entries.join("\n")
            );
        }
        if !enable && !force {
            let protected = idxs
                .iter()
                .filter_map(|&i| doc.entry(i))
                .find(|e| e.enabled && e.is_protected());
            if let Some(e) = protected {
                let host = e.hosts.iter().find(|h| is_protected_host(h)).unwrap_or(&e.hosts[0]);
                bail!(t!("cli-error-protected-off", host = host.as_str()));
            }
        }
        plan.push((target, idxs, enable));
    }

    let mut messages = Vec::new();
    for (target, idxs, enable) in plan {
        let changed: Vec<usize> = idxs
            .into_iter()
            .filter(|&i| doc.entry(i).is_some_and(|e| e.enabled != enable))
            .collect();
        for &i in &changed {
            doc.set_enabled(i, enable);
        }
        match target {
            Target::Group(name) => messages.push(match (changed.is_empty(), enable) {
                (true, true) => t!("cli-group-already-on", name = name.as_str()),
                (true, false) => t!("cli-group-already-off", name = name.as_str()),
                (false, true) => t!("cli-group-on", name = name.as_str(), count = changed.len()),
                (false, false) => t!("cli-group-off", name = name.as_str(), count = changed.len()),
            }),
            Target::Host(host) if changed.is_empty() => messages.push(if enable {
                t!("cli-host-already-on", host = host.as_str())
            } else {
                t!("cli-host-already-off", host = host.as_str())
            }),
            Target::Host(_) => messages.extend(changed.iter().filter_map(|&i| doc.entry(i)).map(|e| {
                if enable {
                    t!("cli-host-on", entry = entry_text(e))
                } else {
                    t!("cli-host-off", entry = entry_text(e))
                }
            })),
        }
    }
    Ok(messages)
}

// ---- add / rm ----

fn add(
    doc: &mut Document,
    ip: &str,
    hosts: Vec<String>,
    group: Option<&str>,
    comment: Option<String>,
    disabled: bool,
) -> Result<Vec<String>> {
    validate::validate_fields(ip, &hosts.join(" ")).map_err(|e| anyhow!(App::field_error_text(&e)))?;
    let ip = ip.trim();
    let entry = Entry::new(
        !disabled,
        ip,
        hosts,
        comment.map(|c| c.trim().to_string()).filter(|c| !c.is_empty()),
    );
    let same = |e: &Entry| {
        let mut a: Vec<String> = e.hosts.iter().map(|h| h.to_ascii_lowercase()).collect();
        let mut b: Vec<String> = entry.hosts.iter().map(|h| h.to_ascii_lowercase()).collect();
        a.sort();
        b.sort();
        e.ip == entry.ip && a == b
    };
    if doc.entries().any(|(_, e)| same(e)) {
        return Ok(vec![t!("cli-add-exists", entry = entry_text(&entry))]);
    }

    let text = entry_text(&entry);
    let (start, message) = match group.map(str::trim) {
        Some("") => bail!(t!("error-group-empty")),
        Some(name) => {
            let existing = find_named(&doc.groups(), name).and_then(|g| g.start);
            let start = existing.unwrap_or_else(|| doc.create_group(name));
            let name = find_named(&doc.groups(), name)
                .and_then(|g| g.name.clone())
                .unwrap_or_default();
            (Some(start), t!("cli-added-to", entry = text, group = name))
        }
        None => (None, t!("cli-added", entry = text)),
    };
    doc.add_entry(entry, start);
    Ok(vec![message])
}

fn remove_hosts(doc: &mut Document, names: &[String], force: bool) -> Result<Vec<String>> {
    for name in names {
        if host_entries(doc, name).is_empty() {
            if let Some(group) = find_named(&doc.groups(), name) {
                bail!(t!("cli-error-rm-group-hint", name = group_label(group)));
            }
            bail!(t!("cli-error-no-host", name = name.as_str()));
        }
        if !force && is_protected_host(name) {
            bail!(t!("cli-error-protected-rm", host = name.as_str()));
        }
    }
    let mut messages = Vec::new();
    for name in names {
        for i in host_entries(doc, name).into_iter().rev() {
            let Some(old) = doc.entry(i).cloned() else { continue };
            let mut entry = old.clone();
            entry.hosts.retain(|h| !h.eq_ignore_ascii_case(name));
            if entry.hosts.is_empty() {
                doc.delete_entries(&[i]);
                messages.push(t!("cli-removed-entry", entry = entry_text(&old)));
            } else {
                let group = doc.group_of(i);
                doc.update_entry(i, entry, group);
                messages.push(t!("cli-removed-host", host = name.as_str(), entry = entry_text(&old)));
            }
        }
    }
    Ok(messages)
}

fn remove_groups(doc: &mut Document, names: &[String], keep_entries: bool, force: bool) -> Result<Vec<String>> {
    let groups = doc.groups();
    let mut targets = Vec::new();
    for name in names {
        let group = find_named(&groups, name).ok_or_else(|| anyhow!(t!("cli-error-no-group", name = name.as_str())))?;
        let protected = group
            .entries
            .iter()
            .filter_map(|&i| doc.entry(i))
            .any(Entry::is_protected);
        if protected && !keep_entries && !force {
            bail!(t!("cli-error-protected-group", name = group_label(group)));
        }
        targets.push((group_label(group), group.entries.len()));
    }
    let mut messages = Vec::new();
    for (name, count) in targets {
        if let Some(start) = doc.find_group(&name) {
            doc.delete_group(start, keep_entries);
            messages.push(if keep_entries {
                t!("cli-group-unwrapped", name = name.as_str(), count = count)
            } else {
                t!("cli-group-removed", name = name.as_str(), count = count)
            });
        }
    }
    Ok(messages)
}

// ---- saving ----

/// Warns about hosts that the command left enabled with different IPs.
fn warn_conflicts(doc: &Document, old: &str) {
    let before = validate::analyze(&Document::parse(old));
    for c in validate::analyze(doc).conflicts {
        if !before.conflicts.contains(&c) {
            eprintln!(
                "{}",
                Paint(io::stderr().is_terminal()).apply(
                    YELLOW,
                    &t!("cli-warn-conflict", host = c.host.as_str(), ips = c.ips.join(", "))
                )
            );
        }
    }
}

fn commit(ctx: &Context, old: &str, doc: &Document, messages: &[String], paint: Paint) -> Result<()> {
    let say = |text: &str| {
        if !ctx.quiet {
            println!("{text}");
        }
    };
    let new = doc.to_text();
    if new == old || ctx.dry_run {
        messages.iter().for_each(|m| say(m));
        warn_conflicts(doc, old);
        if new != old {
            print_diff(ctx, old, &new, paint);
        }
        return Ok(());
    }

    let backup = writer::backup(old, &paths::backups_dir(), ctx.config.backups_keep)
        .map_err(|e| anyhow!(t!("status-backup-failed", error = format!("{e:#}"))))?;
    let written =
        writer::write(&ctx.path, &new).map_err(|e| anyhow!(t!("status-save-failed", error = format!("{e:#}"))))?;
    if written == WriteResult::NeedsPassword {
        if !io::stdin().is_terminal() {
            bail!(t!("cli-error-sudo-no-tty", path = ctx.path.display().to_string()));
        }
        eprintln!("{}", t!("sudo-prompt", path = ctx.path.display().to_string()));
        writer::write_with_sudo(&ctx.path, &new).map_err(|_| anyhow!(t!("cli-error-sudo-failed")))?;
    }

    messages.iter().for_each(|m| say(m));
    warn_conflicts(doc, old);
    let backup = backup
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_default();
    let mut saved = t!("status-saved", backup = backup);
    if ctx.config.flush_dns && ctx.path == Path::new(SYSTEM_HOSTS) {
        match writer::flush_dns() {
            Some(true) => saved = format!("{saved} · {}", t!("status-dns-flushed")),
            Some(false) => eprintln!("{}", t!("status-dns-failed")),
            None => {}
        }
    }
    say(&paint.apply(DIM, &saved));
    Ok(())
}

fn print_diff(ctx: &Context, old: &str, new: &str, paint: Paint) {
    let path = ctx.path.display().to_string();
    let diff = similar::TextDiff::from_lines(old, new);
    let text = diff.unified_diff().header(&path, &format!("{path} (new)")).to_string();
    for line in text.lines() {
        let colored = match line.chars().next() {
            Some('+') if !line.starts_with("+++") => paint.apply(GREEN, line),
            Some('-') if !line.starts_with("---") => paint.apply(RED, line),
            Some('@') => paint.apply(DIM, line),
            _ => line.to_string(),
        };
        println!("{colored}");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const HOSTS: &str = "\
127.0.0.1 localhost
# @group Docker
127.0.0.1 postgres.local
# 127.0.0.1 kafka.local
# @end
# @group Staging
# 203.0.113.10 api.local
# @end
# @group Local
127.0.0.1 api.local admin.local
# @end
";

    fn targets(names: &[&str]) -> Targets {
        Targets {
            targets: names.iter().map(|s| s.to_string()).collect(),
            group: false,
            host: false,
        }
    }

    fn enabled(doc: &Document, host: &str) -> Vec<bool> {
        host_entries(doc, host)
            .iter()
            .filter_map(|&i| doc.entry(i))
            .map(|e| e.enabled)
            .collect()
    }

    #[test]
    fn switches_groups_and_hosts() {
        let mut doc = Document::parse(HOSTS);
        switch(&mut doc, &targets(&["docker"]), Mode::On, false).unwrap();
        assert_eq!(enabled(&doc, "kafka.local"), [true]);
        switch(&mut doc, &targets(&["kafka.local"]), Mode::Off, false).unwrap();
        assert_eq!(enabled(&doc, "kafka.local"), [false]);
        switch(&mut doc, &targets(&["Docker"]), Mode::Toggle, false).unwrap();
        assert_eq!(enabled(&doc, "postgres.local"), [true]);
        assert_eq!(enabled(&doc, "kafka.local"), [true]);
    }

    #[test]
    fn nothing_changes_when_one_target_fails() {
        let mut doc = Document::parse(HOSTS);
        assert!(switch(&mut doc, &targets(&["Docker", "nope.local"]), Mode::On, false).is_err());
        assert_eq!(doc.to_text(), HOSTS);
    }

    #[test]
    fn refuses_ambiguous_and_risky_changes() {
        let mut doc = Document::parse(HOSTS);
        // api.local is in two entries: enabling both would conflict.
        assert!(switch(&mut doc, &targets(&["api.local"]), Mode::On, false).is_err());
        assert!(switch(&mut doc, &targets(&["localhost"]), Mode::Off, false).is_err());
        switch(&mut doc, &targets(&["localhost"]), Mode::Off, true).unwrap();
        assert_eq!(enabled(&doc, "localhost"), [false]);

        let mut doc = Document::parse("# @group api.local\n# @end\n127.0.0.1 api.local\n");
        assert!(resolve(&doc, "api.local", false, false).is_err());
        assert!(matches!(resolve(&doc, "api.local", true, false), Ok(Target::Group(_))));
        assert!(matches!(resolve(&doc, "api.local", false, true), Ok(Target::Host(_))));
        switch(
            &mut doc,
            &Targets {
                host: true,
                ..targets(&["api.local"])
            },
            Mode::Off,
            false,
        )
        .unwrap();
        assert_eq!(enabled(&doc, "api.local"), [false]);
    }

    #[test]
    fn adds_entries() {
        let mut doc = Document::parse(HOSTS);
        add(
            &mut doc,
            "10.0.0.1",
            vec!["vpn.local".into()],
            Some("work"),
            None,
            false,
        )
        .unwrap();
        assert!(
            doc.to_text()
                .ends_with(&format!("# @group work\n{:<15} vpn.local\n# @end\n", "10.0.0.1"))
        );
        add(
            &mut doc,
            "10.0.0.2",
            vec!["db.local".into()],
            Some("DOCKER"),
            None,
            false,
        )
        .unwrap();
        assert!(
            doc.to_text()
                .contains(&format!("# 127.0.0.1 kafka.local\n{:<15} db.local\n", "10.0.0.2"))
        );

        let before = doc.to_text();
        add(&mut doc, "10.0.0.1", vec!["VPN.local".into()], None, None, false).unwrap();
        assert_eq!(doc.to_text(), before, "an existing entry is not added twice");
        assert!(add(&mut doc, "10.0.0", vec!["x.local".into()], None, None, false).is_err());
    }

    #[test]
    fn removes_hosts_and_groups() {
        let mut doc = Document::parse(HOSTS);
        remove_hosts(&mut doc, &["admin.local".into()], false).unwrap();
        assert!(doc.to_text().contains(&format!("\n{:<15} api.local\n", "127.0.0.1")));
        remove_hosts(&mut doc, &["postgres.local".into()], false).unwrap();
        assert!(!doc.to_text().contains("postgres.local"));
        assert!(remove_hosts(&mut doc, &["localhost".into()], false).is_err());

        let mut doc = Document::parse(HOSTS);
        remove_groups(&mut doc, &["staging".into()], false, false).unwrap();
        assert!(!doc.to_text().contains("Staging") && !doc.to_text().contains("203.0.113.10"));
        remove_groups(&mut doc, &["Docker".into()], true, false).unwrap();
        assert!(!doc.to_text().contains("@group Docker") && doc.to_text().contains("postgres.local"));
    }
}
