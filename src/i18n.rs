use std::borrow::Cow;
use std::collections::HashMap;
use std::sync::atomic::{AtomicU8, Ordering};

use fluent_templates::fluent_bundle::FluentValue;
use fluent_templates::{LanguageIdentifier, Loader, langid, static_loader};

static_loader! {
    static LOCALES = {
        locales: "./locales",
        fallback_language: "en",
        // Isolation marks are invisible but break terminal width calculations.
        customise: |bundle| bundle.set_use_isolating(false),
    };
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Lang {
    En,
    Uk,
}

impl Lang {
    pub const ALL: [Lang; 2] = [Lang::En, Lang::Uk];

    pub fn code(self) -> &'static str {
        match self {
            Lang::En => "en",
            Lang::Uk => "uk",
        }
    }

    pub fn from_code(code: &str) -> Option<Lang> {
        let code = code.trim().to_ascii_lowercase();
        Lang::ALL.into_iter().find(|l| {
            code == l.code()
                || code.starts_with(&format!("{}-", l.code()))
                || code.starts_with(&format!("{}_", l.code()))
        })
    }

    pub fn next(self) -> Lang {
        match self {
            Lang::En => Lang::Uk,
            Lang::Uk => Lang::En,
        }
    }

    fn id(self) -> LanguageIdentifier {
        match self {
            Lang::En => langid!("en"),
            Lang::Uk => langid!("uk"),
        }
    }
}

static CURRENT: AtomicU8 = AtomicU8::new(0);

pub fn set_lang(lang: Lang) {
    CURRENT.store(lang as u8, Ordering::Relaxed);
}

pub fn lang() -> Lang {
    if CURRENT.load(Ordering::Relaxed) == Lang::Uk as u8 {
        Lang::Uk
    } else {
        Lang::En
    }
}

/// `--lang` → config (`auto` means skip) → system locale → English.
pub fn resolve(cli: Option<&str>, config: &str) -> Lang {
    cli.and_then(Lang::from_code)
        .or_else(|| Lang::from_code(config))
        .or_else(system_lang)
        .unwrap_or(Lang::En)
}

fn system_lang() -> Option<Lang> {
    // Explicit environment settings win over macOS system preferences.
    for var in ["LC_ALL", "LC_MESSAGES", "LANG"] {
        if let Ok(v) = std::env::var(var)
            && !v.is_empty()
            && v != "C"
            && v != "POSIX"
            && !v.starts_with("C.")
        {
            return Lang::from_code(&v).or(Some(Lang::En));
        }
    }
    sys_locale::get_locale().and_then(|l| Lang::from_code(&l))
}

pub fn tr(key: &str) -> String {
    LOCALES.lookup(&lang().id(), key)
}

pub fn tr_args(key: &str, args: Vec<(&'static str, FluentValue<'_>)>) -> String {
    let args: HashMap<Cow<'static, str>, FluentValue> = args.into_iter().map(|(k, v)| (Cow::Borrowed(k), v)).collect();
    LOCALES.lookup_with_args(&lang().id(), key, &args)
}

/// `t!("key")` or `t!("key", count = 3, name = "x")`.
#[macro_export]
macro_rules! t {
    ($key:expr) => {
        $crate::i18n::tr($key)
    };
    ($key:expr, $($name:ident = $value:expr),+ $(,)?) => {
        $crate::i18n::tr_args($key, vec![$((stringify!($name), fluent_templates::fluent_bundle::FluentValue::from($value))),+])
    };
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeSet;
    use std::fs;

    fn keys(lang: &str) -> BTreeSet<String> {
        let text = fs::read_to_string(format!("locales/{lang}/main.ftl")).unwrap();
        text.lines()
            .filter(|l| l.chars().next().is_some_and(|c| c.is_ascii_lowercase()))
            .filter_map(|l| l.split_once('=').map(|(k, _)| k.trim().to_string()))
            .collect()
    }

    /// Every `t!("...")` key in the sources.
    fn used_keys() -> BTreeSet<String> {
        let mut out = BTreeSet::new();
        let mut stack = vec![std::path::PathBuf::from("src")];
        while let Some(p) = stack.pop() {
            if p.is_dir() {
                stack.extend(fs::read_dir(&p).unwrap().flatten().map(|e| e.path()));
                continue;
            }
            let text = fs::read_to_string(&p).unwrap();
            for (pos, _) in text.match_indices("t!(\"") {
                let ident_before = text[..pos]
                    .chars()
                    .last()
                    .is_some_and(|c| c.is_alphanumeric() || c == '_');
                let key = text[pos + 4..].split('"').next().unwrap_or_default();
                if !ident_before && key != "key" && key != "..." {
                    out.insert(key.to_string());
                }
            }
        }
        out
    }

    #[test]
    fn locales_have_same_keys() {
        let (en, uk) = (keys("en"), keys("uk"));
        assert!(!en.is_empty());
        assert_eq!(
            en.difference(&uk).collect::<Vec<_>>(),
            Vec::<&String>::new(),
            "missing in uk"
        );
        assert_eq!(
            uk.difference(&en).collect::<Vec<_>>(),
            Vec::<&String>::new(),
            "missing in en"
        );
    }

    #[test]
    fn used_keys_exist() {
        let en = keys("en");
        let missing: Vec<_> = used_keys().into_iter().filter(|k| !en.contains(k)).collect();
        assert!(missing.is_empty(), "keys used in code but not translated: {missing:?}");
    }

    #[test]
    fn ukrainian_plurals() {
        set_lang(Lang::Uk);
        assert_eq!(crate::t!("entries-count", count = 1), "1 запис");
        assert_eq!(crate::t!("entries-count", count = 3), "3 записи");
        assert_eq!(crate::t!("entries-count", count = 5), "5 записів");
        set_lang(Lang::En);
        assert_eq!(crate::t!("entries-count", count = 1), "1 entry");
        assert_eq!(crate::t!("entries-count", count = 5), "5 entries");
    }

    #[test]
    fn language_resolution() {
        assert_eq!(resolve(Some("uk"), "en"), Lang::Uk);
        assert_eq!(resolve(None, "uk"), Lang::Uk);
        assert_eq!(resolve(Some("xx"), "en"), Lang::En);
        assert_eq!(Lang::from_code("uk_UA.UTF-8"), Some(Lang::Uk));
        assert_eq!(Lang::from_code("en-US"), Some(Lang::En));
    }
}
