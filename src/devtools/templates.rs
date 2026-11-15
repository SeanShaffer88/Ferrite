//! Starter documents: README, Keep a Changelog, and MADR architecture
//! decision records.

use std::path::{Path, PathBuf};

/// README skeleton for a project called `name`.
pub fn readme(name: &str) -> String {
    format!(
        "# {name}\n\
         \n\
         One sentence: what {name} is and who it is for.\n\
         \n\
         ## Install\n\
         \n\
         ```bash\n\
         # how to install\n\
         ```\n\
         \n\
         ## Usage\n\
         \n\
         ```bash\n\
         # the smallest useful example\n\
         ```\n\
         \n\
         ## Configuration\n\
         \n\
         | Option | Default | Description |\n\
         |---|---|---|\n\
         | | | |\n\
         \n\
         ## Development\n\
         \n\
         How to build, test and run locally.\n\
         \n\
         ## Contributing\n\
         \n\
         How to report bugs and send changes.\n\
         \n\
         ## License\n\
         \n\
         State the license and link to `LICENSE`.\n"
    )
}

/// Changelog in the [Keep a Changelog](https://keepachangelog.com/en/1.1.0/) format.
pub fn changelog() -> String {
    "# Changelog\n\
     \n\
     All notable changes to this project will be documented in this file.\n\
     \n\
     The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),\n\
     and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).\n\
     \n\
     ## [Unreleased]\n\
     \n\
     ### Added\n\
     \n\
     ### Changed\n\
     \n\
     ### Deprecated\n\
     \n\
     ### Removed\n\
     \n\
     ### Fixed\n\
     \n\
     ### Security\n"
        .to_string()
}

/// Architecture decision record in the minimal [MADR](https://adr.github.io/madr/) layout.
pub fn adr(number: u32, title: &str, date: &str) -> String {
    format!(
        "---\n\
         status: proposed\n\
         date: {date}\n\
         ---\n\
         \n\
         # {number:04}. {title}\n\
         \n\
         ## Context and Problem Statement\n\
         \n\
         What problem are we solving, and why now?\n\
         \n\
         ## Considered Options\n\
         \n\
         * Option 1\n\
         * Option 2\n\
         \n\
         ## Decision Outcome\n\
         \n\
         Chosen option: \"Option 1\", because …\n\
         \n\
         ### Consequences\n\
         \n\
         * Good, because …\n\
         * Bad, because …\n"
    )
}

/// Today's date as `YYYY-MM-DD` (UTC), without extra dependencies.
pub fn today() -> String {
    let secs = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let days = (secs / 86_400) as i64;
    // Civil-from-days (Howard Hinnant), proleptic Gregorian calendar.
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1_460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = yoe + era * 400 + i64::from(m <= 2);
    format!("{y:04}-{m:02}-{d:02}")
}

/// Directory ADRs go in: an existing `docs/adr`, `docs/decisions` or `adr`,
/// else `docs/adr`.
pub fn adr_dir(root: &Path) -> PathBuf {
    for rel in ["docs/adr", "docs/decisions", "doc/adr", "adr"] {
        let dir = root.join(rel);
        if dir.is_dir() {
            return dir;
        }
    }
    root.join("docs/adr")
}

/// Next ADR number in `dir`: one more than the highest `NNNN-*.md` present.
pub fn next_adr_number(dir: &Path) -> u32 {
    let highest = std::fs::read_dir(dir)
        .into_iter()
        .flatten()
        .flatten()
        .filter_map(|e| {
            let name = e.file_name().to_string_lossy().into_owned();
            let digits: String = name.chars().take_while(char::is_ascii_digit).collect();
            (!digits.is_empty() && name.ends_with(".md"))
                .then(|| digits.parse::<u32>().ok())
                .flatten()
        })
        .max()
        .unwrap_or(0);
    highest + 1
}

/// Path for a new ADR that does not exist yet.
pub fn next_adr_path(dir: &Path) -> (u32, PathBuf) {
    let mut n = next_adr_number(dir);
    loop {
        let path = dir.join(format!("{n:04}-new-decision.md"));
        if !path.exists() {
            return (n, path);
        }
        n += 1;
    }
}
