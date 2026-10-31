# hostkeep — TUI editor for the hosts file

> Status: **approved, MVP v0.1 implemented** (see section 7)
> Language: Rust · Interface: full-screen terminal UI

---

## 1. Goal

Make it easy to view, add, edit, enable and disable entries in `/etc/hosts` (and similar files)
without breaking hand-made formatting and without fear of corrupting the file.

**Core principles**

1. **Lossless.** The file is parsed so that every comment, blank line, indentation and line order
   is preserved. Only the lines the user changed are rewritten.
2. **Safe.** A backup before every write; atomic writes; a diff is shown before saving.
3. **Compatible.** The file stays a plain hosts file: it can be edited by hand or by other tools,
   and `hostkeep` picks the changes up correctly.

---

## 2. Data model and file format

### 2.1 Line types

| Type | Example | Shown as |
|---|---|---|
| Active entry | `127.0.0.1  api.local  www.api.local  # backend` | ● enabled |
| Disabled entry | `# 127.0.0.1  api.local` | ○ disabled |
| Comment | `# Any text` | not an entry (visible in the raw view) |
| Blank line | | preserved |
| Group marker | `# @group Docker` / `# @end` | group header |

A **disabled entry** is a commented-out line whose content after `#` parses strictly as
`IP hostname...`. Every other commented line is a plain comment.

One line may carry several host names — that is one entry (one IP plus a list of names).
A trailing comment (`# backend`) is shown as the entry's description.

### 2.2 Groups

Groups are blocks between markers in the file itself:

```
# @group Docker
127.0.0.1   postgres.local
127.0.0.1   redis.local
# 127.0.0.1 kafka.local
# @end
```

- Entries outside markers belong to the virtual group **"Ungrouped"**.
- System entries (`localhost`, `broadcasthost`, `ip6-localhost`, …) are marked `◆` as protected:
  disabling or deleting them asks for confirmation.
- Markers are ordinary comments, so other programs simply ignore them.

---

## 3. Features

### 3.1 MVP

**Viewing and navigation**
- F-01 Group list (left panel) with `enabled/total` counters.
- F-02 Table of the selected group's entries: state, IP, hosts, description.
- F-03 "All" group — a flat list of every entry.
- F-04 Keyboard navigation (arrows and vim keys `hjkl`, `g`/`G`, PgUp/PgDn); mouse optional.
- F-05 Search/filter `/` by IP, host name or description (live filtering).
- F-06 Raw view — the hosts file as text with syntax highlighting (read-only).

**Managing entries**
- F-07 Enable/disable an entry — `Space`.
- F-08 Enable/disable a whole group — `Space` on the group.
- F-09 Add an entry — form: IP, hosts (space-separated), description, group.
- F-10 Edit an entry — the same form.
- F-11 Delete an entry — with confirmation.
- F-12 Duplicate an entry (e.g. to change only the IP).
- F-13 Move an entry to another group, or up/down within its group.
- F-14 Multi-select (`v`) for bulk actions: enable, disable, delete, move.

**Managing groups**
- F-15 Create, rename, delete a group (deleting either drops its entries or moves them to "Ungrouped").
- F-16 Reorder groups.

**Validation and hints**
- F-17 IPv4/IPv6 and host name (RFC 1123) validation right in the form, with an error message.
- F-18 Conflict warning: the same host enabled with different IPs (⚠ in the table).
- F-19 Exact duplicate warning.

**Saving and safety**
- F-20 Changes accumulate in memory; a `● modified` indicator in the header.
- F-21 A diff screen (before/after) and confirmation before saving.
- F-22 A backup before every write to `~/.local/share/hostkeep/backups/hosts-YYYYMMDD-HHMMSS`;
  the newest N are kept (20 by default).
- F-23 Atomic write: temporary file → `rename`; permissions and owner are preserved.
- F-24 External change detection: if the file changed on disk since it was opened, warn and offer
  to reload or overwrite.
- F-25 Undo/redo (`u` / `Ctrl+r`) within the session.
- F-26 Restore from a backup — a backup list with a diff preview.
- F-27 DNS cache flush after saving (configurable):
  macOS `dscacheutil -flushcache && killall -HUP mDNSResponder`,
  Linux `resolvectl flush-caches` (when systemd-resolved is present).

