## Command line

cli-about = Terminal editor for the hosts file
cli-file = Edit this file instead of the configured hosts file
cli-lang = Interface language: uk or en
cli-help = Show this help
cli-version = Show version
cli-usage = Usage
cli-options = Options
cli-commands = Commands
cli-arguments = Arguments
cli-dry-run = Show the changes as a diff without saving
cli-quiet = Print nothing on success
cmd-list = List entries, grouped
cmd-groups = List groups with enabled/total counts
cmd-on = Enable groups or hosts
cmd-off = Disable groups or hosts
cmd-toggle = Switch groups or hosts to the opposite state
cmd-add = Add an entry
cmd-rm = Remove host names, or whole groups with --group
cmd-completions = Print a shell completion script
arg-list-group = Only this group
arg-enabled = Only enabled entries
arg-disabled = Only disabled entries
arg-json = Print JSON
arg-targets = Group or host names
arg-as-group = Treat the names as groups
arg-as-host = Treat the names as hosts
arg-force = Allow changing system entries (localhost etc.)
arg-ip = IP address
arg-hosts = Host names
arg-add-group = Add to this group (created if missing)
arg-comment = Description
arg-add-disabled = Add the entry disabled
arg-rm-names = Host names (group names with --group)
arg-rm-group = Remove the named groups with their entries
arg-keep-entries = Keep the group's entries, only remove the group
arg-shell = Shell: bash, zsh, fish, elvish or powershell
cli-entries = { $count ->
    [one] 1 entry
   *[other] { $count } entries
}
cli-group-on = Enabled group "{ $name }" ({ cli-entries })
cli-group-off = Disabled group "{ $name }" ({ cli-entries })
cli-group-already-on = Group "{ $name }" is already enabled
cli-group-already-off = Group "{ $name }" is already disabled
cli-host-on = Enabled { $entry }
cli-host-off = Disabled { $entry }
cli-host-already-on = { $host } is already enabled
cli-host-already-off = { $host } is already disabled
cli-added = Added { $entry }
cli-added-to = Added { $entry } to "{ $group }"
cli-add-exists = { $entry } already exists
cli-removed-host = Removed { $host } from { $entry }
cli-removed-entry = Removed { $entry }
cli-group-removed = Removed group "{ $name }" ({ cli-entries })
cli-group-unwrapped = Removed group "{ $name }"; { cli-entries } kept as ungrouped
cli-warn-conflict = Warning: { $host } is now enabled with different addresses: { $ips }
cli-error-not-found = No group or host named "{ $name }". See: hostkeep list
cli-error-no-group = No group named "{ $name }". See: hostkeep groups
cli-error-no-host = No host named "{ $name }". See: hostkeep list
cli-error-rm-group-hint = No host named "{ $name }", but there is such a group. To remove the group: hostkeep rm --group "{ $name }"
cli-error-ambiguous = "{ $name }" is both a group and a host. Add --group or --host.
cli-error-host-many = { $host } is in several entries; enabling them all would conflict. Switch the group that holds the right one, or edit in the TUI:
cli-error-protected-off = { $host } is a system entry; disabling it can break your system. Repeat with --force.
cli-error-protected-rm = { $host } is a system entry; removing it can break your system. Repeat with --force.
cli-error-protected-group = Group "{ $name }" contains a system entry (localhost etc.). Repeat with --force, or use --keep-entries.
cli-error-sudo-no-tty = Administrator rights are needed to write { $path }, but there is no terminal to ask for a password. Run the command with sudo.
cli-error-sudo-failed = Not saved: sudo did not succeed.
error-config = Cannot read config { $path }: { $error }. Using defaults.
error-not-terminal = hostkeep needs an interactive terminal.
error-open = Cannot open { $path }: { $error }
sudo-prompt = Administrator rights are needed to write { $path }.

## Header and panels

header-modified = modified
header-needs-sudo = sudo needed to save
panel-groups = Groups
group-all = All
group-ungrouped = Ungrouped
enabled-of = { $enabled }/{ $total } enabled
entries-count = { $count ->
    [one] { $count } entry
   *[other] { $count } entries
}
col-ip = IP
col-hosts = Hosts
col-comment = Description
col-group = Group
empty-group = No entries yet — press a to add one
empty-filter = Nothing matches the search

## Bottom bar

