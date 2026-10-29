use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use super::*;

fn ctrl(key: &KeyEvent, c: char) -> bool {
    key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char(c)
}

/// Shared list/scroll movement: returns the new position.
fn step(pos: usize, len: usize, key: &KeyEvent, page: usize) -> Option<usize> {
    let last = len.saturating_sub(1);
    Some(match key.code {
        KeyCode::Up | KeyCode::Char('k') => pos.saturating_sub(1),
        KeyCode::Down | KeyCode::Char('j') => (pos + 1).min(last),
        KeyCode::PageUp => pos.saturating_sub(page),
        KeyCode::PageDown => (pos + page).min(last),
        KeyCode::Home | KeyCode::Char('g') => 0,
        KeyCode::End | KeyCode::Char('G') => last,
        _ => return None,
    })
}

fn scroll(value: &mut u16, key: &KeyEvent) -> bool {
    match step(*value as usize, u16::MAX as usize, key, 20) {
        // `End`/`G` would jump to u16::MAX; the renderer clamps it to the content.
        Some(v) => {
            *value = v as u16;
            true
        }
        None => false,
    }
}

impl App {
    pub fn handle_key(&mut self, key: KeyEvent) {
        self.status = None;
        if ctrl(&key, 'c') {
            self.request_quit();
            return;
        }
        match self.modal.take() {
            Some(modal) => self.handle_modal(modal, key),
            None if self.searching => self.handle_search(key),
            None => self.handle_main(key),
        }
    }

