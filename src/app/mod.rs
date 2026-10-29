mod input;
mod keys;

use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

use anyhow::Result;
use ratatui::crossterm::event::{self, Event, KeyEventKind};
use ratatui::widgets::{ListState, TableState};

pub use input::TextInput;

use crate::config::{Config, SYSTEM_HOSTS};
use crate::hosts::model::{Document, Entry, Group};
use crate::hosts::validate::{self, FieldError, Report};
use crate::hosts::writer::{self, WriteResult};
use crate::i18n;
use crate::{paths, t, ui};

const UNDO_LIMIT: usize = 200;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Focus {
    Groups,
    Entries,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StatusKind {
    Info,
    Warn,
    Error,
}

pub struct Form {
    /// Line being edited; `None` when adding.
    pub editing: Option<usize>,
    pub ip: TextInput,
    pub hosts: TextInput,
    pub comment: TextInput,
    /// Index into [`App::group_choices`].
    pub group: usize,
    pub enabled: bool,
    pub focus: usize,
    pub error: Option<String>,
    /// Screen row of the popup's top, fixed at the first draw so the form grows downwards.
    pub top: Option<u16>,
}

impl Form {
    pub const FIELDS: usize = 5;
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DialogKind {
    DeleteEntries(Vec<usize>),
    ToggleProtected,
    DeleteGroup(usize),
    Quit,
    ExternalChange,
}

pub struct Dialog {
    pub kind: DialogKind,
    pub title: String,
    pub message: String,
    pub buttons: Vec<String>,
    pub selected: usize,
}

pub struct DiffView {
    pub old: String,
    pub new: String,
    pub scroll: u16,
}

pub struct BackupsView {
    pub files: Vec<PathBuf>,
    pub selected: usize,
    pub scroll: u16,
}

pub enum NamePurpose {
    Create,
    Rename(usize),
}

pub struct NameInput {
    pub purpose: NamePurpose,
    pub input: TextInput,
    pub error: Option<String>,
}

pub struct GroupPicker {
    pub targets: Vec<usize>,
    pub selected: usize,
}

pub enum Modal {
    Form(Form),
    Dialog(Dialog),
    Diff(DiffView),
    Raw { scroll: u16 },
    Backups(BackupsView),
    Help { scroll: u16 },
    GroupName(NameInput),
    GroupPicker(GroupPicker),
}

pub struct App {
    pub path: PathBuf,
    pub config: Config,
    pub config_path: PathBuf,
    pub doc: Document,
    /// State as last loaded from or saved to disk.
    pub saved: Document,
    /// File content on disk at load/save time, to detect external edits.
    pub disk_text: String,
    undo: Vec<Document>,
    redo: Vec<Document>,
    pub report: Report,
    pub writable: bool,

    pub focus: Focus,
    /// 0 = "All", `n` = `doc.groups()[n - 1]`.
    pub group_sel: usize,
    pub entry_sel: usize,
    pub groups_state: ListState,
    pub entries_state: TableState,
    /// Rows visible in the entries table at the last draw (for PgUp/PgDn).
    pub page_size: usize,
    /// Width of the hosts column at the last draw (to tell if a row was shortened).
    pub hosts_width: usize,
    pub filter: TextInput,
    pub searching: bool,
    pub marked: BTreeSet<usize>,

    pub modal: Option<Modal>,
    pub status: Option<(String, StatusKind)>,
    /// Content waiting for an interactive `sudo` write (TUI must be suspended).
    pending_sudo: Option<String>,
    last_backup: Option<PathBuf>,
    quit_after_save: bool,
    pub quit: bool,
}

impl App {
    pub fn new(path: PathBuf, config: Config, config_path: PathBuf) -> Result<Self> {
        let text = fs::read_to_string(&path)?;
        let doc = Document::parse(&text);
        let mut app = Self {
            writable: writer::can_write(&path),
            path,
            config,
            config_path,
            report: validate::analyze(&doc),
            saved: doc.clone(),
            doc,
            disk_text: text,
            undo: Vec::new(),
            redo: Vec::new(),
            focus: Focus::Entries,
            group_sel: 0,
            entry_sel: 0,
            groups_state: ListState::default(),
            entries_state: TableState::default(),
            page_size: 10,
            hosts_width: usize::MAX,
            filter: TextInput::default(),
            searching: false,
            marked: BTreeSet::new(),
            modal: None,
            status: None,
            pending_sudo: None,
            last_backup: None,
            quit_after_save: false,
            quit: false,
        };
        app.clamp();
        Ok(app)
    }

    pub fn is_dirty(&self) -> bool {
        self.doc != self.saved
    }

    pub fn set_status(&mut self, text: String, kind: StatusKind) {
        self.status = Some((text, kind));
    }

    // ---- view helpers ----

    pub fn groups(&self) -> Vec<Group> {
        self.doc.groups()
    }

    /// `None` when "All" is selected.
    pub fn selected_group(&self) -> Option<Group> {
        if self.group_sel == 0 {
            None
        } else {
            self.groups().into_iter().nth(self.group_sel - 1)
        }
    }

    pub fn group_label(group: &Group) -> String {
        group.name.clone().unwrap_or_else(|| t!("group-ungrouped"))
    }

    fn matches_filter(&self, entry: &Entry) -> bool {
        let q = self.filter.value.trim().to_lowercase();
        q.is_empty()
            || entry.ip.to_lowercase().contains(&q)
            || entry.hosts.iter().any(|h| h.to_lowercase().contains(&q))
            || entry.comment.as_deref().is_some_and(|c| c.to_lowercase().contains(&q))
    }

    /// Line indices of entries shown in the table.
    pub fn visible_entries(&self) -> Vec<usize> {
        let all: Vec<usize> = match self.selected_group() {
            None => self.doc.entries().map(|(i, _)| i).collect(),
            Some(g) => g.entries,
        };
        all.into_iter()
            .filter(|&i| self.doc.entry(i).is_some_and(|e| self.matches_filter(e)))
            .collect()
    }

    pub fn current_entry(&self) -> Option<usize> {
        self.visible_entries().get(self.entry_sel).copied()
    }

    /// Marked entries, or the current one when nothing is marked.
    fn targets(&self) -> Vec<usize> {
        if self.marked.is_empty() {
            self.current_entry().into_iter().collect()
        } else {
            self.marked.iter().copied().collect()
        }
    }

    /// Ungrouped first, then named groups — what the form and the picker offer.
    pub fn group_choices(&self) -> Vec<(String, Option<usize>)> {
        self.groups().iter().map(|g| (Self::group_label(g), g.start)).collect()
    }

    fn clamp(&mut self) {
        let groups = self.groups().len();
        self.group_sel = self.group_sel.min(groups);
        let n = self.visible_entries().len();
        self.entry_sel = self.entry_sel.min(n.saturating_sub(1));
    }

    fn select_line(&mut self, idx: usize) {
        if let Some(pos) = self.visible_entries().iter().position(|&i| i == idx) {
            self.entry_sel = pos;
            return;
        }
        // The entry left the current view (e.g. moved to another group): follow it.
        if self.group_sel != 0 {
            let group = self.doc.group_of(idx);
            if let Some(pos) = self.groups().iter().position(|g| g.start == group) {
                self.group_sel = pos + 1;
            }
            if let Some(pos) = self.visible_entries().iter().position(|&i| i == idx) {
                self.entry_sel = pos;
            }
        }
    }

    fn select_group_start(&mut self, start: Option<usize>) {
        if let Some(pos) = self.groups().iter().position(|g| g.start == start) {
            self.group_sel = pos + 1;
            self.entry_sel = 0;
        }
    }

    // ---- mutations ----

    /// Runs a document change with undo support. No-op changes leave no undo step.
    fn mutate<R>(&mut self, f: impl FnOnce(&mut Document) -> R) -> R {
        let before = self.doc.clone();
        let result = f(&mut self.doc);
        if self.doc != before {
            self.undo.push(before);
            if self.undo.len() > UNDO_LIMIT {
                self.undo.remove(0);
            }
            self.redo.clear();
            self.after_change();
        }
        result
    }

    fn after_change(&mut self) {
        self.marked.clear();
        self.report = validate::analyze(&self.doc);
        self.clamp();
    }

    pub fn undo(&mut self) {
        match self.undo.pop() {
            Some(prev) => {
                self.redo.push(std::mem::replace(&mut self.doc, prev));
                self.after_change();
                self.set_status(t!("status-undo"), StatusKind::Info);
            }
            None => self.set_status(t!("status-nothing-to-undo"), StatusKind::Info),
        }
    }

    pub fn redo(&mut self) {
        match self.redo.pop() {
            Some(next) => {
                self.undo.push(std::mem::replace(&mut self.doc, next));
                self.after_change();
                self.set_status(t!("status-redo"), StatusKind::Info);
            }
            None => self.set_status(t!("status-nothing-to-redo"), StatusKind::Info),
        }
    }

    fn toggle_targets(&mut self, force: bool) {
        let targets = self.targets();
        if targets.is_empty() {
            return;
        }
        if !force
            && targets
                .iter()
                .any(|&i| self.doc.entry(i).is_some_and(Entry::is_protected))
        {
            self.open_dialog(
                DialogKind::ToggleProtected,
                "dialog-protected-title",
                t!("dialog-protected-toggle"),
                &["btn-yes", "btn-cancel"],
            );
            return;
        }
        let current = self.current_entry();
        let enable = !targets.iter().all(|&i| self.doc.entry(i).is_some_and(|e| e.enabled));
        self.mutate(|d| targets.iter().for_each(|&i| d.set_enabled(i, enable)));
        if let Some(i) = current {
            self.select_line(i);
        }
    }

    fn delete_targets(&mut self) {
        let targets = self.targets();
        if targets.is_empty() {
            return;
        }
        let protected = targets
            .iter()
            .any(|&i| self.doc.entry(i).is_some_and(Entry::is_protected));
        let msg = if protected {
            t!("dialog-protected-delete")
        } else {
            t!("dialog-delete-entries", count = targets.len())
        };
        self.open_dialog(
            DialogKind::DeleteEntries(targets),
            "dialog-delete-title",
            msg,
            &["btn-delete", "btn-cancel"],
        );
    }

    // ---- modals ----

    fn open_dialog(&mut self, kind: DialogKind, title: &str, message: String, buttons: &[&str]) {
        self.modal = Some(Modal::Dialog(Dialog {
            kind,
            title: t!(title),
            message,
            buttons: buttons.iter().map(|b| t!(b)).collect(),
            selected: 0,
        }));
    }

    fn open_form(&mut self, editing: Option<usize>) {
        let choices = self.group_choices();
        let entry = editing.and_then(|i| self.doc.entry(i)).cloned();
        let group_start = match editing {
            Some(i) => self.doc.group_of(i),
            None => self.selected_group().and_then(|g| g.start),
        };
        let group = choices.iter().position(|(_, s)| *s == group_start).unwrap_or(0);
        self.modal = Some(Modal::Form(Form {
            editing,
            ip: TextInput::new(entry.as_ref().map_or("127.0.0.1", |e| &e.ip)),
            hosts: TextInput::new(&entry.as_ref().map(|e| e.hosts.join(" ")).unwrap_or_default()),
            comment: TextInput::new(entry.as_ref().and_then(|e| e.comment.as_deref()).unwrap_or_default()),
            group,
            enabled: entry.as_ref().is_none_or(|e| e.enabled),
            focus: if editing.is_some() { 0 } else { 1 },
            error: None,
            top: None,
        }));
    }

    pub fn field_error_text(err: &FieldError) -> String {
        match err {
            FieldError::IpEmpty => t!("error-ip-empty"),
            FieldError::IpInvalid(ip) => t!("error-ip-invalid", ip = ip.as_str()),
            FieldError::HostsEmpty => t!("error-hosts-empty"),
            FieldError::HostInvalid(h) => t!("error-host-invalid", host = h.as_str()),
        }
    }

    fn submit_form(&mut self, form: Form) -> Option<Form> {
        if let Err(e) = validate::validate_fields(&form.ip.value, &form.hosts.value) {
            return Some(Form {
                error: Some(Self::field_error_text(&e)),
                ..form
            });
        }
        let comment = Some(form.comment.value.trim().to_string()).filter(|c| !c.is_empty());
        let entry = Entry::new(
            form.enabled,
            form.ip.value.trim(),
            form.hosts.value.split_whitespace().map(str::to_string).collect(),
            comment,
        );
        let group = self.group_choices().get(form.group).and_then(|(_, s)| *s);
        let idx = match form.editing {
            Some(i) => self.mutate(|d| d.update_entry(i, entry, group)),
            None => self.mutate(|d| d.add_entry(entry, group)),
        };
        self.filter = TextInput::default();
        self.select_line(idx);
        self.focus = Focus::Entries;
        None
    }

    fn submit_group_name(&mut self, mut name_input: NameInput) -> Option<NameInput> {
        let name = name_input.input.value.trim().to_string();
        let current = match name_input.purpose {
            NamePurpose::Rename(start) => self
                .groups()
                .into_iter()
                .find(|g| g.start == Some(start))
                .and_then(|g| g.name),
            NamePurpose::Create => None,
        };
        let error = if name.is_empty() {
            Some(t!("error-group-empty"))
        } else if current.as_deref() != Some(name.as_str()) && self.doc.find_group(&name).is_some() {
            Some(t!("error-group-exists", name = name.as_str()))
        } else {
            None
        };
        if error.is_some() {
            name_input.error = error;
            return Some(name_input);
        }
        let start = match name_input.purpose {
            NamePurpose::Create => self.mutate(|d| d.create_group(&name)),
            NamePurpose::Rename(start) => {
                self.mutate(|d| d.rename_group(start, &name));
                start
            }
        };
        self.select_group_start(Some(start));
        None
    }

    fn open_backups(&mut self) {
        let files = writer::list_backups(&paths::backups_dir());
        if files.is_empty() {
            self.set_status(t!("status-no-backups"), StatusKind::Info);
            return;
        }
        self.modal = Some(Modal::Backups(BackupsView {
            files,
            selected: 0,
            scroll: 0,
        }));
    }

    fn restore_backup(&mut self, path: &Path) {
        match fs::read_to_string(path) {
            Ok(text) => {
                let restored = Document::parse(&text);
                self.mutate(|d| *d = restored);
                let name = path
                    .file_name()
                    .map(|n| n.to_string_lossy().to_string())
                    .unwrap_or_default();
                self.set_status(t!("status-restored", name = name), StatusKind::Info);
            }
            Err(e) => self.set_status(t!("status-error", error = e.to_string()), StatusKind::Error),
        }
    }

    pub fn switch_language(&mut self) {
        let lang = i18n::lang().next();
        i18n::set_lang(lang);
        self.config.language = lang.code().to_string();
        match self.config.save(&self.config_path) {
            Ok(()) => self.set_status(t!("status-language"), StatusKind::Info),
            Err(e) => self.set_status(t!("status-error", error = e.to_string()), StatusKind::Error),
        }
    }

    // ---- saving ----

    fn request_save(&mut self) {
        if !self.is_dirty() {
            self.set_status(t!("status-no-changes"), StatusKind::Info);
            if self.quit_after_save {
                self.quit = true;
            }
            return;
        }
        match fs::read_to_string(&self.path) {
            Ok(on_disk) if on_disk != self.disk_text => {
                self.open_dialog(
                    DialogKind::ExternalChange,
                    "dialog-external-title",
                    t!("dialog-external"),
                    &["btn-reload", "btn-overwrite", "btn-cancel"],
                );
            }
            _ => self.open_diff(),
        }
    }

    /// Diffs against what is on disk right now, so an overwrite shows what it will drop.
    fn open_diff(&mut self) {
        self.modal = Some(Modal::Diff(DiffView {
            old: fs::read_to_string(&self.path).unwrap_or_else(|_| self.saved.to_text()),
            new: self.doc.to_text(),
            scroll: 0,
        }));
    }

    fn reload(&mut self) {
        match fs::read_to_string(&self.path) {
            Ok(text) => {
                let doc = Document::parse(&text);
                self.mutate(|d| *d = doc.clone());
                self.saved = doc;
                self.disk_text = text;
                self.set_status(t!("status-reloaded"), StatusKind::Info);
            }
            Err(e) => self.set_status(t!("status-error", error = e.to_string()), StatusKind::Error),
        }
    }

    fn save(&mut self) {
        let text = self.doc.to_text();
        let backup = match fs::read_to_string(&self.path)
            .map_err(anyhow::Error::from)
            .and_then(|current| writer::backup(&current, &paths::backups_dir(), self.config.backups_keep))
        {
            Ok(p) => p,
            Err(e) => {
                self.quit_after_save = false;
                self.set_status(t!("status-backup-failed", error = format!("{e:#}")), StatusKind::Error);
                return;
            }
        };
        self.last_backup = Some(backup);
        match writer::write(&self.path, &text) {
            Ok(WriteResult::Done) => self.after_save(text),
            Ok(WriteResult::NeedsPassword) => self.pending_sudo = Some(text),
            Err(e) => {
                self.quit_after_save = false;
                self.set_status(t!("status-save-failed", error = format!("{e:#}")), StatusKind::Error);
            }
        }
    }

    fn after_save(&mut self, text: String) {
        self.saved = self.doc.clone();
        self.disk_text = text;
        let backup = self
            .last_backup
            .take()
            .and_then(|p| p.file_name().map(|n| n.to_string_lossy().to_string()))
            .unwrap_or_default();
        let mut msg = t!("status-saved", backup = backup);
        let mut kind = StatusKind::Info;
        if self.config.flush_dns && self.path == *SYSTEM_HOSTS {
            match writer::flush_dns() {
                Some(true) => msg = format!("{msg} · {}", t!("status-dns-flushed")),
                Some(false) => {
                    msg = format!("{msg} · {}", t!("status-dns-failed"));
                    kind = StatusKind::Warn;
                }
                None => {}
            }
        }
        self.set_status(msg, kind);
        if self.quit_after_save {
            self.quit = true;
        }
    }

    fn request_quit(&mut self) {
        if self.is_dirty() {
            self.open_dialog(
                DialogKind::Quit,
                "dialog-quit-title",
                t!("dialog-quit"),
                &["btn-save", "btn-discard", "btn-cancel"],
            );
        } else {
            self.quit = true;
        }
    }
}

/// Main loop. Leaves the alternate screen while `sudo` asks for a password.
pub fn run(mut app: App) -> Result<()> {
    let mut terminal = ratatui::init();
    let result = (|| -> Result<()> {
        while !app.quit {
            terminal.draw(|f| ui::draw(f, &mut app))?;
            if let Event::Key(key) = event::read()?
                && key.kind == KeyEventKind::Press
            {
                app.handle_key(key);
            }
            if let Some(text) = app.pending_sudo.take() {
                ratatui::restore();
                println!("{}", t!("sudo-prompt", path = app.path.display().to_string()));
                let result = writer::write_with_sudo(&app.path, &text);
                terminal = ratatui::init();
                terminal.clear()?;
                match result {
                    Ok(()) => app.after_save(text),
                    Err(_) => {
                        // sudo already printed its own error in the terminal.
                        app.quit_after_save = false;
                        app.set_status(t!("status-sudo-failed"), StatusKind::Error);
                    }
                }
            }
        }
        Ok(())
    })();
    ratatui::restore();
    result
}