**Permissions**
- F-28 The tool runs as a regular user and reads the file without root.
  Without write permission the header shows `🔒 sudo needed to save`. On save:
  1. first `sudo -n` (no password) — if sudo still remembers the password (5 minutes by default),
     the save happens silently without leaving the TUI;
  2. otherwise the TUI steps aside for a moment, the standard `sudo` prompt appears
     (Touch ID works too if enabled for sudo on macOS), then the interface comes back.
  The tool never reads or stores the password itself.
  Running `sudo hostkeep` is supported too; config and backups then still come from the real
  user's home directory (`SUDO_USER`).

**Other**
- F-29 Help screen `?` listing every key.
- F-30 `--file <path>` — edit any file (for tests and safe experiments).
- F-31 Config `~/.config/hostkeep/config.toml`: hosts path, number of backups, DNS flush, theme, language.

**Localization (i18n)**
- F-32 All interface texts (headers, hints, messages, validation errors, help, `--help`) live in
  translation files; no user-facing strings in code.
- F-33 Built-in languages: **Ukrainian** (`uk`) and **English** (`en`). Translations are compiled
  into the binary — nothing extra to install.
- F-34 Language choice, by priority: `--lang uk|en` → `language` in the config →
  system locale (`LC_ALL`/`LC_MESSAGES`/`LANG`, macOS system settings) → `en` as fallback.
- F-35 Switch the language on the fly — `L` (saved to the config).
- F-36 Correct plural forms (uk: 1 запис / 2 записи / 5 записів).
- F-37 The hosts file content does not depend on the language: markers `# @group` / `# @end`
  are always the same; "Ungrouped" / "Без групи" is only an interface label.
- F-38 Adding a language means adding one file, `locales/<code>/main.ftl`.

### 3.2 Later (v1.x, optional)

- L-01 **Profiles** — saved sets of enabled groups (e.g. `work`, `home`), switched in one action.
- L-02 **CLI commands without the TUI** for scripts: `hostkeep list`, `hostkeep on <host|group>`,
  `hostkeep off ...`, `hostkeep add <ip> <host>...`.
- L-03 Import entries from a file / URL (block lists), export a group to a file.
- L-04 Resolution check: what the system actually returns for the selected host.
- L-05 Windows support (`C:\Windows\System32\drivers\etc\hosts`, UAC instead of sudo).
- L-06 Light theme / custom colors.

### 3.3 Out of scope

- Managing DNS servers, `/etc/resolver/*`, dnsmasq.
- Syncing between machines.

---

## 4. Interface design

Minimum terminal size: 80×24. The layout adapts: below 70 columns only the focused panel is shown
(switch with `Tab`).

### 4.1 Main screen

```
 hostkeep ─ /etc/hosts                                               ● modified  EN
┌ Groups ─────────────┐┌ Docker ──────────────────────────────────── 2/3 enabled ┐
│  All           9/14 ││       IP          Hosts                Description      │
│  Ungrouped      3/3 ││   ●   127.0.0.1   postgres.local       db               │
│▶ Docker         2/3 ││▶  ●   127.0.0.1   redis.local          cache            │
│  Work VPN       3/5 ││   ○   127.0.0.1   kafka.local                           │
│  Blocking       1/3 ││                                                         │
│                     ││                                                         │
└─────────────────────┘└─────────────────────────────────────────────────────────┘
 ⚠ api.local is enabled with several IPs: 10.0.0.5, 127.0.0.1
 Space toggle  a add  e edit  d delete  / search  s save  ? help
```

- `●` green — enabled, `○` gray — disabled, `⚠` yellow — conflict, `≡` — duplicate, `◆` — system entry.
- The active panel has a bright border, the inactive one a dim border.
- The status line (above the key hints) shows warnings, results of actions
  ("Saved, backup: hosts-20261029-114200") and errors in red.
- The bottom line shows context-sensitive key hints (they change with the panel/mode).
- An entry with more names than fit shows `first-name +N`; the full list of the selected entry
  appears in the status line.

### 4.2 Add / edit form (modal)

```
┌ New entry ───────────────────────────────────────────────────┐
│                                                              │
│ IP address   [127.0.0.1                                    ] │
│ Hosts        [api.local                                    ] │
│              [www.api.local                                ] │
│ Description  [local backend                                ] │
│ Group        ◀ Docker ▶                                      │
│ State        [x] enabled                                     │
│                                                              │
│ ✗ "www.api..local" is not a valid host name                  │
│                                                              │
│       Enter Save  Tab next field  Esc Cancel                 │
└──────────────────────────────────────────────────────────────┘
```

