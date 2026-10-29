use std::borrow::Cow;

use super::parser::{format_entry, format_group_end, format_group_start, parse_line, toggle_raw};
use super::validate::is_protected_host;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Entry {
    pub enabled: bool,
    pub ip: String,
    pub hosts: Vec<String>,
    pub comment: Option<String>,
    /// Original line text; `None` once the entry was edited and must be re-formatted.
    pub raw: Option<String>,
}

impl Entry {
    pub fn new(enabled: bool, ip: &str, hosts: Vec<String>, comment: Option<String>) -> Self {
        Self {
            enabled,
            ip: ip.to_string(),
            hosts,
            comment,
            raw: None,
        }
    }

    pub fn is_protected(&self) -> bool {
        self.hosts.iter().any(|h| is_protected_host(h))
    }

    /// Same content, ignoring the original formatting.
    pub fn same_content(&self, other: &Entry) -> bool {
        self.enabled == other.enabled
            && self.ip == other.ip
            && self.hosts == other.hosts
            && self.comment == other.comment
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Line {
    Blank(String),
    Comment(String),
    Entry(Entry),
    GroupStart {
        raw: String,
        name: String,
    },
    GroupEnd(String),
    /// A non-comment line that is not a valid entry; kept untouched.
    Other(String),
}

impl Line {
    pub fn text(&self) -> Cow<'_, str> {
        match self {
            Line::Blank(r) | Line::Comment(r) | Line::GroupEnd(r) | Line::Other(r) => Cow::Borrowed(r),
            Line::GroupStart { raw, .. } => Cow::Borrowed(raw),
            Line::Entry(e) => match &e.raw {
                Some(r) => Cow::Borrowed(r),
                None => Cow::Owned(format_entry(e)),
            },
        }
    }
}

/// A group as seen by the UI. `start == None` is the virtual "ungrouped" group.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Group {
    pub name: Option<String>,
    pub start: Option<usize>,
    /// Index of the `# @end` line, if the group is terminated.
    pub end: Option<usize>,
    /// Last line index belonging to the group (inclusive).
    pub last: usize,
    pub entries: Vec<usize>,
}

