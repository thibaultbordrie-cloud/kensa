//! Anonymity guard: nothing in this public repository may point to a real company, person,
//! internal tool or cloud project. The forbidden terms are stored only as hashes, so this file
//! does not reveal them either. Also: e-mail addresses must be example ones.

use std::path::{Path, PathBuf};

use sha2::{Digest, Sha256};
use unicode_normalization::UnicodeNormalization;

const FORBIDDEN: &[&str] = &[
    "08f6f59e0540944622c9",
    "0a13fe27742142ab0a55",
    "2384ec0e2963ba2a2c71",
    "3462ee2ec1e78c77d7f0",
    "3af7d17d9a4bc5308c57",
    "43b54885c5690ed70656",
    "44b1d90c896f767137f4",
    "54557012f65abc5908f3",
    "5845901d0e075c9bb3ab",
    "69185407befe30912e51",
    "7cbdf191488e3c89e8ce",
    "844ebbe693aaaf6a39d3",
    "915f4a517d06ef5fbf31",
    "91ae1deae4b736c50156",
    "9768f993f262761fe41e",
    "9be83f38bdcd57c6fe98",
    "abb81aaf9978b9966fed",
];
const TEXT: &[&str] = &["rs", "toml", "md", "yml", "yaml", "json", "txt", "html", "xml", "sh", ""];
const SKIP: &[&str] = &["target", ".git", "kensa-fixture"];

fn files(dir: &Path, out: &mut Vec<PathBuf>) {
    for entry in std::fs::read_dir(dir).unwrap().flatten() {
        let path = entry.path();
        let name = entry.file_name().to_string_lossy().to_string();
        if path.is_dir() {
            if !SKIP.contains(&name.as_str()) {
                files(&path, out);
            }
        } else if TEXT.contains(&path.extension().and_then(|e| e.to_str()).unwrap_or("")) || name == "LICENSE" {
            out.push(path);
        }
    }
}

fn words(text: &str) -> Vec<String> {
    let plain: String =
        text.to_lowercase().nfkd().filter(|c| !unicode_normalization::char::is_combining_mark(*c)).collect();
    plain
        .split(|c: char| !(c.is_ascii_lowercase() || c.is_ascii_digit()))
        .filter(|w| !w.is_empty())
        .map(String::from)
        .collect()
}

fn hash(s: &str) -> String {
    Sha256::digest(s.as_bytes()).iter().map(|b| format!("{b:02x}")).collect::<String>()[..20].to_string()
}

fn repo_files() -> Vec<(PathBuf, String)> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let mut out = Vec::new();
    files(root, &mut out);
    out.into_iter()
        .map(|p| {
            let rel = p.strip_prefix(root).unwrap().to_path_buf();
            (rel, std::fs::read_to_string(&p).unwrap_or_default())
        })
        .collect()
}

#[test]
fn no_identifying_term_anywhere() {
    let mut hits = Vec::new();
    for (path, text) in repo_files() {
        let w = words(&format!("{text} {}", path.display()));
        let mut grams: Vec<String> = w.clone();
        grams.extend(w.windows(2).map(|p| p.join(" ")));
        grams.extend(w.windows(3).map(|p| p.join(" ")));
        if grams.iter().any(|g| FORBIDDEN.contains(&hash(g).as_str())) {
            hits.push(path.display().to_string());
        }
    }
    assert!(hits.is_empty(), "identifying term found in: {hits:?}");
}

#[test]
fn only_example_email_addresses() {
    let mut bad = Vec::new();
    for (path, text) in repo_files() {
        for (i, _) in text.match_indices('@') {
            let domain: String =
                text[i + 1..].chars().take_while(|c| c.is_ascii_alphanumeric() || *c == '.' || *c == '-').collect();
            let before = text[..i].chars().last().map(|c| c.is_ascii_alphanumeric()).unwrap_or(false);
            if before
                && domain.contains('.')
                && !domain.ends_with(".example")
                && !["example.com", "example.org", "users.noreply.github.com"]
                    .iter()
                    .any(|d| domain.trim_end_matches('.').ends_with(d))
            {
                bad.push(format!("{}: @{domain}", path.display()));
            }
        }
    }
    assert!(bad.is_empty(), "{bad:?}");
}
