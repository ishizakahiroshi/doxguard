use anyhow::{Context, Result};
use regex::Regex;

use crate::config::Config;

#[derive(Debug, Clone)]
enum AllowRule {
    Email,
}

#[derive(Debug, Clone)]
pub struct StructuralPattern {
    pub name: String,
    pub regex: Regex,
    pub suggestion: String,
    allow: Option<AllowRule>,
}

impl StructuralPattern {
    pub fn is_allowed(&self, matched: &str, config: &Config) -> bool {
        match self.allow {
            Some(AllowRule::Email) => email_allowed(matched, config),
            None => false,
        }
    }
}

fn email_allowed(matched: &str, config: &Config) -> bool {
    let email = matched.to_ascii_lowercase();
    if config
        .allow
        .emails
        .iter()
        .any(|allowed| allowed.eq_ignore_ascii_case(&email))
    {
        return true;
    }
    let domain = email
        .split_once('@')
        .map(|(_, domain)| domain)
        .unwrap_or("");
    config.allow.email_domains.iter().any(|allowed| {
        let allowed = allowed.to_ascii_lowercase();
        domain == allowed || domain.ends_with(&format!(".{allowed}"))
    })
}

fn pattern(
    name: &str,
    regex: &str,
    suggestion: &str,
    allow: Option<AllowRule>,
) -> StructuralPattern {
    StructuralPattern {
        name: name.to_owned(),
        regex: Regex::new(regex).expect("built-in regex must be valid"),
        suggestion: suggestion.to_owned(),
        allow,
    }
}

pub fn build(config: &Config) -> Result<Vec<StructuralPattern>> {
    let mut patterns = Vec::new();
    if config.structural.windows_path {
        patterns.push(pattern(
            "Windows personal absolute path",
            r"[A-Za-z]:[\\/](?:Users|dev)[\\/]",
            "Remove the personal absolute path or replace it with a placeholder",
            None,
        ));
    }
    if config.structural.posix_home {
        patterns.push(pattern(
            "POSIX home path",
            r"/(?:Users|home)/[a-zA-Z0-9_.-]+/",
            "Replace the home path with ~/ or a placeholder",
            None,
        ));
    }
    if config.structural.private_ip {
        patterns.push(pattern(
            "Private IPv4 (RFC1918)",
            r"\b(?:10\.\d{1,3}\.\d{1,3}\.\d{1,3}|172\.(?:1[6-9]|2\d|3[01])\.\d{1,3}\.\d{1,3}|192\.168\.\d{1,3}\.\d{1,3})\b",
            "Generalize or remove the internal IP address",
            None,
        ));
    }
    if config.structural.email {
        patterns.push(pattern(
            "Email address (not on public allowlist)",
            r"\b[A-Za-z0-9._%+-]+@[A-Za-z0-9][A-Za-z0-9.-]*\.[A-Za-z]{2,}\b",
            "Remove the non-public email or add an explicitly public address/domain to allow",
            Some(AllowRule::Email),
        ));
    }
    for custom in &config.structural.custom {
        patterns.push(StructuralPattern {
            name: custom.name.clone(),
            regex: Regex::new(&custom.regex)
                .with_context(|| format!("invalid custom regex `{}`", custom.name))?,
            suggestion: custom
                .suggestion
                .clone()
                .unwrap_or_else(|| format!("Review match for custom pattern: {}", custom.name)),
            allow: None,
        });
    }
    Ok(patterns)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn email_allow_supports_exact_and_subdomains() {
        let mut config = Config::default();
        config.allow.emails.push("public@sample.test".to_owned());
        config.allow.email_domains.push("public.test".to_owned());
        assert!(email_allowed("public@sample.test", &config));
        assert!(email_allowed("team@news.public.test", &config));
        assert!(!email_allowed("private@sample.test", &config));
    }
}