impl Group {
    pub fn enabled_count(&self, doc: &Document) -> usize {
        self.entries
            .iter()
            .filter(|&&i| doc.entry(i).is_some_and(|e| e.enabled))
            .count()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Document {
    pub lines: Vec<Line>,
    trailing_newline: bool,
}

impl Document {
    pub fn parse(text: &str) -> Self {
        let trailing_newline = text.ends_with('\n');
        let body = text.strip_suffix('\n').unwrap_or(text);
        let lines = if text.is_empty() {
            Vec::new()
        } else {
            body.split('\n').map(parse_line).collect()
        };
        Self {
            lines,
            trailing_newline,
        }
    }

    pub fn to_text(&self) -> String {
        let mut out = self.lines.iter().map(|l| l.text()).collect::<Vec<_>>().join("\n");
        if self.trailing_newline || !out.is_empty() {
            out.push('\n');
        }
        out
    }

    pub fn entry(&self, idx: usize) -> Option<&Entry> {
        match self.lines.get(idx) {
            Some(Line::Entry(e)) => Some(e),
            _ => None,
        }
    }

    fn entry_mut(&mut self, idx: usize) -> Option<&mut Entry> {
        match self.lines.get_mut(idx) {
            Some(Line::Entry(e)) => Some(e),
            _ => None,
        }
    }

    pub fn entries(&self) -> impl Iterator<Item = (usize, &Entry)> {
        self.lines.iter().enumerate().filter_map(|(i, l)| match l {
            Line::Entry(e) => Some((i, e)),
            _ => None,
        })
    }

    /// The ungrouped group first, then named groups in file order.
    pub fn groups(&self) -> Vec<Group> {
        let mut ungrouped = Group {
            name: None,
            start: None,
            end: None,
            last: 0,
            entries: Vec::new(),
        };
        let mut named: Vec<Group> = Vec::new();
        let mut current: Option<Group> = None;

        for (i, line) in self.lines.iter().enumerate() {
            match line {
                Line::GroupStart { name, .. } => {
                    if let Some(mut g) = current.take() {
                        g.last = i - 1;
                        named.push(g);
                    }
                    current = Some(Group {
                        name: Some(name.clone()),
                        start: Some(i),
                        end: None,
                        last: i,
                        entries: Vec::new(),
                    });
                }
                Line::GroupEnd(_) => {
                    if let Some(mut g) = current.take() {
                        g.end = Some(i);
                        g.last = i;
                        named.push(g);
                    }
                }
                Line::Entry(_) => match current.as_mut() {
                    Some(g) => g.entries.push(i),
                    None => ungrouped.entries.push(i),
                },
                _ => {}
            }
        }
        if let Some(mut g) = current {
            g.last = self.lines.len() - 1;
            named.push(g);
        }
        let mut groups = vec![ungrouped];
        groups.extend(named);
        groups
    }

    /// Start line of the group containing `idx` (`None` = ungrouped).
    pub fn group_of(&self, idx: usize) -> Option<usize> {
        self.groups()
            .into_iter()
            .find(|g| g.entries.contains(&idx))
            .and_then(|g| g.start)
    }

    fn group(&self, start: Option<usize>) -> Option<Group> {
        self.groups().into_iter().find(|g| g.start == start)
    }

    #[cfg(test)]
    pub fn group_names(&self) -> Vec<String> {
        self.groups().into_iter().filter_map(|g| g.name).collect()
    }

    pub fn find_group(&self, name: &str) -> Option<usize> {
        self.groups()
            .into_iter()
            .find(|g| g.name.as_deref() == Some(name))
            .and_then(|g| g.start)
    }

    // ---- entry operations ----

    pub fn set_enabled(&mut self, idx: usize, enabled: bool) {
        if let Some(e) = self.entry_mut(idx) {
            if e.enabled == enabled {
                return;
            }
            e.enabled = enabled;
            if let Some(raw) = &e.raw {
                e.raw = Some(toggle_raw(raw, enabled));
            }
        }
    }

    #[cfg(test)]
    pub fn toggle(&mut self, idx: usize) {
        if let Some(enabled) = self.entry(idx).map(|e| e.enabled) {
            self.set_enabled(idx, !enabled);
        }
    }

    /// Enables every entry of the group unless all are enabled already, then disables them.
    /// Protected entries are left alone.
    pub fn toggle_group(&mut self, start: Option<usize>) {
        let Some(group) = self.group(start) else { return };
        let targets: Vec<usize> = group
            .entries
            .into_iter()
            .filter(|&i| !self.entry(i).is_some_and(Entry::is_protected))
            .collect();
        let enable = !targets.iter().all(|&i| self.entry(i).is_some_and(|e| e.enabled));
        for i in targets {
            self.set_enabled(i, enable);
        }
    }

    /// Where a new entry for the group goes: after its last entry, else right after its header.
    fn insertion_point(&self, start: Option<usize>) -> usize {
        let groups = self.groups();
        let Some(group) = groups.iter().find(|g| g.start == start) else {
            return self.lines.len();
        };
        if let Some(&last) = group.entries.last() {
            return last + 1;
        }
        match group.start {
            Some(s) => s + 1,
            None => groups.get(1).and_then(|g| g.start).unwrap_or(self.lines.len()),
        }
    }

    /// Returns the line index of the inserted entry.
    pub fn add_entry(&mut self, mut entry: Entry, group: Option<usize>) -> usize {
        entry.raw = None;
        let at = self.insertion_point(group);
        self.lines.insert(at, Line::Entry(entry));
        at
    }

    /// Replaces an entry, moving it if the group changed. Returns its new line index.
    pub fn update_entry(&mut self, idx: usize, entry: Entry, group: Option<usize>) -> usize {
        if self.group_of(idx) != group {
            let group_name = group.and_then(|s| self.group_name(s));
            self.lines.remove(idx);
            let group = group_name.and_then(|n| self.find_group(&n));
            return self.add_entry(entry, group);
        }
        if let Some(e) = self.entry_mut(idx)
            && !e.same_content(&entry)
        {
            *e = Entry { raw: None, ..entry };
        }
        idx
    }

    fn group_name(&self, start: usize) -> Option<String> {
        match self.lines.get(start) {
            Some(Line::GroupStart { name, .. }) => Some(name.clone()),
            _ => None,
        }
    }

    /// Deletes the given entries (any order).
    pub fn delete_entries(&mut self, idxs: &[usize]) {
        let mut idxs: Vec<usize> = idxs.iter().copied().filter(|&i| self.entry(i).is_some()).collect();
        idxs.sort_unstable();
        idxs.dedup();
        for i in idxs.into_iter().rev() {
            self.lines.remove(i);
        }
    }

    pub fn duplicate(&mut self, idx: usize) -> Option<usize> {
        let mut copy = self.entry(idx)?.clone();
        copy.raw = None;
        self.lines.insert(idx + 1, Line::Entry(copy));
        Some(idx + 1)
    }

    /// Swaps the entry with its neighbour inside the same group. Returns the new index.
    pub fn move_entry(&mut self, idx: usize, down: bool) -> usize {
        let group = self.groups().into_iter().find(|g| g.entries.contains(&idx));
        let Some(group) = group else { return idx };
        let pos = group.entries.iter().position(|&i| i == idx).unwrap_or(0);
        let neighbour = if down {
            group.entries.get(pos + 1)
        } else {
            pos.checked_sub(1).and_then(|p| group.entries.get(p))
        };
        match neighbour {
            Some(&n) => {
                self.lines.swap(idx, n);
                n
            }
            None => idx,
        }
    }

    /// Moves entries to another group, keeping their relative order. Returns their new indices.
    pub fn move_entries_to_group(&mut self, idxs: &[usize], group_name: Option<&str>) -> Vec<usize> {
        let mut idxs: Vec<usize> = idxs.iter().copied().filter(|&i| self.entry(i).is_some()).collect();
        idxs.sort_unstable();
        idxs.dedup();
        let entries: Vec<Entry> = idxs.iter().filter_map(|&i| self.entry(i).cloned()).collect();
        for &i in idxs.iter().rev() {
            self.lines.remove(i);
        }
        entries
            .into_iter()
            .map(|e| {
                let group = group_name.and_then(|n| self.find_group(n));
                let at = self.insertion_point(group);
                self.lines.insert(at, Line::Entry(e));
                at
            })
            .collect()
    }

    // ---- group operations ----

    /// Returns the start line of the new group.
    pub fn create_group(&mut self, name: &str) -> usize {
        if self.lines.last().is_some_and(|l| !matches!(l, Line::Blank(_))) {
            self.lines.push(Line::Blank(String::new()));
        }
        let start = self.lines.len();
        self.lines.push(Line::GroupStart {
            raw: format_group_start(name),
            name: name.to_string(),
        });
        self.lines.push(Line::GroupEnd(format_group_end()));
        start
    }

    pub fn rename_group(&mut self, start: usize, new_name: &str) {
        if let Some(line @ Line::GroupStart { .. }) = self.lines.get_mut(start) {
            *line = Line::GroupStart {
                raw: format_group_start(new_name),
                name: new_name.to_string(),
            };
        }
    }

    /// Adds missing `# @end` markers so groups can be moved around safely.
    fn terminate_groups(&mut self) {
        let unterminated: Vec<usize> = self
            .groups()
            .into_iter()
            .filter(|g| g.start.is_some() && g.end.is_none())
            .map(|g| g.last)
            .collect();
        for last in unterminated.into_iter().rev() {
            self.lines.insert(last + 1, Line::GroupEnd(format_group_end()));
        }
    }

    /// With `keep_entries`, only the markers are removed and the entries become ungrouped.
    pub fn delete_group(&mut self, start: usize, keep_entries: bool) {
        let Some(name) = self.group_name(start) else { return };
        self.terminate_groups();
        let Some(group) = self.find_group(&name).and_then(|s| self.group(Some(s))) else {
            return;
        };
        let (s, end) = (group.start.unwrap_or(start), group.end.unwrap_or(group.last));
        if keep_entries {
            self.lines.remove(end);
            self.lines.remove(s);
            return;
        }
        self.lines.drain(s..=end);
        if s > 0 && matches!(self.lines.get(s - 1), Some(Line::Blank(_))) {
            self.lines.remove(s - 1);
        }
    }

    /// Swaps a named group with its neighbour. Returns the group's new start line.
    pub fn move_group(&mut self, start: usize, down: bool) -> usize {
        let Some(name) = self.group_name(start) else {
            return start;
        };
        self.terminate_groups();
        let named: Vec<Group> = self.groups().into_iter().skip(1).collect();
        let Some(pos) = named.iter().position(|g| g.name.as_deref() == Some(&name)) else {
            return start;
        };
        let other = if down {
            pos + 1
        } else {
            match pos.checked_sub(1) {
                Some(p) => p,
                None => return named[pos].start.unwrap_or(start),
            }
        };
        let Some(other_group) = named.get(other) else {
            return named[pos].start.unwrap_or(start);
        };
        let (a, b) = if down {
            (&named[pos], other_group)
        } else {
            (other_group, &named[pos])
        };
        let (a_s, a_e, b_s, b_e) = (a.start.unwrap_or(0), a.last, b.start.unwrap_or(0), b.last);

        let mut lines = Vec::with_capacity(self.lines.len());
        lines.extend_from_slice(&self.lines[..a_s]);
        lines.extend_from_slice(&self.lines[b_s..=b_e]);
        lines.extend_from_slice(&self.lines[a_e + 1..b_s]);
        lines.extend_from_slice(&self.lines[a_s..=a_e]);
        lines.extend_from_slice(&self.lines[b_e + 1..]);
        self.lines = lines;
        self.find_group(&name).unwrap_or(start)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = "\
##
# Host Database
##
127.0.0.1\tlocalhost
255.255.255.255\tbroadcasthost
::1             localhost
garbage line

# @group Docker
127.0.0.1   postgres.local  # db
#127.0.0.1  redis.local
# @end

# @group Work
10.0.0.5 api.local
";

    #[test]
    fn roundtrip_is_byte_exact() {
        for text in [
            SAMPLE,
            "",
            "\n",
            "no newline 1.2.3.4",
            "127.0.0.1 a\n\n\n",
            "\r\n127.0.0.1 a\r\n",
        ] {
            assert_eq!(
                Document::parse(text).to_text(),
                text.to_string()
                    + if text.ends_with('\n') || text.is_empty() {
                        ""
                    } else {
                        "\n"
                    }
            );
        }
    }

    #[test]
    fn groups_are_detected() {
        let doc = Document::parse(SAMPLE);
        let groups = doc.groups();
        assert_eq!(groups.len(), 3);
        assert_eq!(groups[0].entries, vec![3, 4, 5]);
        assert_eq!(
            (groups[1].name.as_deref(), groups[1].start, groups[1].end),
            (Some("Docker"), Some(8), Some(11))
        );
        assert_eq!(groups[1].entries, vec![9, 10]);
        assert_eq!(groups[1].enabled_count(&doc), 1);
        assert_eq!(
            (groups[2].name.as_deref(), groups[2].end, groups[2].last),
            (Some("Work"), None, 14)
        );
    }

    #[test]
    fn toggle_only_touches_one_line() {
        let mut doc = Document::parse(SAMPLE);
        doc.toggle(10);
        let text = doc.to_text();
        assert_eq!(
            text,
            SAMPLE.replace("#127.0.0.1  redis.local", "127.0.0.1  redis.local")
        );
        doc.toggle(10);
        assert!(doc.to_text().contains("# 127.0.0.1  redis.local"));
    }

    #[test]
    fn toggle_group_skips_protected() {
        let mut doc = Document::parse(SAMPLE);
        doc.toggle_group(None);
        assert!(doc.entries().all(|(i, e)| i > 5 || e.enabled));
        doc.toggle_group(Some(8));
        assert!(doc.entry(10).unwrap().enabled);
        doc.toggle_group(Some(8));
        assert!(!doc.entry(9).unwrap().enabled && !doc.entry(10).unwrap().enabled);
    }

    #[test]
    fn add_goes_to_end_of_group() {
        let mut doc = Document::parse(SAMPLE);
        let at = doc.add_entry(Entry::new(true, "127.0.0.1", vec!["kafka.local".into()], None), Some(8));
        assert_eq!(at, 11);
        assert_eq!(doc.groups()[1].entries, vec![9, 10, 11]);
        let at = doc.add_entry(Entry::new(true, "1.1.1.1", vec!["x".into()], None), None);
        assert_eq!(at, 6);
    }

    #[test]
    fn add_to_empty_document_and_group() {
        let mut doc = Document::parse("");
        doc.add_entry(Entry::new(true, "1.1.1.1", vec!["x".into()], None), None);
        let g = doc.create_group("New");
        doc.add_entry(Entry::new(false, "2.2.2.2", vec!["y".into()], None), Some(g));
        assert_eq!(
            doc.to_text(),
            "1.1.1.1         x\n\n# @group New\n# 2.2.2.2         y\n# @end\n"
        );
    }

    #[test]
    fn update_moves_between_groups() {
        let mut doc = Document::parse(SAMPLE);
        let e = Entry::new(true, "10.0.0.9", vec!["postgres.local".into()], None);
        let new_idx = doc.update_entry(9, e, doc.find_group("Work"));
        assert_eq!(doc.group_of(new_idx), doc.find_group("Work"));
        assert_eq!(doc.entry(new_idx).unwrap().ip, "10.0.0.9");
    }

    #[test]
    fn update_without_changes_keeps_raw() {
        let mut doc = Document::parse(SAMPLE);
        let e = doc.entry(9).unwrap().clone();
        doc.update_entry(9, Entry { raw: None, ..e }, Some(8));
        assert_eq!(doc.to_text(), SAMPLE);
    }

    #[test]
    fn move_entry_within_group() {
        let mut doc = Document::parse(SAMPLE);
        assert_eq!(doc.move_entry(9, true), 10);
        assert_eq!(doc.entry(10).unwrap().hosts, vec!["postgres.local"]);
        assert_eq!(doc.move_entry(10, true), 10);
    }

    #[test]
    fn delete_group_keep_or_drop() {
        let mut doc = Document::parse(SAMPLE);
        doc.delete_group(8, true);
        assert_eq!(doc.groups().len(), 2);
        assert!(doc.to_text().contains("postgres.local"));

        let mut doc = Document::parse(SAMPLE);
        doc.delete_group(8, false);
        let text = doc.to_text();
        assert!(!text.contains("postgres.local") && !text.contains("Docker"));
        assert!(text.contains("# @group Work"));
    }

    #[test]
    fn move_group_swaps_blocks() {
        let mut doc = Document::parse(SAMPLE);
        let new_start = doc.move_group(8, true);
        let names = doc.group_names();
        assert_eq!(names, vec!["Work", "Docker"]);
        assert_eq!(
            doc.lines[new_start],
            Line::GroupStart {
                raw: "# @group Docker".into(),
                name: "Docker".into()
            }
        );
        // Work got its missing @end, so it does not swallow Docker.
        assert_eq!(doc.groups()[1].entries.len(), 1);
        assert_eq!(doc.groups()[2].entries.len(), 2);
    }

    #[test]
    fn move_entries_to_group_keeps_order() {
        let mut doc = Document::parse(SAMPLE);
        let new = doc.move_entries_to_group(&[10, 9], Some("Work"));
        let hosts: Vec<_> = new.iter().map(|&i| doc.entry(i).unwrap().hosts[0].clone()).collect();
        assert_eq!(hosts, vec!["postgres.local", "redis.local"]);
        assert!(doc.groups()[1].entries.is_empty());
    }
}