    fn handle_search(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Esc => {
                self.filter = TextInput::default();
                self.searching = false;
            }
            KeyCode::Enter | KeyCode::Down | KeyCode::Up => self.searching = false,
            _ => {
                self.filter.handle(key);
                self.entry_sel = 0;
            }
        }
        self.clamp();
    }

    fn handle_main(&mut self, key: KeyEvent) {
        if ctrl(&key, 'r') {
            return self.redo();
        }
        if ctrl(&key, 's') {
            return self.request_save();
        }
        let shift_move = match key.code {
            KeyCode::Char('K') => Some(false),
            KeyCode::Char('J') => Some(true),
            KeyCode::Up if key.modifiers.contains(KeyModifiers::SHIFT) => Some(false),
            KeyCode::Down if key.modifiers.contains(KeyModifiers::SHIFT) => Some(true),
            _ => None,
        };
        if let Some(down) = shift_move {
            return self.move_selected(down);
        }

        match key.code {
            KeyCode::Char('q') => return self.request_quit(),
            KeyCode::Esc => {
                if !self.filter.value.is_empty() {
                    self.filter = TextInput::default();
                    self.clamp();
                } else if !self.marked.is_empty() {
                    self.marked.clear();
                } else {
                    self.request_quit();
                }
                return;
            }
            KeyCode::Tab | KeyCode::BackTab => {
                self.focus = if self.focus == Focus::Groups {
                    Focus::Entries
                } else {
                    Focus::Groups
                };
                return;
            }
            KeyCode::Left | KeyCode::Char('h') => return self.focus = Focus::Groups,
            KeyCode::Right | KeyCode::Char('l') => return self.focus = Focus::Entries,
            KeyCode::Char('/') => {
                self.searching = true;
                self.focus = Focus::Entries;
                return;
            }
            KeyCode::Char('u') => return self.undo(),
            KeyCode::Char('s') => return self.request_save(),
            KeyCode::Char('r') => return self.modal = Some(Modal::Raw { scroll: 0 }),
            KeyCode::Char('b') => return self.open_backups(),
            KeyCode::Char('?') | KeyCode::F(1) => return self.modal = Some(Modal::Help { scroll: 0 }),
            KeyCode::Char('L') => return self.switch_language(),
            _ => {}
        }

        match self.focus {
            Focus::Groups => self.handle_groups(key),
            Focus::Entries => self.handle_entries(key),
        }
    }

    fn handle_groups(&mut self, key: KeyEvent) {
        let len = self.groups().len() + 1;
        if let Some(pos) = step(self.group_sel, len, &key, self.page_size) {
            if pos != self.group_sel {
                self.group_sel = pos;
                self.entry_sel = 0;
                self.marked.clear();
            }
            return;
        }
        let group = self.selected_group();
        match key.code {
            KeyCode::Enter => self.focus = Focus::Entries,
            KeyCode::Char(' ') => match group {
                Some(g) => self.mutate(|d| d.toggle_group(g.start)),
                None => self.set_status(t!("status-select-group"), StatusKind::Info),
            },
            KeyCode::Char('a') => {
                self.modal = Some(Modal::GroupName(NameInput {
                    purpose: NamePurpose::Create,
                    input: TextInput::default(),
                    error: None,
                }))
            }
            KeyCode::Char('e') => match group.and_then(|g| g.start.zip(g.name)) {
                Some((start, name)) => {
                    self.modal = Some(Modal::GroupName(NameInput {
                        purpose: NamePurpose::Rename(start),
                        input: TextInput::new(&name),
                        error: None,
                    }))
                }
                None => self.set_status(t!("status-group-builtin"), StatusKind::Info),
            },
            KeyCode::Char('d') | KeyCode::Delete => match group.and_then(|g| g.start.map(|s| (s, g))) {
                Some((start, g)) => {
                    let msg = t!(
                        "dialog-delete-group",
                        name = g.name.unwrap_or_default(),
                        count = g.entries.len()
                    );
                    self.open_dialog(
                        DialogKind::DeleteGroup(start),
                        "dialog-delete-title",
                        msg,
                        &["btn-delete-with-entries", "btn-keep-entries", "btn-cancel"],
                    );
                }
                None => self.set_status(t!("status-group-builtin"), StatusKind::Info),
            },
            _ => {}
        }
    }

    fn handle_entries(&mut self, key: KeyEvent) {
        let len = self.visible_entries().len();
        if let Some(pos) = step(self.entry_sel, len, &key, self.page_size) {
            self.entry_sel = pos;
            return;
        }
        let current = self.current_entry();
        match key.code {
            KeyCode::Char(' ') => self.toggle_targets(false),
            KeyCode::Char('a') => self.open_form(None),
            KeyCode::Char('e') | KeyCode::Enter => {
                if let Some(i) = current {
                    self.open_form(Some(i));
                }
            }
            KeyCode::Char('d') | KeyCode::Delete => self.delete_targets(),
            KeyCode::Char('c') => {
                if let Some(i) = current
                    && let Some(new) = self.mutate(|d| d.duplicate(i))
                {
                    self.select_line(new);
                }
            }
            KeyCode::Char('m') => {
                let targets = self.targets();
                if !targets.is_empty() {
                    self.modal = Some(Modal::GroupPicker(GroupPicker { targets, selected: 0 }));
                }
            }
            KeyCode::Char('v') => {
                if let Some(i) = current {
                    if !self.marked.remove(&i) {
                        self.marked.insert(i);
                    }
                    self.entry_sel = (self.entry_sel + 1).min(len.saturating_sub(1));
                }
            }
            _ => {}
        }
    }

    fn move_selected(&mut self, down: bool) {
        match self.focus {
            Focus::Entries => {
                if let Some(i) = self.current_entry() {
                    let new = self.mutate(|d| d.move_entry(i, down));
                    self.select_line(new);
                }
            }
            Focus::Groups => {
                if let Some(start) = self.selected_group().and_then(|g| g.start) {
                    let new = self.mutate(|d| d.move_group(start, down));
                    self.select_group_start(Some(new));
                }
            }
        }
    }

    fn handle_modal(&mut self, modal: Modal, key: KeyEvent) {
        let next = match modal {
            Modal::Form(form) => self.handle_form(form, key),
            Modal::Dialog(dialog) => self.handle_dialog(dialog, key),
            Modal::Diff(mut diff) => match key.code {
                KeyCode::Enter | KeyCode::Char('y') => {
                    self.save();
                    None
                }
                KeyCode::Esc | KeyCode::Char('n') | KeyCode::Char('q') => {
                    self.quit_after_save = false;
                    None
                }
                _ => {
                    scroll(&mut diff.scroll, &key);
                    Some(Modal::Diff(diff))
                }
            },
            Modal::Raw { scroll: mut s } => match key.code {
                KeyCode::Esc | KeyCode::Char('q') | KeyCode::Char('r') => None,
                _ => {
                    scroll(&mut s, &key);
                    Some(Modal::Raw { scroll: s })
                }
            },
            Modal::Help { scroll: mut s } => match key.code {
                KeyCode::Esc | KeyCode::Char('q') | KeyCode::Char('?') => None,
                _ => {
                    scroll(&mut s, &key);
                    Some(Modal::Help { scroll: s })
                }
            },
            Modal::Backups(mut view) => match key.code {
                KeyCode::Esc | KeyCode::Char('q') | KeyCode::Char('b') => None,
                KeyCode::Enter => {
                    let path = view.files[view.selected].clone();
                    self.restore_backup(&path);
                    None
                }
                KeyCode::PageUp | KeyCode::PageDown => {
                    scroll(&mut view.scroll, &key);
                    Some(Modal::Backups(view))
                }
                _ => {
                    if let Some(pos) = step(view.selected, view.files.len(), &key, 10)
                        && pos != view.selected
                    {
                        view.selected = pos;
                        view.scroll = 0;
                    }
                    Some(Modal::Backups(view))
                }
            },
            Modal::GroupName(mut name) => match key.code {
                KeyCode::Esc => None,
                KeyCode::Enter => self.submit_group_name(name).map(Modal::GroupName),
                _ => {
                    name.input.handle(key);
                    name.error = None;
                    Some(Modal::GroupName(name))
                }
            },
            Modal::GroupPicker(mut picker) => {
                let choices = self.group_choices();
                match key.code {
                    KeyCode::Esc => None,
                    KeyCode::Enter => {
                        let target = choices.get(picker.selected).and_then(|(_, s)| *s).and_then(|s| {
                            self.groups()
                                .into_iter()
                                .find(|g| g.start == Some(s))
                                .and_then(|g| g.name)
                        });
                        let new = self.mutate(|d| d.move_entries_to_group(&picker.targets, target.as_deref()));
                        if let Some(&first) = new.first() {
                            self.select_line(first);
                        }
                        None
                    }
                    _ => {
                        if let Some(pos) = step(picker.selected, choices.len(), &key, 10) {
                            picker.selected = pos;
                        }
                        Some(Modal::GroupPicker(picker))
                    }
                }
            }
        };
        // A handler may have opened a new modal (e.g. a dialog after save); keep it.
        if self.modal.is_none() {
            self.modal = next;
        }
    }

    fn handle_form(&mut self, mut form: Form, key: KeyEvent) -> Option<Modal> {
        match key.code {
            KeyCode::Esc => return None,
            KeyCode::Enter => return self.submit_form(form).map(Modal::Form),
            // In the hosts field ↑↓ move between names first, then on to the next field.
            KeyCode::Up | KeyCode::Down if form.focus == 1 && form.hosts.move_line(key.code == KeyCode::Up) => {}
            KeyCode::Tab | KeyCode::Down => form.focus = (form.focus + 1) % Form::FIELDS,
            KeyCode::BackTab | KeyCode::Up => form.focus = (form.focus + Form::FIELDS - 1) % Form::FIELDS,
            _ => {
                let groups = self.group_choices().len();
                match form.focus {
                    0 => _ = form.ip.handle(key),
                    1 => _ = form.hosts.handle(key),
                    2 => _ = form.comment.handle(key),
                    3 => match key.code {
                        KeyCode::Left => form.group = (form.group + groups - 1) % groups,
                        KeyCode::Right | KeyCode::Char(' ') => form.group = (form.group + 1) % groups,
                        _ => {}
                    },
                    _ => {
                        if matches!(key.code, KeyCode::Char(' ') | KeyCode::Left | KeyCode::Right) {
                            form.enabled = !form.enabled;
                        }
                    }
                }
                // Re-validate live once the user has typed something.
                form.error = if form.ip.value.is_empty() && form.hosts.value.is_empty() {
                    None
                } else {
                    validate::validate_fields(&form.ip.value, &form.hosts.value)
                        .err()
                        .filter(|e| !matches!(e, FieldError::HostsEmpty))
                        .map(|e| Self::field_error_text(&e))
                };
            }
        }
        Some(Modal::Form(form))
    }

    fn handle_dialog(&mut self, mut dialog: Dialog, key: KeyEvent) -> Option<Modal> {
        let n = dialog.buttons.len();
        let choice = match key.code {
            KeyCode::Left | KeyCode::BackTab | KeyCode::Char('h') => {
                dialog.selected = (dialog.selected + n - 1) % n;
                None
            }
            KeyCode::Right | KeyCode::Tab | KeyCode::Char('l') => {
                dialog.selected = (dialog.selected + 1) % n;
                None
            }
            KeyCode::Enter => Some(dialog.selected),
            KeyCode::Esc => Some(n - 1),
            _ => None,
        };
        let Some(choice) = choice else {
            return Some(Modal::Dialog(dialog));
        };
        let cancel = choice == n - 1;
        match dialog.kind {
            _ if cancel => self.quit_after_save = false,
            DialogKind::DeleteEntries(targets) => {
                self.mutate(|d| d.delete_entries(&targets));
            }
            DialogKind::ToggleProtected => self.toggle_targets(true),
            DialogKind::DeleteGroup(start) => {
                let keep = choice == 1;
                // Select the group above the deleted one.
                let above = self.group_sel.saturating_sub(1).max(1);
                self.mutate(|d| d.delete_group(start, keep));
                self.group_sel = above;
                self.clamp();
            }
            DialogKind::Quit => match choice {
                0 => {
                    self.quit_after_save = true;
                    self.request_save();
                }
                _ => self.quit = true,
            },
            DialogKind::ExternalChange => match choice {
                0 => {
                    self.quit_after_save = false;
                    self.reload();
                }
                _ => self.open_diff(),
            },
        }
        None
    }
}
