use anyhow::Result;
use regex::Regex;

use crate::config::{Config, compile_custom_regex};

#[derive(Debug, Clone)]
enum AllowRule {
    Email,
    Domain,
    Ipv4,
    PrivateRepo,
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
            Some(AllowRule::Domain) => config.allow.domains.iter().any(|allowed| {
                let matched = matched.to_ascii_lowercase();
                let allowed = allowed.trim().trim_matches('.').to_ascii_lowercase();
                matched == allowed || matched.ends_with(&format!(".{allowed}"))
            }),
            Some(AllowRule::Ipv4) => ipv4_address(matched).is_some_and(|address| {
                config
                    .allow
                    .ipv4
                    .iter()
                    .filter_map(|s| ipv4_network(s))
                    .any(|(net, mask)| address & mask == net & mask)
            }),
            Some(AllowRule::PrivateRepo) => {
                matched.rsplit(['/', '\\']).next().is_some_and(|repo| {
                    config
                        .allow
                        .private_repos
                        .iter()
                        .any(|allowed| allowed.eq_ignore_ascii_case(repo))
                })
            }
            None => false,
        }
    }

    /// Rust regex has no lookaround. Check token boundaries without consuming
    /// separators so adjacent values can both be found.
    pub fn valid_span(&self, line: &str, span: std::ops::Range<usize>) -> bool {
        let before = line[..span.start].chars().next_back();
        let after = line[span.end..].chars().next();
        match self.allow {
            Some(AllowRule::Domain) => {
                let word = |c: char| c.is_ascii_alphanumeric() || "._-".contains(c);
                !(before.is_some_and(word)
                    || after.is_some_and(|c| c.is_ascii_alphanumeric() || "_-".contains(c))
                    || (after == Some('.')
                        && line[span.end + 1..].starts_with(|c: char| c.is_ascii_alphanumeric())))
            }
            Some(AllowRule::Ipv4) => {
                !before.is_some_and(|c| c.is_ascii_digit() || c == '.')
                    && !after.is_some_and(|c| c.is_ascii_digit() || c == '.')
            }
            Some(AllowRule::Email) => {
                let trailing_domain = after == Some('.')
                    && line[span.end + 1..].starts_with(|c: char| c.is_ascii_alphanumeric());
                trailing_domain || !is_density_image_name(&line[span.clone()])
            }
            _ => true,
        }
    }
}

const IMAGE_EXTENSIONS: &[&str] = &[
    "png", "jpg", "jpeg", "gif", "webp", "avif", "heic", "bmp", "ico", "svg",
];

/// True for image file names such as `icon@2x.png` or `AppIcon-20x20@2x-1.png`
/// that the email regex reads as `local@domain`. Only `@<digits>x` (optionally
/// `-<digits>`) followed by an image extension and nothing else qualifies, so
/// `someone@2x.com` and `a@3x.png.example.com` still count as emails.
fn is_density_image_name(matched: &str) -> bool {
    let Some((_, domain)) = matched.split_once('@') else {
        return false;
    };
    let domain = domain.to_ascii_lowercase();
    let Some((stem, extension)) = domain.rsplit_once('.') else {
        return false;
    };
    if !IMAGE_EXTENSIONS.contains(&extension) {
        return false;
    }
    let (density, suffix) = match stem.split_once('-') {
        Some((density, suffix)) => (density, Some(suffix)),
        None => (stem, None),
    };
    let Some(digits) = density.strip_suffix('x') else {
        return false;
    };
    let numeric = |s: &str| !s.is_empty() && s.bytes().all(|b| b.is_ascii_digit());
    numeric(digits) && suffix.is_none_or(numeric)
}

fn ipv4_address(value: &str) -> Option<u32> {
    let parts: Vec<_> = value.split('.').collect();
    if parts.len() != 4 {
        return None;
    }
    let mut result = 0u32;
    for part in parts {
        if part.is_empty() || part.len() > 3 || !part.bytes().all(|b| b.is_ascii_digit()) {
            return None;
        }
        result = (result << 8) | u32::from(part.parse::<u8>().ok()?);
    }
    Some(result)
}

