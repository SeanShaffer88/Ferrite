//! Text transforms for the "Transform Selection" picker.

/// A transform the user can pick.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Transform {
    SortLines,
    SortLinesDesc,
    UniqueLines,
    ReverseLines,
    TrimTrailing,
    Upper,
    Lower,
    Title,
    Snake,
    Camel,
    Kebab,
    Base64Encode,
    Base64Decode,
    UrlEncode,
    UrlDecode,
    JsonEscape,
    JsonUnescape,
    InsertUuid,
    InsertTimestamp,
    InsertUnixTime,
}

impl Transform {
    pub const ALL: [Transform; 20] = [
        Transform::SortLines,
        Transform::SortLinesDesc,
        Transform::UniqueLines,
        Transform::ReverseLines,
        Transform::TrimTrailing,
        Transform::Upper,
        Transform::Lower,
        Transform::Title,
        Transform::Snake,
        Transform::Camel,
        Transform::Kebab,
        Transform::Base64Encode,
        Transform::Base64Decode,
        Transform::UrlEncode,
        Transform::UrlDecode,
        Transform::JsonEscape,
        Transform::JsonUnescape,
        Transform::InsertUuid,
        Transform::InsertTimestamp,
        Transform::InsertUnixTime,
    ];

    /// Locale key for the label.
    pub fn key(self) -> &'static str {
        match self {
            Transform::SortLines => "devtools.transform.sort_lines",
            Transform::SortLinesDesc => "devtools.transform.sort_lines_desc",
            Transform::UniqueLines => "devtools.transform.unique_lines",
            Transform::ReverseLines => "devtools.transform.reverse_lines",
            Transform::TrimTrailing => "devtools.transform.trim_trailing",
            Transform::Upper => "devtools.transform.upper",
            Transform::Lower => "devtools.transform.lower",
            Transform::Title => "devtools.transform.title_case",
            Transform::Snake => "devtools.transform.snake",
            Transform::Camel => "devtools.transform.camel",
            Transform::Kebab => "devtools.transform.kebab",
            Transform::Base64Encode => "devtools.transform.base64_encode",
            Transform::Base64Decode => "devtools.transform.base64_decode",
            Transform::UrlEncode => "devtools.transform.url_encode",
            Transform::UrlDecode => "devtools.transform.url_decode",
            Transform::JsonEscape => "devtools.transform.json_escape",
            Transform::JsonUnescape => "devtools.transform.json_unescape",
            Transform::InsertUuid => "devtools.transform.insert_uuid",
            Transform::InsertTimestamp => "devtools.transform.insert_timestamp",
            Transform::InsertUnixTime => "devtools.transform.insert_unix_time",
        }
    }

    /// Inserts at the caret instead of rewriting a selection.
    pub fn is_insert(self) -> bool {
        matches!(
            self,
            Transform::InsertUuid | Transform::InsertTimestamp | Transform::InsertUnixTime
        )
    }

    /// Works on whole lines; with no selection it applies to the document.
    pub fn is_line_op(self) -> bool {
        matches!(
            self,
            Transform::SortLines
                | Transform::SortLinesDesc
                | Transform::UniqueLines
                | Transform::ReverseLines
                | Transform::TrimTrailing
        )
    }

    /// Apply to `text`. Errors are user-facing messages (e.g. bad Base64).
    pub fn apply(self, text: &str) -> Result<String, String> {
        Ok(match self {
            Transform::SortLines => map_lines(text, |l| l.sort()),
            Transform::SortLinesDesc => map_lines(text, |l| {
                l.sort();
                l.reverse()
            }),
            Transform::UniqueLines => map_lines(text, |l| {
                let mut seen = std::collections::HashSet::new();
                l.retain(|x| seen.insert(x.clone()));
            }),
            Transform::ReverseLines => map_lines(text, |l| l.reverse()),
            Transform::TrimTrailing => map_lines(text, |l| {
                for x in l.iter_mut() {
                    *x = x.trim_end().to_string();
                }
            }),
            Transform::Upper => text.to_uppercase(),
            Transform::Lower => text.to_lowercase(),
            Transform::Title => words(text)
                .iter()
                .map(|w| capitalize(w))
                .collect::<Vec<_>>()
                .join(" "),
            Transform::Snake => words(text)
                .iter()
                .map(|w| w.to_lowercase())
                .collect::<Vec<_>>()
                .join("_"),
            Transform::Kebab => words(text)
                .iter()
                .map(|w| w.to_lowercase())
                .collect::<Vec<_>>()
                .join("-"),
            Transform::Camel => {
                let w = words(text);
                let mut out = String::new();
                for (i, x) in w.iter().enumerate() {
                    if i == 0 {
                        out.push_str(&x.to_lowercase());
                    } else {
                        out.push_str(&capitalize(x));
                    }
                }
                out
            }
            Transform::Base64Encode => base64_encode(text.as_bytes()),
            Transform::Base64Decode => {
                let bytes = base64_decode(text.trim()).ok_or("not valid Base64")?;
                String::from_utf8(bytes).map_err(|_| "decoded bytes are not UTF-8 text")?
            }
            Transform::UrlEncode => url_encode(text),
            Transform::UrlDecode => url_decode(text).ok_or("not valid percent-encoding")?,
            Transform::JsonEscape => {
                let quoted = serde_json::to_string(text).map_err(|e| e.to_string())?;
                quoted[1..quoted.len() - 1].to_string()
            }
            Transform::JsonUnescape => {
                let t = text.trim();
                let quoted = if t.starts_with('"') && t.ends_with('"') && t.len() >= 2 {
                    t.to_string()
                } else {
                    format!("\"{t}\"")
                };
                serde_json::from_str::<String>(&quoted).map_err(|e| e.to_string())?
            }
            Transform::InsertUuid => uuid_v4(),
            Transform::InsertTimestamp => chrono::Local::now().to_rfc3339(),
            Transform::InsertUnixTime => chrono::Utc::now().timestamp().to_string(),
        })
    }
}

