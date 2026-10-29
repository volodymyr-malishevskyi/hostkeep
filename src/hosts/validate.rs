use std::collections::{BTreeMap, BTreeSet};
use std::net::IpAddr;

use super::model::Document;

/// Parses an IP, accepting an IPv6 zone suffix (`fe80::1%lo0`).
pub fn parse_ip(s: &str) -> Option<IpAddr> {
    let addr = s.split_once('%').map_or(s, |(a, _)| a);
    addr.parse().ok()
}

/// RFC 1123 hostname; underscores are tolerated because they are common in hosts files.
pub fn is_valid_hostname(s: &str) -> bool {
    let s = s.strip_suffix('.').unwrap_or(s);
    if s.is_empty() || s.len() > 253 {
        return false;
    }
    s.split('.').all(|label| {
        !label.is_empty()
            && label.len() <= 63
            && !label.starts_with('-')
            && !label.ends_with('-')
            && label.chars().all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
    })
}

/// Entries a user should not disable or delete by accident.
pub fn is_protected_host(host: &str) -> bool {
    matches!(
        host.to_ascii_lowercase().as_str(),
        "localhost" | "localhost.localdomain" | "broadcasthost" | "ip6-localhost" | "ip6-loopback"
    )
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FieldError {
    IpEmpty,
    IpInvalid(String),
    HostsEmpty,
    HostInvalid(String),
}

pub fn validate_fields(ip: &str, hosts: &str) -> Result<(), FieldError> {
    let ip = ip.trim();
    if ip.is_empty() {
        return Err(FieldError::IpEmpty);
    }
    if parse_ip(ip).is_none() {
        return Err(FieldError::IpInvalid(ip.to_string()));
    }
    let mut any = false;
    for host in hosts.split_whitespace() {
        any = true;
        if !is_valid_hostname(host) {
            return Err(FieldError::HostInvalid(host.to_string()));
        }
    }
    if !any {
        return Err(FieldError::HostsEmpty);
    }
    Ok(())
}

/// A hostname enabled with different addresses of the same family.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Conflict {
    pub host: String,
    pub ips: Vec<String>,
}

#[derive(Debug, Default)]
pub struct Report {
    pub conflicts: Vec<Conflict>,
    /// Line indices of enabled entries that exactly repeat an earlier (ip, host) pair.
    pub duplicate_lines: BTreeSet<usize>,
    /// Line indices of entries involved in a conflict.
    pub conflict_lines: BTreeSet<usize>,
}

impl Report {
    pub fn conflict_for(&self, hosts: &[String]) -> Option<&Conflict> {
        self.conflicts
            .iter()
            .find(|c| hosts.iter().any(|h| h.eq_ignore_ascii_case(&c.host)))
    }
}

pub fn analyze(doc: &Document) -> Report {
    let mut report = Report::default();
    // host -> (is_v6 -> ip -> lines)
    let mut by_host: BTreeMap<String, BTreeMap<bool, BTreeMap<String, Vec<usize>>>> = BTreeMap::new();
    let mut seen_pairs = BTreeSet::new();

    for (idx, entry) in doc.entries() {
        if !entry.enabled {
            continue;
        }
        let v6 = parse_ip(&entry.ip).is_some_and(|ip| ip.is_ipv6());
        for host in &entry.hosts {
            let host = host.to_ascii_lowercase();
            if !seen_pairs.insert((entry.ip.clone(), host.clone())) {
                report.duplicate_lines.insert(idx);
            }
            by_host
                .entry(host)
                .or_default()
                .entry(v6)
                .or_default()
                .entry(entry.ip.clone())
                .or_default()
                .push(idx);
        }
    }

    for (host, families) in by_host {
        for ips in families.values() {
            if ips.len() > 1 {
                report.conflict_lines.extend(ips.values().flatten());
                report.conflicts.push(Conflict {
                    host: host.clone(),
                    ips: ips.keys().cloned().collect(),
                });
            }
        }
    }
    report
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hostnames() {
        assert!(is_valid_hostname("api.local"));
        assert!(is_valid_hostname("my_host"));
        assert!(is_valid_hostname("a-b.c"));
        assert!(!is_valid_hostname("www.api..local"));
        assert!(!is_valid_hostname("-bad"));
        assert!(!is_valid_hostname("bad-"));
        assert!(!is_valid_hostname("is,not"));
        assert!(!is_valid_hostname(""));
    }

    #[test]
    fn ips() {
        assert!(parse_ip("127.0.0.1").is_some());
        assert!(parse_ip("::1").is_some());
        assert!(parse_ip("fe80::1%lo0").is_some());
        assert!(parse_ip("300.0.0.1").is_none());
        assert!(parse_ip("localhost").is_none());
    }

    #[test]
    fn fields() {
        assert_eq!(validate_fields("", "a"), Err(FieldError::IpEmpty));
        assert_eq!(validate_fields("1.2.3.4", "  "), Err(FieldError::HostsEmpty));
        assert!(matches!(validate_fields("x", "a"), Err(FieldError::IpInvalid(_))));
        assert!(matches!(validate_fields("1.2.3.4", "a b..c"), Err(FieldError::HostInvalid(h)) if h == "b..c"));
        assert_eq!(validate_fields("1.2.3.4", "a b"), Ok(()));
    }

    #[test]
    fn conflicts_and_duplicates() {
        let doc = Document::parse(
            "127.0.0.1 localhost\n::1 localhost\n127.0.0.1 api.local\n10.0.0.5 api.local\n# 10.0.0.6 api.local\n127.0.0.1 localhost\n",
        );
        let report = analyze(&doc);
        assert_eq!(
            report.conflicts,
            vec![Conflict {
                host: "api.local".into(),
                ips: vec!["10.0.0.5".into(), "127.0.0.1".into()]
            }]
        );
        assert_eq!(report.conflict_lines, BTreeSet::from([2, 3]));
        assert_eq!(report.duplicate_lines, BTreeSet::from([5]));
    }
}
