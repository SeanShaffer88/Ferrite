//! Warn when a file contains something that looks like a credential, so it
//! can be removed before the file is committed or shared. Pattern-based and
//! offline; values are never logged or sent anywhere.

use super::schema::{Problem, ProblemSource, Severity};
use regex::Regex;
use std::sync::OnceLock;

struct Rule {
    name: &'static str,
    re: Regex,
}

fn rules() -> &'static [Rule] {
    static RULES: OnceLock<Vec<Rule>> = OnceLock::new();
    RULES.get_or_init(|| {
        let defs: &[(&str, &str)] = &[
            ("private key", r"-----BEGIN (?:RSA |EC |DSA |OPENSSH |PGP |ENCRYPTED )?PRIVATE KEY(?: BLOCK)?-----"),
            ("AWS access key ID", r"\b(?:AKIA|ASIA)[0-9A-Z]{16}\b"),
            ("GitHub token", r"\b(?:gh[pousr]_[A-Za-z0-9]{36,255}|github_pat_[A-Za-z0-9_]{60,255})\b"),
            ("GitLab token", r"\bglpat-[A-Za-z0-9_\-]{20,}\b"),
            ("Slack token", r"\bxox[abposr]-[A-Za-z0-9-]{10,}\b"),
            ("Slack webhook URL", r"https://hooks\.slack\.com/services/T[A-Za-z0-9_]+/B[A-Za-z0-9_]+/[A-Za-z0-9_]+"),
            ("Stripe secret key", r"\b(?:sk|rk)_live_[A-Za-z0-9]{20,}\b"),
            ("Google API key", r"\bAIza[0-9A-Za-z_\-]{35}\b"),
            ("Anthropic API key", r"\bsk-ant-[A-Za-z0-9_\-]{20,}\b"),
            ("OpenAI API key", r"\bsk-(?:proj-)?[A-Za-z0-9_\-]{32,}\b"),
            ("npm token", r"\bnpm_[A-Za-z0-9]{36}\b"),
            ("PyPI token", r"\bpypi-AgEIcHlwaS5vcmc[A-Za-z0-9_\-]{50,}\b"),
            ("Discord bot token", r"\b[MN][A-Za-z0-9_\-]{23,25}\.[A-Za-z0-9_\-]{6}\.[A-Za-z0-9_\-]{27,}\b"),
            ("JSON Web Token", r"\beyJ[A-Za-z0-9_\-]{10,}\.eyJ[A-Za-z0-9_\-]{10,}\.[A-Za-z0-9_\-]{10,}\b"),
            ("password in a URL", r"\b[a-z][a-z0-9+.\-]*://[^\s:/@]+:[^\s@/]{3,}@[^\s/]+"),
        ];
        defs.iter()
            .filter_map(|(name, pat)| Regex::new(pat).ok().map(|re| Rule { name, re }))
            .collect()
    })
}

/// Skip scanning documents above this size (bytes).
pub const MAX_SCAN_BYTES: usize = 2 * 1024 * 1024;

/// One warning per matched credential.
pub fn scan(content: &str) -> Vec<Problem> {
    if content.len() > MAX_SCAN_BYTES {
        return Vec::new();
    }
    let mut out = Vec::new();
    for (idx, line) in content.lines().enumerate() {
        // Placeholders in docs/examples are not secrets.
        let lower = line.to_ascii_lowercase();
        if lower.contains("example") || lower.contains("xxxxxxxx") || lower.contains("<your") {
            continue;
        }
        for rule in rules() {
            if let Some(m) = rule.re.find(line) {
                let col = line[..m.start()].chars().count() + 1;
                out.push(Problem {
                    source: ProblemSource::Lint,
                    severity: Severity::Warning,
                    line: idx + 1,
                    col,
                    message: format!(
                        "Looks like a {} — remove it before committing or sharing this file",
                        rule.name
                    ),
                    pointer: String::new(),
                });
                break; // one warning per line is enough
            }
        }
    }
    out
}