/// Split into lines (keeping the line ending style and trailing newline).
fn map_lines(text: &str, f: impl FnOnce(&mut Vec<String>)) -> String {
    let crlf = text.contains("\r\n");
    let nl = if crlf { "\r\n" } else { "\n" };
    let trailing = text.ends_with('\n');
    let body = text
        .strip_suffix(nl)
        .or(text.strip_suffix('\n'))
        .unwrap_or(text);
    let mut lines: Vec<String> = body
        .split('\n')
        .map(|l| l.trim_end_matches('\r').to_string())
        .collect();
    f(&mut lines);
    let mut out = lines.join(nl);
    if trailing {
        out.push_str(nl);
    }
    out
}

/// Words for case conversion: splits on non-alphanumerics and camelCase humps.
fn words(text: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut cur = String::new();
    let mut prev_lower = false;
    for c in text.chars() {
        if c.is_alphanumeric() {
            if c.is_uppercase() && prev_lower && !cur.is_empty() {
                out.push(std::mem::take(&mut cur));
            }
            prev_lower = c.is_lowercase() || c.is_ascii_digit();
            cur.push(c);
        } else {
            if !cur.is_empty() {
                out.push(std::mem::take(&mut cur));
            }
            prev_lower = false;
        }
    }
    if !cur.is_empty() {
        out.push(cur);
    }
    out
}

fn capitalize(w: &str) -> String {
    let mut c = w.chars();
    match c.next() {
        Some(f) => f.to_uppercase().collect::<String>() + &c.as_str().to_lowercase(),
        None => String::new(),
    }
}

const B64: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";

fn base64_encode(data: &[u8]) -> String {
    let mut out = String::with_capacity(data.len().div_ceil(3) * 4);
    for chunk in data.chunks(3) {
        let b = [
            chunk[0],
            *chunk.get(1).unwrap_or(&0),
            *chunk.get(2).unwrap_or(&0),
        ];
        let n = (u32::from(b[0]) << 16) | (u32::from(b[1]) << 8) | u32::from(b[2]);
        for i in 0..4 {
            if i <= chunk.len() {
                out.push(B64[((n >> (18 - 6 * i)) & 63) as usize] as char);
            } else {
                out.push('=');
            }
        }
    }
    out
}

/// Standard or URL-safe alphabet, padding optional, whitespace ignored.
fn base64_decode(s: &str) -> Option<Vec<u8>> {
    let mut buf = 0u32;
    let mut bits = 0u32;
    let mut out = Vec::new();
    for c in s.chars().filter(|c| !c.is_whitespace() && *c != '=') {
        let v = match c {
            'A'..='Z' => c as u32 - 'A' as u32,
            'a'..='z' => c as u32 - 'a' as u32 + 26,
            '0'..='9' => c as u32 - '0' as u32 + 52,
            '+' | '-' => 62,
            '/' | '_' => 63,
            _ => return None,
        };
        buf = (buf << 6) | v;
        bits += 6;
        if bits >= 8 {
            bits -= 8;
            out.push(((buf >> bits) & 0xff) as u8);
        }
    }
    Some(out)
}

fn url_encode(s: &str) -> String {
    let mut out = String::new();
    for b in s.bytes() {
        if b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_' | b'.' | b'~') {
            out.push(b as char);
        } else {
            out.push_str(&format!("%{b:02X}"));
        }
    }
    out
}

fn url_decode(s: &str) -> Option<String> {
    let b = s.as_bytes();
    let mut out = Vec::with_capacity(b.len());
    let mut i = 0;
    while i < b.len() {
        match b[i] {
            b'%' if i + 2 < b.len() => {
                let h = std::str::from_utf8(&b[i + 1..i + 3]).ok()?;
                out.push(u8::from_str_radix(h, 16).ok()?);
                i += 3;
            }
            b'%' => return None,
            b'+' => {
                out.push(b' ');
                i += 1;
            }
            x => {
                out.push(x);
                i += 1;
            }
        }
    }
    String::from_utf8(out).ok()
}

/// Random (version 4) UUID from the standard library's per-process
/// random keys. Unique, not intended for cryptographic use.
fn uuid_v4() -> String {
    use std::hash::{BuildHasher, Hasher};
    let mut bytes = [0u8; 16];
    for (i, half) in bytes.chunks_mut(8).enumerate() {
        let mut h = std::collections::hash_map::RandomState::new().build_hasher();
        h.write_u128(
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_nanos())
                .unwrap_or(0),
        );
        h.write_usize(i);
        half.copy_from_slice(&h.finish().to_le_bytes());
    }
    bytes[6] = (bytes[6] & 0x0f) | 0x40;
    bytes[8] = (bytes[8] & 0x3f) | 0x80;
    let hex: String = bytes.iter().map(|b| format!("{b:02x}")).collect();
    format!(
        "{}-{}-{}-{}-{}",
        &hex[0..8],
        &hex[8..12],
        &hex[12..16],
        &hex[16..20],
        &hex[20..32]
    )
}
