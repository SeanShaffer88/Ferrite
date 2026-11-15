//! Links to the current line(s): a web permalink pinned to the HEAD commit,
//! or a `path:line` reference relative to the repository.

use std::path::{Path, PathBuf};

/// Repository web base URL from a git remote URL, e.g.
/// `git@github.com:o/r.git` → `https://github.com/o/r`.
pub fn web_base(remote: &str) -> Option<String> {
    let r = remote.trim().trim_end_matches('/');
    let r = r.strip_suffix(".git").unwrap_or(r);
    let (host, path) = if let Some(rest) = r.strip_prefix("git@") {
        rest.split_once(':')?
    } else {
        let rest = r
            .strip_prefix("ssh://")
            .or_else(|| r.strip_prefix("https://"))
            .or_else(|| r.strip_prefix("http://"))?;
        let rest = rest.rsplit_once('@').map(|(_, h)| h).unwrap_or(rest);
        let (host, path) = rest.split_once('/')?;
        // Drop an ssh port (`host:2222`).
        (host.split(':').next().unwrap_or(host), path)
    };
    Some(format!("https://{host}/{}", path.trim_start_matches('/')))
}

/// Line anchor and blob path segment for the host's URL style.
fn host_style(base: &str, rel: &str, sha: &str, l1: usize, l2: usize) -> String {
    let multi = l2 > l1;
    if base.contains("gitlab") {
        let a = if multi {
            format!("#L{l1}-{l2}")
        } else {
            format!("#L{l1}")
        };
        format!("{base}/-/blob/{sha}/{rel}{a}")
    } else if base.contains("bitbucket.org") {
        let a = if multi {
            format!("#lines-{l1}:{l2}")
        } else {
            format!("#lines-{l1}")
        };
        format!("{base}/src/{sha}/{rel}{a}")
    } else if base.contains("gitea") || base.contains("codeberg.org") {
        let a = if multi {
            format!("#L{l1}-L{l2}")
        } else {
            format!("#L{l1}")
        };
        format!("{base}/src/commit/{sha}/{rel}{a}")
    } else {
        // GitHub and compatible hosts.
        let a = if multi {
            format!("#L{l1}-L{l2}")
        } else {
            format!("#L{l1}")
        };
        format!("{base}/blob/{sha}/{rel}{a}")
    }
}

/// Repository info for a file: (workdir, path relative to it with `/`).
pub fn repo_relative(path: &Path) -> Option<(PathBuf, String)> {
    let repo = git2::Repository::discover(path).ok()?;
    let root = repo.workdir()?.to_path_buf();
    let canon = path.canonicalize().unwrap_or_else(|_| path.to_path_buf());
    let root_c = root.canonicalize().unwrap_or_else(|_| root.clone());
    let rel = canon
        .strip_prefix(&root_c)
        .or_else(|_| path.strip_prefix(&root))
        .ok()?;
    Some((root, rel.to_string_lossy().replace('\\', "/")))
}

/// Web permalink for lines `l1..=l2` (1-based) of `path`.
pub fn permalink(path: &Path, l1: usize, l2: usize) -> Result<String, String> {
    let repo = git2::Repository::discover(path).map_err(|_| "not in a git repository")?;
    let (_, rel) = repo_relative(path).ok_or("file is outside the repository")?;
    let head = repo
        .head()
        .ok()
        .and_then(|h| h.peel_to_commit().ok())
        .ok_or("the repository has no commits")?;
    let sha = head.id().to_string();
    let remote = repo
        .find_remote("origin")
        .ok()
        .or_else(|| {
            let names = repo.remotes().ok()?;
            let first = names.iter().flatten().next()?.to_string();
            repo.find_remote(&first).ok()
        })
        .ok_or("no git remote configured")?;
    let url = remote.url().ok_or("remote has no URL")?;
    let base = web_base(url).ok_or("remote URL is not a web host")?;
    let rel_enc: String = rel
        .split('/')
        .map(|s| s.replace(' ', "%20").replace('#', "%23"))
        .collect::<Vec<_>>()
        .join("/");
    Ok(host_style(&base, &rel_enc, &sha, l1, l2.max(l1)))
}