The hosts field shows one name per row and grows with them (1 to 8 rows, then scrolls).

### 4.3 Save confirmation (diff)

```
┌ Save changes to /etc/hosts? ──────────────────────────────────────────────┐
│  12   # @group Docker                                                     │
│  13   127.0.0.1   postgres.local  # db                                    │
│  14 - # 127.0.0.1 redis.local     # cache                                 │
│  14 + 127.0.0.1   redis.local     # cache                                 │
│  15 + 127.0.0.1   kafka.local                                             │
│  16   # @end                                                              │
│                                                                           │
│  +2  −1   A backup will be created. sudo will be required.                │
│        Enter/y Save  Esc/n Back  ↑↓ PgUp PgDn scroll                      │
└───────────────────────────────────────────────────────────────────────────┘
```

`+` lines are green, `−` lines red.

### 4.4 Other screens

- **Search** — a `/query` field in the bottom line, the table filters live; `Esc` clears.
- **Raw view** (`r`) — the whole file with line numbers and highlighting: IPs blue, hosts white,
  comments gray, group markers magenta, invalid lines red.
- **Backups** (`b`) — backup list on the left, diff against the current state on the right,
  `Enter` restores.
- **Help** (`?`) — a table of every key.
- **Quit with unsaved changes** — a "Save / Don't save / Cancel" dialog.

### 4.5 Keys

| Key | Action |
|---|---|
| `↑↓` / `jk` | move in the list |
| `←→` / `hl`, `Tab` | switch panels |
| `g` / `G`, `PgUp` / `PgDn` | first / last, page up / down |
| `Space` | enable/disable an entry or a group |
| `a` | add an entry (a group in the groups panel) |
| `e` / `Enter` | edit an entry, rename a group |
| `d` / `Del` | delete |
| `c` | duplicate |
| `m` | move to a group |
| `J` / `K`, `Shift+↑↓` | move down / up |
| `v` | multi-select |
| `/` | search |
| `u` / `Ctrl+r` | undo / redo |
| `s` / `Ctrl+s` | save (via diff) |
| `r` | raw view |
| `b` | backups |
| `?` | help |
| `L` | switch language (uk ⇄ en) |
| `q` / `Esc` | quit / close a window |

In the form: `Tab` / `Shift+Tab` move between fields; in the hosts field `↑↓` move between names
first, `Space` starts a new name; `Alt+←→` (or `Ctrl+←→`) jump by word in every text field.

---

## 5. Technical approach

| Aspect | Choice |
|---|---|
| TUI | `ratatui` (with its bundled `crossterm`) |
| CLI arguments | `clap` |
| Config | `serde` + `toml` |
| Paths | XDG variables via `std::env`, `libc` for the `SUDO_USER` home |
| Diff | `similar` |
| i18n | Project Fluent: `fluent-templates` (compiled-in translations, CLDR plural rules) + `sys-locale` |
| Errors | `anyhow` |
| Atomic write | `tempfile` |
| Platforms | macOS, Linux |

**Module layout**

```
src/
  main.rs          — arguments, startup
  config.rs        — config file
  paths.rs         — config/backup locations, sudo-aware ownership
  i18n.rs          — language choice, t!("key", args) macro
  hosts/
    parser.rs      — lossless parser: file line ↔ Line enum
    model.rs       — Document, Entry, Group; operations (toggle, add, move…)
    validate.rs    — IP, host name, conflicts, duplicates
    writer.rs      — atomic write, sudo, backups, DNS flush
  app/
    mod.rs         — UI state, undo/redo, saving, main loop
    keys.rs        — key handling → actions
    input.rs       — text field
  ui/
    mod.rs         — layout, theme, shared helpers
    main_view.rs   — header, panels, status line, key bar
    modals.rs      — form, dialogs, diff, raw view, backups, help
locales/
  en/main.ftl
  uk/main.ftl
```

Translation file example (`locales/uk/main.ftl`):

```ftl
group-ungrouped = Без групи
status-saved = Збережено, бекап: { $backup }
dialog-delete-entries = Видалити { $count ->
    [one] { $count } запис
    [few] { $count } записи
   *[many] { $count } записів
}?
```