pub(crate) fn ipv4_network(value: &str) -> Option<(u32, u32)> {
    let (address, prefix) = if let Some((address, prefix)) = value.split_once('/') {
        (address, prefix.parse::<u32>().ok()?)
    } else {
        (value, 32)
    };
    if prefix > 32 {
        return None;
    }
    let mask = if prefix == 0 {
        0
    } else {
        u32::MAX << (32 - prefix)
    };
    Some((ipv4_address(address)?, mask))
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
            "Remove the personal absolute path or replace it with a placeholder / 個人の絶対パスを削除するか、プレースホルダーへ置き換えてください",
            None,
        ));
    }
    if config.structural.posix_home {
        patterns.push(pattern(
            "POSIX home path",
            r"/(?:Users|home)/[a-zA-Z0-9_.-]+/",
            "Replace the home path with ~/ or a placeholder / ホームパスを ~/ またはプレースホルダーへ置き換えてください",
            None,
        ));
    }
    if config.structural.private_ip && !config.structural.ipv4 {
        // Real octets only (0-255, zero padding allowed) so version strings like
        // 10.999.0.1 stay quiet while inventory-style 192.168.001.100 still hits.
        const OCTET: &str = r"(?:25[0-5]|2[0-4]\d|[01]?\d?\d)";
        let private_ip = format!(
            r"\b(?:10\.{OCTET}\.{OCTET}\.{OCTET}|172\.(?:1[6-9]|2\d|3[01])\.{OCTET}\.{OCTET}|192\.168\.{OCTET}\.{OCTET})\b"
        );
        patterns.push(pattern(
            "Private IPv4 (RFC1918)",
            &private_ip,
            "Generalize or remove the internal IP address; for documentation examples, use RFC5737 addresses (192.0.2.0/24, 198.51.100.0/24, 203.0.113.0/24) / 内部 IP アドレスを一般化するか削除してください。文書の例には RFC5737 の文書用アドレスを使ってください",
            Some(AllowRule::Ipv4),
        ));
    }
    if config.structural.ipv4 {
        const OCTET: &str = r"(?:25[0-5]|2[0-4]\d|[01]?\d?\d)";
        patterns.push(pattern(
            "IPv4 (not on allowlist)",
            &format!(r"{OCTET}\.{OCTET}\.{OCTET}\.{OCTET}"),
            "Remove the IP address or explicitly allow a public address/network",
            Some(AllowRule::Ipv4),
        ));
    }
    if config.structural.domain {
        patterns.push(pattern(
            "Domain (not on allowlist)",
            r"(?i)[a-z0-9][a-z0-9-]{0,62}(?:\.[a-z0-9][a-z0-9-]{0,62})*\.(?:jp|com|net|org|io|dev|ai|co|me|app|cloud|tech|test)",
            "Remove the domain or explicitly allow a public domain",
            Some(AllowRule::Domain),
        ));
    }
    if config.structural.private_repo_path {
        patterns.push(pattern(
            "Private repository path (not on allowlist)",
            r"(?i)github[\\/]+private[\\/]+[A-Za-z0-9_.-]+",
            "Remove the private repository path or explicitly allow a public repository name",
            Some(AllowRule::PrivateRepo),
        ));
    }
    if config.structural.email {
        patterns.push(pattern(
            "Email address (not on public allowlist)",
            r"\b[A-Za-z0-9._%+-]+@[A-Za-z0-9][A-Za-z0-9.-]*\.[A-Za-z]{2,}\b",
            "Remove the non-public email or add an explicitly public address/domain to allow / 非公開メールを削除するか、公開済みと確認したアドレスまたはドメインを許可設定へ追加してください",
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