key-toggle = toggle
key-toggle-group = toggle group
key-add = add
key-edit = edit
key-delete = delete
key-search = search
key-save = save
key-help = help
key-new-group = new group
key-rename = rename
key-move = move
key-apply = apply
key-clear = clear
key-scroll = scroll
key-next-field = next field

## Status line

status-saved = Saved, backup: { $backup }
status-dns-flushed = DNS cache flushed
status-dns-failed = could not flush DNS cache
status-no-changes = No changes to save
status-save-failed = Save failed: { $error }
status-sudo-failed = Not saved: sudo did not succeed. Your changes are still here.
status-backup-failed = Backup failed, nothing was written: { $error }
status-error = Error: { $error }
status-undo = Undone
status-redo = Redone
status-nothing-to-undo = Nothing to undo
status-nothing-to-redo = Nothing to redo
status-reloaded = Reloaded from disk
status-restored = Restored from { $name } — press s to save
status-no-backups = No backups yet
status-language = Language: English
status-select-group = Select a group to toggle it
status-group-builtin = This group is built in and cannot be changed
status-marked = { $count ->
    [one] { $count } entry selected
   *[other] { $count } entries selected
} — Space, d, m act on all of them, Esc clears
warn-conflict = { $host } is enabled with several IPs: { $ips }
warn-duplicate = This entry is a duplicate of an earlier one
warn-conflicts-total = { $count ->
    [one] { $count } host is enabled with several IPs
   *[other] { $count } hosts are enabled with several IPs
}
warn-invalid-lines = { $count ->
    [one] { $count } line is not a valid entry (see r)
   *[other] { $count } lines are not valid entries (see r)
}

## Form

form-add-title = New entry
form-edit-title = Edit entry
form-ip = IP address
form-hosts = Hosts
form-comment = Description
form-group = Group
form-state = State
form-enabled = enabled
error-ip-empty = Enter an IP address
error-ip-invalid = "{ $ip }" is not a valid IP address
error-hosts-empty = Enter at least one host name
error-host-invalid = "{ $host }" is not a valid host name
group-new-title = New group
group-rename-title = Rename group
error-group-empty = Enter a group name
error-group-exists = Group "{ $name }" already exists
picker-title = Move to group

## Dialogs

btn-yes = Yes
btn-ok = OK
btn-cancel = Cancel
btn-save = Save
btn-back = Back
btn-close = Close
btn-delete = Delete
btn-discard = Don't save
btn-reload = Reload
btn-overwrite = Overwrite
btn-restore = Restore
btn-delete-with-entries = Delete with entries
btn-keep-entries = Keep entries
dialog-delete-title = Delete
dialog-delete-entries = Delete { $count ->
    [one] { $count } entry
   *[other] { $count } entries
}?
dialog-delete-group = Delete group "{ $name }" ({ $count ->
    [one] { $count } entry
   *[other] { $count } entries
})? Entries can be deleted too or kept as ungrouped.
dialog-protected-title = System entry
dialog-protected-toggle = This includes a system entry (localhost etc.). Disabling it can break your system. Continue?
dialog-protected-delete = This includes a system entry (localhost etc.). Deleting it can break your system. Delete anyway?
dialog-quit-title = Unsaved changes
dialog-quit = Save changes before quitting?
dialog-external-title = File changed on disk
dialog-external = The hosts file was changed by another program since it was opened. Reload it (your changes are kept in undo) or overwrite it?

## Diff, raw, backups

diff-title = Save changes to { $path }?
diff-backup-note = A backup will be created.
diff-sudo-note = sudo will be required.
raw-title = Raw file (read-only)
backups-title = Backups
backups-diff-title = Current → backup
backups-identical = Identical to the current state

## Help

help-title = Keys
help-move = Move up / down
help-panels = Switch panels
help-jump = First / last, page up / down
help-toggle = Enable / disable entry or group
help-add = Add entry (group in the groups panel)
help-edit = Edit entry, rename group
help-delete = Delete
help-duplicate = Duplicate entry
help-move-group = Move entry to another group
help-reorder = Move entry or group up / down
help-mark = Select several entries
help-search = Search
help-undo = Undo / redo
help-save = Save (with preview)
help-raw = View raw file
help-backups = Backups
help-language = Switch language (uk ⇄ en)
help-help = This help
help-quit = Quit