The layout does not rely on string lengths: column and window widths are computed from the actual
translation, so longer English or Ukrainian texts do not break it.

**Testing:** the parser is covered by round-trip tests (parse → serialize = the same file byte for
byte); model operations by unit tests; all manual checks run via `--file` on a copy, never on the
real `/etc/hosts`. i18n tests: the `en` and `uk` key sets match, and every key used in code exists.

---

## 6. Decisions

1. **Platforms:** macOS + Linux. Windows later (L-05).
2. **Permissions:** run without root, `sudo` only on save, first without a password (F-28).
3. **Groups:** `# @group Name` / `# @end` markers inside the hosts file itself, no separate metadata
   file — so groups don't drift after the hosts file is edited by hand.
4. **Scope:** MVP only (F-01…F-38); the "Later" list is not included.
5. **Interface language:** i18n, Ukrainian + English (F-32…F-38).
6. **Name:** `hostkeep` (renamed from the working title `hostsed`, which clashes with an existing
   hosts editor on PyPI). It lives in one place (`Cargo.toml`, read through `CARGO_PKG_NAME`).
7. **No managed block:** hostkeep edits the whole file rather than only a marked section; the file
   stays the single source of truth, and protected entries plus backups cover the safety side.

---

## 7. Implementation status (v0.1)

F-01…F-38 are implemented, except for the items below. 44 automated tests (parser round-trip,
model, validation, writing/backups, config, i18n, form helpers, CLI commands), plus manual checks of
every screen through a pty and of every CLI command on a copy of a hosts file.

**Deviations from the spec**
- F-04: no mouse support (it was optional) — mouse capture breaks text selection in the terminal.
- F-31: there is no `theme` key in the config — only the dark theme exists; a light one is L-06.
- F-32: argument parsing errors from clap (`invalid value …`) stay in English;
  `--help` is fully translated.
- Backups: `$XDG_DATA_HOME/hostkeep/backups` (default `~/.local/share/hostkeep/backups`),
  config: `$XDG_CONFIG_HOME/hostkeep/config.toml` — the same on macOS and Linux.
- System entries are marked `◆` (not 🔒 — emoji take different widths in different terminals).
- The diff before saving is computed against the current disk content, not the state at opening,
  so "Overwrite" shows which external changes will be lost.
- While the interactive `sudo` runs, hostkeep ignores Ctrl+C, so cancelling the password prompt does
  not kill the app with unsaved changes.

**Additions after v0.1**
- Entries with many names: the table shows as many names as fit, then `+N`; the full list of the
  selected entry is in the status line. The form's hosts field shows one name per row and grows with
  them from 1 to 8 rows (the form opens centered, then only grows downwards); `↑↓` move between
  names (from the first/last row — to the neighbouring field), `Space` starts a new name,
  `Alt/Ctrl+←→` jump by word in every text field.
- Diff, raw file and backups views take the whole screen below the header (a margin showed clipped
  bits of the main view). On narrow terminals the hosts column gets priority: the group column in
  "All" hides when hosts would get under 24 characters, the description takes at most 2/5 and no
  more than its longest text, and cut text ends with `…`.
- CLI commands (section 8): `list`, `groups`, `on`, `off`, `toggle`, `add`, `rm`, `completions`.

---

## 8. CLI commands (v0.2)

L-02: the same file operations without the TUI, for scripts, aliases and quick one-off changes.
`hostkeep` with no command still opens the TUI.

### 8.1 Commands

```
hostkeep                                   open the TUI (as now)
hostkeep list   [GROUP] [--enabled | --disabled] [--json]
hostkeep groups [--json]
hostkeep on     TARGET...  [--group | --host]
hostkeep off    TARGET...  [--group | --host] [--force]
hostkeep toggle TARGET...  [--group | --host] [--force]
hostkeep add    IP HOST... [-g GROUP] [-c COMMENT] [--disabled]
hostkeep rm     HOST...    [--force]
hostkeep rm     --group GROUP... [--keep-entries] [--force]
hostkeep completions bash|zsh|fish
```

Global options for every command: `--file PATH`, `--lang uk|en`, `--dry-run` (print the diff, write
nothing), `-q/--quiet` (no output on success).

**TARGET** is a group name or a host name, matched exactly and case-insensitively. A name that is
both a group and a host is an error that asks for `--group` or `--host`.

