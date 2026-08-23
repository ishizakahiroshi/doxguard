use anyhow::Result;
use regex::Regex;

use crate::config::{Config, compile_custom_regex};

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
            // Case-insensitive Users/dev; allow JSON-style doubled backslashes.
            r"(?i)[A-Za-z]:(?:\\+|/)(?:Users|dev)(?:\\+|/)",
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
        // Real octets only (0-255, zero padding allowed) so version strings like
        // 10.999.0.1 stay quiet while inventory-style 192.168.001.100 still hits.
        const OCTET: &str = r"(?:25[0-5]|2[0-4]\d|[01]?\d?\d)";
        let private_ip = format!(
            r"\b(?:10\.{OCTET}\.{OCTET}\.{OCTET}|172\.(?:1[6-9]|2\d|3[01])\.{OCTET}\.{OCTET}|192\.168\.{OCTET}\.{OCTET})\b"
        );
        patterns.push(pattern(
            "Private IPv4 (RFC1918)",
            &private_ip,
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
            regex: compile_custom_regex(custom)?,
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
    fn private_ip_requires_valid_octets() {
        let config = Config::default();
        let patterns = build(&config).unwrap();
        let ip = patterns
            .iter()
            .find(|pattern| pattern.name.contains("RFC1918"))
            .unwrap();
        assert!(ip.regex.is_match("10.255.0.1"));
        assert!(ip.regex.is_match("192.168.50.9"));
        assert!(ip.regex.is_match("172.31.0.254"));
        // Zero-padded octets appear in real inventories and exports.
        assert!(ip.regex.is_match("192.168.001.100"));
        assert!(ip.regex.is_match("10.01.2.3"));
        assert!(!ip.regex.is_match("10.256.0.1"));
        assert!(!ip.regex.is_match("192.168.300.1"));
        assert!(!ip.regex.is_match("172.32.0.1"));
    }

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
