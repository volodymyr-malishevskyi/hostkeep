//! Lossless line parser: every line keeps its original text until it is edited.

use super::model::{Entry, Line};
use super::validate::{is_valid_hostname, parse_ip};

pub const GROUP_START: &str = "@group";
pub const GROUP_END: &str = "@end";

pub fn parse_line(raw: &str) -> Line {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return Line::Blank(raw.to_string());
    }
    if let Some(body) = trimmed.strip_prefix('#') {
        let body = body.trim();
        if body == GROUP_END {
            return Line::GroupEnd(raw.to_string());
        }
        if let Some(name) = body.strip_prefix(GROUP_START) {
            let name = name.trim();
            if !name.is_empty() && body[GROUP_START.len()..].starts_with(char::is_whitespace) {
                return Line::GroupStart {
                    raw: raw.to_string(),
                    name: name.to_string(),
                };
            }
        }
        // `## ...` is always prose; a single `#` may hide a disabled entry.
        if !body.starts_with('#')
            && let Some(mut entry) = parse_entry(body, true)
        {
            entry.enabled = false;
            entry.raw = Some(raw.to_string());
            return Line::Entry(entry);
        }
        return Line::Comment(raw.to_string());
    }
    match parse_entry(trimmed, false) {
        Some(mut entry) => {
            entry.raw = Some(raw.to_string());
            Line::Entry(entry)
        }
        None => Line::Other(raw.to_string()),
    }
}

/// `strict` requires valid hostnames: used for commented lines so prose is not mistaken for an entry.
fn parse_entry(text: &str, strict: bool) -> Option<Entry> {
    let (data, comment) = match text.split_once('#') {
        Some((d, c)) => (d, Some(c.trim()).filter(|c| !c.is_empty())),
        None => (text, None),
    };
    let mut tokens = data.split_whitespace();
    let ip = tokens.next()?;
    parse_ip(ip)?;
    let hosts: Vec<String> = tokens.map(str::to_string).collect();
    if hosts.is_empty() || (strict && !hosts.iter().all(|h| is_valid_hostname(h))) {
        return None;
    }
    Some(Entry {
        enabled: true,
        ip: ip.to_string(),
        hosts,
        comment: comment.map(str::to_string),
        raw: None,
    })
}

pub fn format_entry(entry: &Entry) -> String {
    let mut out = String::new();
    if !entry.enabled {
        out.push_str("# ");
    }
    out.push_str(&format!("{:<15} {}", entry.ip, entry.hosts.join(" ")));
    if let Some(c) = &entry.comment {
        out.push_str("  # ");
        out.push_str(c);
    }
    out
}

pub fn format_group_start(name: &str) -> String {
    format!("# {GROUP_START} {name}")
}

pub fn format_group_end() -> String {
    format!("# {GROUP_END}")
}

/// Flips the enabled state while keeping the original spacing of the line.
pub fn toggle_raw(raw: &str, enable: bool) -> String {
    if enable {
        raw.trim_start().trim_start_matches('#').trim_start().to_string()
    } else {
        format!("# {raw}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(line: &Line) -> &Entry {
        match line {
            Line::Entry(e) => e,
            other => panic!("expected entry, got {other:?}"),
        }
    }

    #[test]
    fn active_entry_with_comment() {
        let l = parse_line("127.0.0.1\tapi.local  www.api.local # backend");
        let e = entry(&l);
        assert!(e.enabled);
        assert_eq!(e.ip, "127.0.0.1");
        assert_eq!(e.hosts, vec!["api.local", "www.api.local"]);
        assert_eq!(e.comment.as_deref(), Some("backend"));
    }

    #[test]
    fn disabled_entry() {
        let e = entry(&parse_line("#  10.0.0.1 kafka.local")).clone();
        assert!(!e.enabled);
        assert_eq!(e.hosts, vec!["kafka.local"]);
    }

    #[test]
    fn prose_comments_stay_comments() {
        for line in [
            "# localhost is used to configure the loopback interface",
            "## 127.0.0.1 old.local",
            "##",
            "# 127.0.0.1 is, the loopback",
            "#",
        ] {
            assert!(matches!(parse_line(line), Line::Comment(_)), "{line}");
        }
    }

    #[test]
    fn group_markers() {
        assert!(matches!(parse_line("# @group Docker dev"), Line::GroupStart { name, .. } if name == "Docker dev"));
        assert!(matches!(parse_line("#@end"), Line::GroupEnd(_)));
        assert!(matches!(parse_line("# @groupie"), Line::Comment(_)));
        assert!(matches!(parse_line("# @group"), Line::Comment(_)));
    }

    #[test]
    fn invalid_and_blank() {
        assert!(matches!(parse_line("   "), Line::Blank(_)));
        assert!(matches!(parse_line("garbage here"), Line::Other(_)));
        assert!(matches!(parse_line("127.0.0.1"), Line::Other(_)));
    }

    #[test]
    fn toggle_keeps_spacing() {
        assert_eq!(toggle_raw("127.0.0.1\ta  b", false), "# 127.0.0.1\ta  b");
        assert_eq!(toggle_raw("#127.0.0.1\ta  b", true), "127.0.0.1\ta  b");
        assert_eq!(toggle_raw("  #  ::1 x", true), "::1 x");
    }

    #[test]
    fn format_roundtrip() {
        let e = Entry::new(
            false,
            "10.0.0.1",
            vec!["a.local".into(), "b.local".into()],
            Some("note".into()),
        );
        let text = format_entry(&e);
        assert_eq!(text, "# 10.0.0.1        a.local b.local  # note");
        let parsed = entry(&parse_line(&text)).clone();
        assert_eq!(
            (parsed.enabled, parsed.ip, parsed.hosts, parsed.comment),
            (e.enabled, e.ip, e.hosts, e.comment)
        );
    }
}