### 8.2 Examples

```
$ hostkeep list
Ungrouped
  ● ◆ 127.0.0.1        localhost
  ● ◆ 255.255.255.255  broadcasthost
Docker                                   3/4
  ●   127.0.0.1        postgres.local    # main database
  ○   127.0.0.1        kafka.local
Work VPN                                 2/3
  ●   10.8.0.12        gitlab.corp.internal
  ○   10.8.0.20        grafana.corp.internal

$ hostkeep groups
Ungrouped   3/3
Docker      3/4
Staging     0/2
Work VPN    2/3

$ hostkeep off "Work VPN"
Disabled group "Work VPN" (2 entries). Backup: hosts-20261031-101500

$ hostkeep on kafka.local
Enabled kafka.local (127.0.0.1)

$ hostkeep on kafka.local
kafka.local is already enabled — nothing to save

$ hostkeep add 127.0.0.1 api.local admin.local -g Docker -c "new backend"
Added 127.0.0.1 api.local admin.local to "Docker"

$ hostkeep rm admin.local
Removed admin.local from 127.0.0.1 api.local admin.local

$ hostkeep off localhost
localhost is a system entry; disabling it can break your system. Repeat with --force.

$ hostkeep off Docker --dry-run
--- /etc/hosts
+++ /etc/hosts (new)
@@ …
-127.0.0.1       postgres.local  # main database
+# 127.0.0.1     postgres.local  # main database
```

### 8.3 Behaviour

- **Output.** `list` and `groups` print the table above. Colors (green ● / grey ○, as in the TUI)
  only when stdout is a terminal and `NO_COLOR` is not set; piped output is plain text. `--json`
  prints an array of `{ip, hosts, comment, enabled, group, protected}` / `{name, enabled, total}`.
  The ◆ and ●/○ markers stay in plain output so it is still readable; scripts use `--json`.
- **Saving** is the same as in the TUI: a backup first, then an atomic write. If the file is not
  writable: `sudo -n cp`, otherwise the regular `sudo` prompt in the same terminal. Without a
  terminal (cron, CI) it fails with a hint to run `sudo hostkeep …`. After saving `/etc/hosts`, the
  DNS cache is flushed when `flush_dns = true`.
- **No change, no write.** If every target is already in the requested state, nothing is written,
  no backup is made, and the exit code is 0.
- **Protected entries** (`localhost` etc.) need `--force` for `off` and `rm`, matching the TUI's
  confirmation dialog. `on` never needs it.
- **`add`** validates the IP and host names like the form does. A missing group (`-g`) is created at
  the end of the file. A host that is already enabled with a different IP is a warning, not an error
  (the TUI shows the same as ⚠).
- **`on HOST` / `toggle HOST`** with the host in several entries (e.g. the same name with different
  IPs in different groups) is an error: enabling all of them would create a conflict, so the message
  lists the entries and suggests switching their group instead. `off HOST` disables all of them.
  An entry with several names is switched as a whole line; the message shows the full entry.
- **`toggle`** flips a host entry; for a group it works like `Space` in the TUI: enable all unless all
  are enabled, then disable all. Disabling a protected entry needs `--force`, as with `off`.
- **`rm HOST`** removes that name from its entries; an entry left with no names is deleted.
- **`rm --group NAME`** deletes the group with its entries; `--keep-entries` removes only the markers,
  and the entries become ungrouped (the same choice as in the TUI dialog). Unlike `on`/`off`, `rm`
  never guesses: without `--group` its arguments are host names, so a host that shares a name with a
  group can never delete the whole group. Protected entries inside need `--force` unless kept.
- Several targets in one command are resolved first: one unknown or ambiguous target fails the whole
  command before anything changes, and all changes are saved in one write with one backup.
- **Exit codes:** 0 success (including "nothing to change"), 1 error (unknown target, invalid input,
  write failure), 2 wrong arguments (clap).
- **Messages** are translated (uk/en) like the TUI; `--json` keys and values are not.

### 8.4 Decisions

1. Targets are auto-detected as a group or a host; `--group`/`--host` only resolve a clash.
2. `toggle` is included.
3. `rm --group` is included, with `--keep-entries` and an explicit `--group` (see above).
4. `completions` prints static completions for commands and flags (clap_complete); completing group
   and host names from the file is left for later.
5. No `list --grep`: `grep` on the plain output does it.
